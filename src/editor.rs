use std::{collections::HashMap, io::Write, mem};

use crossterm::{
    ExecutableCommand, QueueableCommand, cursor,
    event::{self, Event, EventStream, KeyCode, KeyModifiers},
    style::{self},
    terminal,
};
use futures::{StreamExt, future::FutureExt, select};
use serde::{Deserialize, Serialize};

use crate::{
    buffer::Buffer,
    config::{Config, KeyAction},
    highlighter::Highlighter,
    theme::{Style, Theme},
    unicode,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Action {
    // Buffer actions
    Quit,
    Save,
    Undo,
    UndoMultiple(Vec<Action>),
    CenterView,

    // Cursor movement
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    MoveToTop,
    MoveToBottom,
    MoveToLineEnd,
    MoveToLineStart,
    MoveLineToViewCenter,
    MoveLineToViewBottom,
    MoveViewDownOneLine,
    MoveViewUpOneLine,
    MoveToBottomOfBuffer,
    MoveToTopOfBuffer,
    PageUp,
    PageDown,

    // Commands
    Command(String),

    // Text editing
    InsertCharAtCursor(char),
    RemoveCharAt(usize, usize),
    InsertLineAbove,
    InsertLineBelow,
    InsertLineAt(usize, Option<String>),
    InsertNewLine,
    DeletePreviousChar,
    DeleteCharAtCursor,
    DeleteCurrentLine,
    DeleteLineAt(usize),

    GoToLine(usize),

    // Misc
    EnterMode(Mode),
    SetWaitingKey(Box<KeyAction>),
}

#[allow(unused)]
pub enum GoToLinePosition {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Mode {
    Normal,
    Insert,
    Command,
    Visual,
    Replace,
}

#[derive(Debug)]
pub struct StyleInfo {
    pub start: usize,
    pub end: usize,
    pub style: Style,
}

impl StyleInfo {
    pub fn contains(&self, pos: usize) -> bool {
        pos >= self.start && pos < self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Cell {
    c: char,
    style: Style,
}

#[derive(Debug, Clone)]
pub struct RenderBuffer {
    cells: Vec<Cell>,
    width: usize,
    #[allow(dead_code)]
    height: usize,
}

impl RenderBuffer {
    #[allow(dead_code)]
    fn new_with_contents(width: usize, height: usize, style: Style, contents: Vec<String>) -> Self {
        let mut cells = vec![];

        for line in contents {
            for c in line.chars() {
                cells.push(Cell {
                    c,
                    style: style.clone(),
                });
            }
            for _ in 0..width.saturating_sub(line.len()) {
                cells.push(Cell {
                    c: ' ',
                    style: style.clone(),
                });
            }
        }

        Self {
            cells,
            width,
            height,
        }
    }

    fn new(width: usize, height: usize, default_style: Style) -> Self {
        let cells = vec![
            Cell {
                c: ' ',
                style: default_style.clone(),
            };
            width * height
        ];

        Self {
            cells,
            width,
            height,
        }
    }

    fn set_char(&mut self, x: usize, y: usize, c: char, style: &Style) {
        let pos = (y * self.width) + x;
        self.cells[pos] = Cell {
            c,
            style: style.clone(),
        };
    }

    fn set_text(&mut self, x: usize, y: usize, text: &str, style: &Style) {
        let pos = (y * self.width) + x;
        for (i, c) in text.chars().enumerate() {
            self.cells[pos + i] = Cell {
                c,
                style: style.clone(),
            }
        }
    }

    fn diff(&self, other: &RenderBuffer) -> Vec<Change<'_>> {
        let mut changes = vec![];
        for (pos, cell) in self.cells.iter().enumerate() {
            if *cell != other.cells[pos] {
                let y = pos / self.width;
                let x = pos % self.width;

                changes.push(Change { x, y, cell });
            }
        }

        changes
    }
}

#[derive(Debug)]
pub struct Change<'a> {
    x: usize,
    y: usize,
    cell: &'a Cell,
}

pub struct Editor {
    config: Config,
    pub theme: Theme,
    highlighter: Highlighter,
    buffer: Buffer,
    size: (u16, u16),
    stdout: std::io::Stdout,
    vtop: usize,
    vleft: usize,
    pos_x: usize,
    pos_y: usize,
    vx: usize,
    mode: Mode,
    waiting_command: Option<String>,
    command: String,
    waiting_key_action: Option<KeyAction>,
    undoable_actions: Vec<Action>,
    insert_undo_actions: Vec<Action>,
    last_error: Option<String>,
}

impl Editor {
    pub fn with_size(
        width: usize,
        height: usize,
        config: Config,
        theme: Theme,
        buffer: Buffer,
    ) -> anyhow::Result<Self> {
        let stdout = std::io::stdout();
        let vx = buffer.len().to_string().len() + 2;
        let size = (width as u16, height as u16);
        let highlighter = Highlighter::new(&theme)?;

        Ok(Self {
            config,
            theme,
            buffer,
            highlighter,
            stdout,
            vtop: 0,
            vleft: 0,
            pos_x: 0,
            pos_y: 0,
            vx,
            mode: Mode::Normal,
            size,
            command: String::new(),
            waiting_command: None,
            waiting_key_action: None,
            undoable_actions: vec![],
            insert_undo_actions: vec![],
            last_error: None,
        })
    }

    pub fn new(config: Config, theme: Theme, buffer: Buffer) -> anyhow::Result<Self> {
        let size = terminal::size()?;
        Self::with_size(size.0 as usize, size.1 as usize, config, theme, buffer)
    }

    fn vwidth(&self) -> usize {
        self.size.0 as usize
    }

    fn vheight(&self) -> usize {
        self.size.1 as usize - 2
    }

    fn line_length(&self) -> usize {
        if let Some(line) = self.view_line(self.pos_y) {
            return line.len();
        }
        0
    }

    fn buffer_line(&self) -> usize {
        self.vtop + self.pos_y
    }

    fn view_line(&self, n: usize) -> Option<String> {
        let line = self.vtop + n;
        self.buffer.get(line)
    }

    fn set_cursor_style(&mut self) -> anyhow::Result<()> {
        self.stdout.queue(match self.waiting_key_action {
            Some(_) => cursor::SetCursorStyle::SteadyUnderScore,
            _ => match self.mode {
                Mode::Normal => cursor::SetCursorStyle::DefaultUserShape,
                Mode::Insert => cursor::SetCursorStyle::SteadyBar,
                _ => cursor::SetCursorStyle::DefaultUserShape,
            },
        })?;

        Ok(())
    }

    fn gutter_width(&self) -> usize {
        self.buffer.len().to_string().len() + 1
    }

    fn draw_gutter(&mut self, buffer: &mut RenderBuffer) {
        let width = self.gutter_width();
        let foreground = self.theme.gutter_style.foreground.unwrap_or(
            self.theme
                .style
                .foreground
                .expect("foreground is defined for theme"),
        );
        let background = self.theme.gutter_style.background.unwrap_or(
            self.theme
                .style
                .background
                .expect("background is defined for theme"),
        );

        for n in 0..self.vheight() {
            let line_number = n + 1 + self.vtop;
            let text = if line_number <= self.buffer.len() {
                line_number.to_string()
            } else {
                " ".repeat(width)
            };

            buffer.set_text(
                0,
                n,
                &format!("{text:>width$} ", width = width),
                &Style {
                    foreground: Some(foreground),
                    background: Some(background),
                    ..Default::default()
                },
            );
        }
    }

    pub fn draw_cursor(&mut self, buffer: &mut RenderBuffer) -> anyhow::Result<()> {
        self.set_cursor_style()?;
        self.stdout.queue(cursor::MoveTo(
            (self.vx + self.pos_x) as u16,
            self.pos_y as u16,
        ))?;
        self.draw_status_line(buffer);

        Ok(())
    }

    pub fn highlight(&mut self, code: &str) -> anyhow::Result<Vec<StyleInfo>> {
        self.highlighter.highlight(code)
    }

    fn fill_line(&mut self, buffer: &mut RenderBuffer, x: usize, y: usize, style: &Style) {
        let width = self.vwidth().saturating_sub(x);
        let line_fill = " ".repeat(width);
        buffer.set_text(x, y, &line_fill, style);
    }

    fn draw_line(&mut self, buffer: &mut RenderBuffer) {
        let line = self.view_line(self.pos_y).unwrap_or_default();
        let style_info = self.highlight(&line).unwrap_or_default();
        let default_style = self.theme.style.clone();

        let mut x = self.vx;
        let mut iter = line.chars().enumerate().peekable();

        while let Some((pos, c)) = iter.next() {
            if c == '\n' || iter.peek().is_none() {
                if c != '\n' {
                    buffer.set_char(x, self.pos_y, c, &default_style);
                    x += 1;
                }
                self.fill_line(buffer, x, self.pos_y, &default_style);
                break;
            }

            if x < self.vwidth() {
                if let Some(style) = determine_style_for_position(&style_info, pos) {
                    buffer.set_char(x, self.pos_y, c, &style);
                } else {
                    buffer.set_char(x, self.pos_y, c, &default_style);
                }
            }
            x += 1;
        }
    }

    pub fn draw_view(&mut self, buffer: &mut RenderBuffer) -> anyhow::Result<()> {
        let vbuffer = self.buffer.view(self.vtop, self.vheight());
        let style_info = self.highlight(&vbuffer)?;
        let vheight = self.vheight();
        let default_style = self.theme.style.clone();

        let mut x = self.vx;
        let mut y = 0;
        let mut iter = vbuffer.chars().enumerate().peekable();

        while let Some((pos, c)) = iter.next() {
            if c == '\n' || iter.peek().is_none() {
                if c != '\n' {
                    buffer.set_char(x, y, c, &default_style);
                    x += 1;
                }
                self.fill_line(buffer, x, y, &default_style);
                x = self.vx;
                y += 1;
                if y > vheight {
                    break;
                }
                continue;
            }

            if x < self.vwidth() {
                if let Some(style) = determine_style_for_position(&style_info, pos) {
                    buffer.set_char(x, y, c, &style);
                } else {
                    buffer.set_char(x, y, c, &default_style);
                }
            }

            x += 1;
        }

        while y < vheight {
            self.fill_line(buffer, self.vx, y, &default_style);
            y += 1;
        }

        self.draw_gutter(buffer);

        Ok(())
    }

    pub fn draw_status_line(&mut self, buffer: &mut RenderBuffer) {
        let mode_str = format!(" {:?} ", self.mode).to_uppercase();
        let file_str = format!(" {}", self.buffer.file.as_deref().unwrap_or("[No Name]"));
        let position_str = format!(
            " {}:{} ",
            self.pos_y + self.vtop + 1,
            self.pos_x + self.vleft + 1
        );

        // Calculate file string width dynamically
        let file_str_width = self.size.0 - mode_str.len() as u16 - position_str.len() as u16 - 2;
        let y = self.size.1 as usize - 2;

        let transition_style = Style {
            foreground: self.theme.status_line_style.outer_style.background,
            background: self.theme.status_line_style.inner_style.background,
            ..Default::default()
        };

        buffer.set_text(0, y, &mode_str, &self.theme.status_line_style.outer_style);

        buffer.set_text(
            mode_str.len(),
            y,
            &self.theme.status_line_style.outer_chars[1].to_string(),
            &transition_style,
        );

        buffer.set_text(
            mode_str.len() + 1,
            y,
            &format!("{:<width$}", file_str, width = file_str_width as usize),
            &self.theme.status_line_style.inner_style,
        );

        buffer.set_text(
            mode_str.len() + 1 + file_str_width as usize,
            y,
            &self.theme.status_line_style.outer_chars[2].to_string(),
            &transition_style,
        );

        buffer.set_text(
            mode_str.len() + 2 + file_str_width as usize,
            y,
            &position_str,
            &self.theme.status_line_style.outer_style,
        );
    }

    fn draw_command_line(&mut self, buffer: &mut RenderBuffer) {
        let style = &self.theme.style;
        let y = self.size.1 as usize - 1;

        if !self.is_command() {
            let wc = if let Some(ref waiting_command) = self.waiting_command {
                waiting_command.clone()
            } else {
                " ".repeat(10)
            };

            if let Some(ref last_error) = self.last_error {
                let error = format!("{:width$}", last_error, width = self.size.0 as usize);
                buffer.set_text(0, self.size.1 as usize - 1, &error, style);
            } else {
                let clear_line = " ".repeat(self.size.0 as usize - 10);
                buffer.set_text(0, y, &clear_line, style);
            }

            buffer.set_text(self.size.0 as usize - 10, y, &wc, style);

            return;
        }

        let cmd_line = format!(
            ":{:width$}",
            self.command,
            width = self.size.0 as usize - self.command.len() - 1
        );

        buffer.set_text(0, self.size.1 as usize - 1, &cmd_line, style);
    }

    #[allow(dead_code)]
    fn is_normal(&self) -> bool {
        matches!(self.mode, Mode::Normal)
    }

    fn is_insert(&self) -> bool {
        matches!(self.mode, Mode::Insert)
    }

    fn is_command(&self) -> bool {
        matches!(self.mode, Mode::Command)
    }

    fn check_bounds(&mut self) {
        let line_len = self.line_length();

        if self.pos_x >= line_len && !self.is_insert() {
            if line_len > 0 {
                self.pos_x = self.line_length() - 1;
            } else if !self.is_insert() {
                self.pos_x = 0;
            }
        }

        if self.pos_x >= self.vwidth() {
            self.pos_x = self.vwidth() - 1;
        }

        let line_in_buffer = self.pos_y + self.vtop;
        if line_in_buffer > self.buffer.len() - 1 {
            self.pos_y = self.buffer.len() - self.vtop - 1;
        }
    }

    fn render_diff(&mut self, change_set: Vec<Change>) -> anyhow::Result<()> {
        for change in change_set {
            let x = change.x;
            let y = change.y;
            let cell = change.cell;

            self.stdout.queue(cursor::MoveTo(x as u16, y as u16))?;
            if let Some(background) = cell.style.background {
                self.stdout
                    .queue(style::SetBackgroundColor(background.into()))?;
            }
            if let Some(foreground) = cell.style.foreground {
                self.stdout
                    .queue(style::SetForegroundColor(foreground.into()))?;
            }
            self.stdout.queue(style::Print(cell.c))?;
        }

        self.set_cursor_style()?;
        self.stdout
            .queue(cursor::MoveTo(
                (self.vx + self.pos_x) as u16,
                self.pos_y as u16,
            ))?
            .flush()?;

        Ok(())
    }

    fn render(&mut self, buffer: &mut RenderBuffer) -> anyhow::Result<()> {
        self.draw_view(buffer)?;
        self.draw_gutter(buffer);
        self.draw_status_line(buffer);

        self.stdout
            .queue(terminal::Clear(terminal::ClearType::All))?
            .queue(cursor::MoveTo(0, 0))?;

        let mut current_style = &self.theme.style;

        for cell in buffer.cells.iter() {
            if cell.style != *current_style {
                if let Some(background) = cell.style.background {
                    self.stdout
                        .queue(style::SetBackgroundColor(background.into()))?;
                }
                if let Some(foreground) = cell.style.foreground {
                    self.stdout
                        .queue(style::SetForegroundColor(foreground.into()))?;
                }
                current_style = &cell.style;
            }

            self.stdout.queue(style::Print(cell.c))?;
        }

        self.draw_cursor(buffer)?;
        self.stdout.flush()?;

        Ok(())
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        terminal::enable_raw_mode()?;
        self.stdout
            .execute(event::EnableMouseCapture)?
            .execute(terminal::EnterAlternateScreen)?
            .execute(terminal::Clear(terminal::ClearType::All))?;

        let mut buffer = RenderBuffer::new(
            self.size.0 as usize,
            self.size.1 as usize,
            self.theme.style.clone(),
        );

        self.render(&mut buffer)?;

        let mut reader = EventStream::new();

        loop {
            let mut event = reader.next().fuse();

            select! {
                maybe_event = event => {
                    match maybe_event {
                        Some(Ok(ev)) => {
                            let current_buffer = buffer.clone();
                            self.check_bounds();

                            if let event::Event::Resize(width, height) = ev {
                                self.size = (width, height);
                                buffer = RenderBuffer::new(
                                    self.size.0 as usize,
                                    self.size.1 as usize,
                                    self.theme.style.clone(),
                                );
                                self.render(&mut buffer)?;
                                continue;
                            }

                            if let Some(action) = self.handle_event(&ev) {
                                let quit = match action {
                                    KeyAction::Single(action) => self.execute(&action, &mut buffer).await?,
                                    KeyAction::Multiple(actions) => {
                                        let mut quit = false;
                                        for action in actions {
                                            if self.execute(&action, &mut buffer).await? {
                                                quit = true;
                                                break;
                                            }
                                        }
                                        quit
                                    }
                                    KeyAction::Nested(actions) => {
                                        if let Event::Key(event::KeyEvent {
                                            code: KeyCode::Char(c),
                                            ..
                                        }) = ev {
                                            self.waiting_command = Some(format!("{c}"));
                                        }
                                        self.waiting_key_action = Some(KeyAction::Nested(actions));
                                        false
                                    }
                                    _ => {false}
                                };

                                if quit {
                                    break;
                                }
                            }

                            self.redraw(&current_buffer, &mut buffer)?;
                        },
                        Some(Err(_error)) => {

                        },
                        None => {}
                    }
                }
            }
        }

        Ok(())
    }

    fn redraw(
        &mut self,
        current_buffer: &RenderBuffer,
        buffer: &mut RenderBuffer,
    ) -> anyhow::Result<()> {
        self.stdout.execute(cursor::Hide)?;
        self.draw_status_line(buffer);
        self.draw_command_line(buffer);
        self.render_diff(buffer.diff(current_buffer))?;
        self.draw_cursor(buffer)?;
        self.stdout.execute(cursor::Show)?;
        Ok(())
    }

    fn handle_event(&mut self, ev: &event::Event) -> Option<KeyAction> {
        if let Some(key_action) = self.waiting_key_action.take() {
            self.waiting_command = None;
            return self.handle_waiting_command(key_action, ev);
        }
        match self.mode {
            Mode::Normal => self.handle_normal_event(ev),
            Mode::Insert => self.handle_insert_event(ev),
            Mode::Command => self.handle_command_event(ev),
            Mode::Visual => self.handle_visual_event(ev),
            Mode::Replace => self.handle_replace_event(ev),
        }
    }

    fn handle_command(&mut self, cmd: &str) -> Option<Action> {
        if let Ok(line) = cmd.parse::<usize>() {
            return Some(Action::GoToLine(line));
        }

        if cmd == "q" {
            return Some(Action::Quit);
        }

        if cmd == "w" {
            return Some(Action::Save);
        }

        None
    }

    fn handle_waiting_command(
        &mut self,
        key_action: KeyAction,
        ev: &event::Event,
    ) -> Option<KeyAction> {
        let KeyAction::Nested(nested_mappings) = key_action else {
            panic!("expected nested mappings");
        };

        self.event_to_key_action(&nested_mappings, ev)
    }

    fn handle_normal_event(&mut self, ev: &event::Event) -> Option<KeyAction> {
        self.event_to_key_action(&self.config.keys.normal, ev)
    }

    fn handle_insert_event(&mut self, ev: &event::Event) -> Option<KeyAction> {
        if let Some(key_action) = self.event_to_key_action(&self.config.keys.insert, ev) {
            return Some(key_action);
        }

        match ev {
            Event::Key(event) => match event.code {
                KeyCode::Char(c) => KeyAction::Single(Action::InsertCharAtCursor(c)).into(),
                _ => None,
            },
            _ => None,
        }
    }

    fn handle_command_event(&mut self, ev: &event::Event) -> Option<KeyAction> {
        if let Event::Key(event) = ev {
            let code = event.code;

            match code {
                KeyCode::Esc => {
                    self.command = String::new();
                    return Some(KeyAction::Single(Action::EnterMode(Mode::Normal)));
                }
                KeyCode::Backspace => {
                    if self.command.len() < 2 {
                        self.command = String::new();
                    } else {
                        self.command = self.command[..self.command.len() - 1].to_string();
                    }
                }
                KeyCode::Enter => {
                    if self.command.trim().is_empty() {
                        return Some(KeyAction::Single(Action::EnterMode(Mode::Normal)));
                    }
                    return Some(KeyAction::Multiple(vec![
                        Action::EnterMode(Mode::Normal),
                        Action::Command(self.command.clone()),
                    ]));
                }
                KeyCode::Char(c) => {
                    self.command = format!("{}{c}", self.command);
                }
                _ => {}
            }
        }

        None
    }

    fn handle_visual_event(&mut self, ev: &event::Event) -> Option<KeyAction> {
        if let Event::Key(event) = ev {
            let code = event.code;

            match code {
                event::KeyCode::Esc => {
                    return Some(KeyAction::Single(Action::EnterMode(Mode::Normal)));
                }
                _ => return None,
            }
        }

        None
    }

    fn handle_replace_event(&mut self, ev: &event::Event) -> Option<KeyAction> {
        if let Event::Key(event) = ev {
            let code = event.code;

            match code {
                event::KeyCode::Esc => {
                    return Some(KeyAction::Single(Action::EnterMode(Mode::Normal)));
                }
                _ => return None,
            }
        }

        None
    }

    pub fn cleanup(&mut self) -> anyhow::Result<()> {
        self.stdout
            .execute(terminal::LeaveAlternateScreen)?
            .execute(event::DisableMouseCapture)?;
        terminal::disable_raw_mode()?;

        Ok(())
    }

    fn current_line_contents(&self) -> Option<String> {
        self.buffer.get(self.buffer_line())
    }

    #[async_recursion::async_recursion]
    async fn execute(
        &mut self,
        action: &Action,
        buffer: &mut RenderBuffer,
    ) -> anyhow::Result<bool> {
        self.last_error = None;
        match action {
            Action::Quit => return Ok(true),
            Action::Save => match self.buffer.save() {
                Ok(msg) => {
                    self.last_error = Some(msg);
                }
                Err(e) => {
                    self.last_error = Some(e.to_string());
                }
            },
            Action::Undo => {
                if let Some(undoable_action) = self.undoable_actions.pop() {
                    self.execute(&undoable_action, buffer).await?;
                }
            }
            Action::UndoMultiple(actions) => {
                for action in actions.iter().rev() {
                    self.execute(action, buffer).await?;
                }
            }
            Action::CenterView => {
                let view_center = self.vheight() / 2;
                let distance_to_center = self.pos_y as isize - view_center as isize;

                if distance_to_center > 0 {
                    let distance_to_center = distance_to_center.unsigned_abs();
                    self.vtop += distance_to_center;
                    self.pos_y = view_center;
                } else if distance_to_center < 0 {
                    let distance_to_center = distance_to_center.unsigned_abs();
                    let new_vtop = self.vtop.saturating_sub(distance_to_center);
                    if self.buffer.len() > self.vtop + distance_to_center && new_vtop != self.vtop {
                        self.vtop = new_vtop;
                        self.pos_y = view_center;
                    }
                }
                self.draw_view(buffer)?;
            }
            Action::MoveUp => {
                if self.pos_y == 0 {
                    if self.vtop > 0 {
                        self.vtop -= 1;
                        self.draw_view(buffer)?;
                    }
                } else {
                    self.pos_y = self.pos_y.saturating_sub(1);
                }
            }
            Action::MoveDown => {
                self.pos_y += 1;
                if self.pos_y >= self.vheight() {
                    self.vtop += 1;
                    self.pos_y -= 1;
                    self.draw_view(buffer)?;
                }
            }
            Action::MoveLeft => {
                _ = self.pos_x.saturating_sub(1);
                if self.pos_x < self.vleft {
                    self.pos_x = self.vleft;
                }
            }
            Action::MoveRight => {
                self.pos_x += 1;
            }
            Action::MoveToTop => {
                self.pos_y = 0;
            }
            Action::MoveToBottom => {
                self.pos_y = self.vheight() - 1;
            }
            Action::MoveToTopOfBuffer => {
                self.vtop = 0;
                self.pos_y = 0;
                self.draw_view(buffer)?;
            }
            Action::MoveToBottomOfBuffer => {
                self.vtop = self.buffer.len() - self.vheight();
                self.pos_y = self.vheight() - 1;
                self.draw_view(buffer)?;
            }
            Action::MoveToLineStart => {
                self.pos_x = 0;
            }
            Action::MoveToLineEnd => {
                self.pos_x = self.line_length().saturating_sub(1);
            }
            Action::MoveLineToViewCenter => {
                let view_center = self.vheight() / 2;
                let distance_to_center = self.pos_y as isize - view_center as isize;

                if distance_to_center > 0 {
                    let distance_to_center = distance_to_center.unsigned_abs();
                    if self.vtop > distance_to_center {
                        let new_vtop = self.vtop + distance_to_center;
                        self.vtop = new_vtop;
                        self.pos_y = view_center;
                        self.draw_view(buffer)?;
                    }
                } else if distance_to_center < 0 {
                    let distance_to_center = distance_to_center.unsigned_abs();
                    let new_vtop = self.vtop.saturating_sub(distance_to_center);
                    let distance_to_go = self.vtop + distance_to_center;
                    if self.buffer.len() > distance_to_go && new_vtop != self.vtop {
                        self.vtop = new_vtop;
                        self.pos_y = view_center;
                        self.draw_view(buffer)?;
                    }
                }
            }
            Action::MoveLineToViewBottom => {
                let line = self.buffer_line();
                if line > self.vtop + self.vheight() {
                    self.vtop = line - self.vheight();
                    self.pos_y = self.vheight() - 1;
                    self.draw_view(buffer)?;
                }
            }
            Action::MoveViewDownOneLine => {
                if self.vtop < self.buffer.len() - self.vheight() {
                    self.vtop += 1;
                    if self.pos_y > 5 {
                        self.pos_y = self.pos_y.saturating_sub(1);
                    } else {
                        self.pos_y = 5;
                    }
                }
                self.draw_view(buffer)?;
            }
            Action::MoveViewUpOneLine => {
                if self.vtop > 0 {
                    self.vtop = self.vtop.saturating_sub(1);
                    if self.pos_y < self.vheight() - 7 {
                        self.pos_y += 1;
                    } else {
                        self.pos_y = self.vheight() - 7
                    }
                }
                self.draw_view(buffer)?;
            }
            Action::PageUp => {
                if self.vtop > 0 {
                    self.vtop = self.vtop.saturating_sub(self.vheight());
                    self.draw_view(buffer)?;
                }
            }
            Action::PageDown => {
                if self.buffer.len() > (self.vtop + self.vheight()) {
                    self.vtop += self.vheight();
                    self.draw_view(buffer)?;
                }
            }
            Action::EnterMode(new_mode) => {
                if !self.is_insert() && matches!(new_mode, Mode::Insert) {
                    self.insert_undo_actions = Vec::new();
                }
                if self.is_insert()
                    && matches!(new_mode, Mode::Normal)
                    && !self.insert_undo_actions.is_empty()
                {
                    let actions = mem::take(&mut self.insert_undo_actions);
                    self.undoable_actions.push(Action::UndoMultiple(actions));
                }

                if self.is_command() {
                    self.draw_command_line(buffer);
                }

                self.mode = *new_mode;
                self.draw_status_line(buffer);
            }
            Action::InsertCharAtCursor(c) => {
                self.insert_undo_actions
                    .push(Action::RemoveCharAt(self.pos_x, self.buffer_line()));
                self.buffer.insert(self.pos_x, self.buffer_line(), *c);
                self.pos_x += 1;
                self.draw_line(buffer);
            }
            Action::RemoveCharAt(x, y) => {
                self.buffer.remove(*x, *y);
                self.draw_line(buffer);
            }
            Action::DeleteCharAtCursor => {
                self.buffer.remove(self.pos_x, self.buffer_line());
                self.draw_line(buffer);
            }
            Action::InsertNewLine => {
                self.insert_undo_actions
                    .push(Action::DeleteLineAt(self.buffer_line() + 1));
                self.buffer
                    .insert_line(self.buffer_line() + 1, String::new());
                self.pos_x = 0;
                self.pos_y += 1;
            }
            Action::InsertLineAbove => {
                self.buffer.insert_line(self.buffer_line(), String::new());
                self.pos_y = self.pos_y.saturating_sub(1);
                self.mode = Mode::Insert;
            }
            Action::InsertLineBelow => {
                self.undoable_actions
                    .push(Action::DeleteLineAt(self.buffer_line() + 1));
                self.buffer
                    .insert_line(self.buffer_line() + 1, String::new());
                self.pos_y += 1;
                self.pos_x = 0;
                self.mode = Mode::Insert;
            }
            Action::InsertLineAt(line, contents) => {
                self.undoable_actions
                    .push(Action::DeleteLineAt(self.buffer_line()));
                if let Some(contents) = contents {
                    self.buffer.insert_line(*line, contents.to_string());
                }
            }
            Action::DeleteCurrentLine => {
                let line = self.buffer_line();
                let contents = self.current_line_contents();

                self.buffer.remove_line(self.buffer_line());
                self.undoable_actions
                    .push(Action::InsertLineAt(line, contents));
            }
            Action::DeletePreviousChar => {
                if self.pos_x > 0
                    && let Some(line) = self.current_line_contents()
                {
                    let line = line.trim_end_matches('\n');
                    let current_byte = unicode::char_to_byte(line, self.pos_x);

                    if let Some(prev_byte) = unicode::prev_grapheme_boundary(line, current_byte) {
                        let prev_char_idx = unicode::byte_to_char(line, prev_byte);

                        let chars_to_remove = self.pos_x - prev_char_idx;

                        self.pos_x = prev_char_idx;

                        let line_num = self.buffer_line();
                        let pos_x = self.pos_x;
                        for _ in 0..chars_to_remove {
                            self.buffer.remove(pos_x, line_num);
                        }

                        self.draw_line(buffer);
                    }
                }
            }
            Action::SetWaitingKey(key_action) => {
                self.waiting_key_action = Some(*(key_action.clone()));
            }
            Action::DeleteLineAt(y) => {
                self.buffer.remove_line(*y);
            }
            Action::Command(cmd) => {
                self.command = String::new();

                if let Some(ref action) = self.handle_command(cmd) {
                    self.last_error = None;
                    return self.execute(action, buffer).await;
                } else {
                    self.last_error = Some(format!("Not an editor command: {cmd:?}"));
                }
            }
            Action::GoToLine(line) => {
                self.go_to_line(*line, buffer, GoToLinePosition::Center)
                    .await?
            }
        }

        Ok(false)
    }

    async fn go_to_line(
        &mut self,
        line: usize,
        buffer: &mut RenderBuffer,
        pos: GoToLinePosition,
    ) -> anyhow::Result<()> {
        if line == 0 {
            self.execute(&Action::MoveToTop, buffer).await?;
            return Ok(());
        }

        if line <= self.buffer.len() {
            let y = line - 1;

            if self.is_within_view(y) {
                self.pos_y = y - self.vtop;
            } else if self.is_within_first_page(y) {
                self.vtop = 0;
                self.pos_y = y;
                self.draw_view(buffer)?;
            } else if self.is_within_last_page(y) {
                self.vtop = self.buffer.len() - self.vheight();
                self.pos_y = y - self.vtop;
                self.draw_view(buffer)?;
            } else {
                if matches!(pos, GoToLinePosition::Bottom) {
                    self.vtop = y - self.vheight();
                    self.pos_y = self.buffer_line() - self.vtop;
                } else {
                    self.vtop = y;
                    self.pos_y = 0;
                    if matches!(pos, GoToLinePosition::Center) {
                        self.execute(&Action::MoveLineToViewCenter, buffer).await?;
                    }
                }

                self.draw_view(buffer)?;
            }
        }
        Ok(())
    }

    fn is_within_view(&self, y: usize) -> bool {
        (self.vtop..self.vtop + self.vheight()).contains(&y)
    }

    fn is_within_last_page(&self, y: usize) -> bool {
        y > self.buffer.len() - self.vheight()
    }

    fn is_within_first_page(&self, y: usize) -> bool {
        y < self.vheight()
    }

    fn event_to_key_action(
        &self,
        mappings: &HashMap<String, KeyAction>,
        ev: &Event,
    ) -> Option<KeyAction> {
        match ev {
            event::Event::Key(event::KeyEvent {
                code, modifiers, ..
            }) => {
                let key = match code {
                    KeyCode::Char(c) => format!("{c}"),
                    _ => format!("{code:?}"),
                };

                let key = match *modifiers {
                    KeyModifiers::CONTROL => format!("Ctrl-{key}"),
                    KeyModifiers::ALT => format!("Alt-{key}"),
                    _ => key,
                };

                mappings.get(&key).cloned()
            }
            _ => None,
        }
    }
}

fn determine_style_for_position(style_info: &[StyleInfo], pos: usize) -> Option<Style> {
    if let Some(s) = style_info
        .iter()
        .find(|style_info| style_info.contains(pos))
    {
        return Some(s.style.clone());
    }

    None
}
