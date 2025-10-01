use std::{collections::HashMap, io::Write, mem};

use crossterm::{
    ExecutableCommand, QueueableCommand, cursor,
    event::{self, Event, EventStream, KeyCode, KeyModifiers},
    style::{self},
    terminal,
};
use futures::{StreamExt, future::FutureExt, select};
use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::{
    buffer::Buffer,
    command,
    config::{Config, KeyAction},
    highlighter::Highlighter,
    log,
    theme::{Style, Theme},
    unicode,
};

#[allow(unused)]
pub enum GoToLinePosition {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
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
    cursor_x: usize,
    cursor_y: usize,
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
            cursor_x: 0,
            cursor_y: 0,
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
        if let Some(line) = self.view_line(self.cursor_y) {
            return line.len();
        }
        0
    }

    fn buffer_line(&self) -> usize {
        self.vtop + self.cursor_y
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
            (self.vx + self.cursor_x) as u16,
            self.cursor_y as u16,
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
        let line = self.view_line(self.cursor_y).unwrap_or_default();
        let style_info = self.highlight(&line).unwrap_or_default();
        let default_style = self.theme.style.clone();

        let mut x = self.vx;
        let mut iter = line.chars().enumerate().peekable();

        while let Some((pos, c)) = iter.next() {
            if c == '\n' || iter.peek().is_none() {
                if c != '\n' {
                    buffer.set_char(x, self.cursor_y, c, &default_style);
                    x += 1;
                }
                self.fill_line(buffer, x, self.cursor_y, &default_style);
                break;
            }

            if x < self.vwidth() {
                if let Some(style) = determine_style_for_position(&style_info, pos) {
                    buffer.set_char(x, self.cursor_y, c, &style);
                } else {
                    buffer.set_char(x, self.cursor_y, c, &default_style);
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
            self.cursor_y + self.vtop + 1,
            self.cursor_x + self.vleft + 1
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

        if self.cursor_x >= line_len && !self.is_insert() {
            if line_len > 0 {
                self.cursor_x = self.line_length() - 1;
            } else if !self.is_insert() {
                self.cursor_x = 0;
            }
        }

        if self.cursor_x >= self.vwidth() {
            self.cursor_x = self.vwidth() - 1;
        }

        let line_in_buffer = self.cursor_y + self.vtop;
        if line_in_buffer > self.buffer.len() - 1 {
            self.cursor_y = self.buffer.len() - self.vtop - 1;
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
                (self.vx + self.cursor_x) as u16,
                self.cursor_y as u16,
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

    fn handle_command(&mut self, cmd: &str) -> Vec<Action> {
        log!("handle_command: {}", cmd);
        self.command = String::new();
        self.waiting_command = None;
        self.last_error = None;

        if let Ok(line) = cmd.parse::<usize>() {
            return vec![Action::GoToLine(line)];
        }

        let commands = &["quit", "write"];

        let parsed = command::parse(commands, cmd);

        let Some(parsed) = parsed else {
            self.last_error = Some(format!("unknown command {cmd:?}"));
            return vec![];
        };

        let mut actions = vec![];
        for cmd in &parsed.commands {
            if cmd == "quit" {
                actions.push(Action::Quit(parsed.is_forced()));
            }

            if cmd == "write" {
                if let Some(file) = parsed.args.first() {
                    actions.push(Action::SaveAs(file.clone()));
                } else {
                    actions.push(Action::Save);
                }
            }
        }

        actions
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
            Action::Quit(force) => {
                if *force {
                    return Ok(true);
                }

                if !self.buffer.dirty {
                    return Ok(true);
                }

                self.last_error = Some("Buffer has unwritten changes".to_string());
                return Ok(false);
            }
            Action::Save => match self.buffer.save() {
                Ok(msg) => {
                    self.last_error = Some(msg);
                }
                Err(e) => {
                    self.last_error = Some(e.to_string());
                }
            },
            Action::SaveAs(new_file_name) => match self.buffer.save_as(new_file_name) {
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
                let distance_to_center = self.cursor_y as isize - view_center as isize;

                if distance_to_center > 0 {
                    let distance_to_center = distance_to_center.unsigned_abs();
                    self.vtop += distance_to_center;
                    self.cursor_y = view_center;
                } else if distance_to_center < 0 {
                    let distance_to_center = distance_to_center.unsigned_abs();
                    let new_vtop = self.vtop.saturating_sub(distance_to_center);
                    if self.buffer.len() > self.vtop + distance_to_center && new_vtop != self.vtop {
                        self.vtop = new_vtop;
                        self.cursor_y = view_center;
                    }
                }
                self.draw_view(buffer)?;
            }
            Action::MoveUp => {
                if self.cursor_y == 0 {
                    if self.vtop > 0 {
                        self.vtop -= 1;
                        self.draw_view(buffer)?;
                    }
                } else {
                    self.cursor_y = self.cursor_y.saturating_sub(1);
                }
            }
            Action::MoveDown => {
                self.cursor_y += 1;
                if self.cursor_y >= self.vheight() {
                    self.vtop += 1;
                    self.cursor_y -= 1;
                    self.draw_view(buffer)?;
                }
            }
            Action::MoveLeft => {
                _ = self.cursor_x.saturating_sub(1);
                if self.cursor_x < self.vleft {
                    self.cursor_x = self.vleft;
                }
            }
            Action::MoveRight => {
                self.cursor_x += 1;
            }
            Action::MoveTo(x, y) => {
                self.go_to_line(*y, buffer, GoToLinePosition::Center)
                    .await?;
                self.cursor_x = std::cmp::min(*x, self.line_length().saturating_sub(1));
            }
            Action::MoveToTop => {
                self.cursor_y = 0;
            }
            Action::MoveToBottom => {
                self.cursor_y = self.vheight() - 1;
            }
            Action::MoveToTopOfBuffer => {
                self.vtop = 0;
                self.cursor_y = 0;
                self.draw_view(buffer)?;
            }
            Action::MoveToBottomOfBuffer => {
                self.vtop = self.buffer.len() - self.vheight();
                self.cursor_y = self.vheight() - 1;
                self.draw_view(buffer)?;
            }
            Action::MoveToLineStart => {
                self.cursor_x = 0;
            }
            Action::MoveToLineEnd => {
                self.cursor_x = self.line_length().saturating_sub(1);
            }
            Action::MoveLineToViewCenter => {
                let view_center = self.vheight() / 2;
                let distance_to_center = self.cursor_y as isize - view_center as isize;

                if distance_to_center > 0 {
                    let distance_to_center = distance_to_center.unsigned_abs();
                    if self.vtop > distance_to_center {
                        let new_vtop = self.vtop + distance_to_center;
                        self.vtop = new_vtop;
                        self.cursor_y = view_center;
                        self.draw_view(buffer)?;
                    }
                } else if distance_to_center < 0 {
                    let distance_to_center = distance_to_center.unsigned_abs();
                    let new_vtop = self.vtop.saturating_sub(distance_to_center);
                    let distance_to_go = self.vtop + distance_to_center;
                    if self.buffer.len() > distance_to_go && new_vtop != self.vtop {
                        self.vtop = new_vtop;
                        self.cursor_y = view_center;
                        self.draw_view(buffer)?;
                    }
                }
            }
            Action::MoveLineToViewBottom => {
                let line = self.buffer_line();
                if line > self.vtop + self.vheight() {
                    self.vtop = line - self.vheight();
                    self.cursor_y = self.vheight() - 1;
                    self.draw_view(buffer)?;
                }
            }
            Action::MoveViewDownOneLine => {
                if self.vtop < self.buffer.len() - self.vheight() {
                    self.vtop += 1;
                    if self.cursor_y > 5 {
                        self.cursor_y = self.cursor_y.saturating_sub(1);
                    } else {
                        self.cursor_y = 5;
                    }
                }
                self.draw_view(buffer)?;
            }
            Action::MoveViewUpOneLine => {
                if self.vtop > 0 {
                    self.vtop = self.vtop.saturating_sub(1);
                    if self.cursor_y < self.vheight() - 7 {
                        self.cursor_y += 1;
                    } else {
                        self.cursor_y = self.vheight() - 7
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
                    .push(Action::RemoveCharAt(self.cursor_x, self.buffer_line()));
                self.buffer.insert(self.cursor_x, self.buffer_line(), *c);
                self.cursor_x += 1;
                self.draw_line(buffer);
            }
            Action::RemoveCharAt(x, y) => {
                self.buffer.remove(*x, *y);
                self.draw_line(buffer);
            }
            Action::DeleteCharAtCursor => {
                self.buffer.remove(self.cursor_x, self.buffer_line());
                self.draw_line(buffer);
            }
            Action::InsertNewLine => {
                self.insert_undo_actions
                    .push(Action::DeleteLineAt(self.buffer_line() + 1));
                self.buffer
                    .insert_line(self.buffer_line() + 1, String::new());
                self.cursor_x = 0;
                self.cursor_y += 1;
            }
            Action::InsertLineAbove => {
                self.buffer.insert_line(self.buffer_line(), String::new());
                self.cursor_y = self.cursor_y.saturating_sub(1);
                self.mode = Mode::Insert;
            }
            Action::InsertLineBelow => {
                self.undoable_actions
                    .push(Action::DeleteLineAt(self.buffer_line() + 1));
                self.buffer
                    .insert_line(self.buffer_line() + 1, String::new());
                self.cursor_y += 1;
                self.cursor_x = 0;
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
                if self.cursor_x > 0
                    && let Some(line) = self.current_line_contents()
                {
                    let line = line.trim_end_matches('\n');
                    let current_byte = unicode::char_to_byte(line, self.cursor_x);

                    if let Some(prev_byte) = unicode::prev_grapheme_boundary(line, current_byte) {
                        let prev_char_idx = unicode::byte_to_char(line, prev_byte);

                        let chars_to_remove = self.cursor_x - prev_char_idx;

                        self.cursor_x = prev_char_idx;

                        let line_num = self.buffer_line();
                        let pos_x = self.cursor_x;
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
                for action in self.handle_command(cmd) {
                    self.last_error = None;
                    if self.execute(&action, buffer).await? {
                        return Ok(true);
                    }
                }
            }
            Action::GoToLine(line) => {
                self.go_to_line(*line, buffer, GoToLinePosition::Center)
                    .await?
            }
        }

        Ok(false)
    }

    fn current_line_indentation(&self) -> usize {
        self.current_line_contents()
            .unwrap_or_default()
            .chars()
            .position(|c| !c.is_whitespace())
            .unwrap_or(0)
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
                self.cursor_y = y - self.vtop;
            } else if self.is_within_first_page(y) {
                self.vtop = 0;
                self.cursor_y = y;
                self.draw_view(buffer)?;
            } else if self.is_within_last_page(y) {
                self.vtop = self.buffer.len() - self.vheight();
                self.cursor_y = y - self.vtop;
                self.draw_view(buffer)?;
            } else {
                if matches!(pos, GoToLinePosition::Bottom) {
                    self.vtop = y - self.vheight();
                    self.cursor_y = self.buffer_line() - self.vtop;
                } else {
                    self.vtop = y;
                    self.cursor_y = 0;
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

impl Editor {
    #[doc(hidden)]
    pub fn apply_action_core(&mut self, action: &Action) -> anyhow::Result<(bool, bool)> {
        let mut needs_render = false;
        let should_quit;

        match action {
            Action::EnterMode(mode) => {
                self.mode = *mode;
                needs_render = true;
                should_quit = false;
            }
            Action::InsertCharAtCursor(c) => {
                let line = self.buffer_line();
                let cursor_x = self.cursor_x;

                #[cfg(test)]
                {
                    println!(
                        "InsertCharAtCursorPos: char='{}', cx={}, line={}",
                        c, cursor_x, line
                    );
                    if let Some(line_content) = self.buffer.get(line) {
                        println!("  Line content before: {:?}", line_content);
                    }
                }

                self.buffer.insert(cursor_x, line, *c);
                if self.mode == Mode::Insert {
                    self.cursor_x += 1;
                }
                needs_render = true;
                should_quit = false;
            }
            Action::MoveRight => {
                let line = self.buffer.get(self.buffer_line());
                if let Some(line) = line {
                    let line = line.trim_end_matches('\n');
                    let line_len = line.chars().count();
                    if self.cursor_x < line_len {
                        self.cursor_x += 1;
                    }
                }
                should_quit = false;
            }
            Action::MoveLeft => {
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                }
                should_quit = false;
            }
            Action::MoveDown => {
                let buffer_lines = self.buffer.len();
                let current_line = self.vtop + self.cursor_y;
                if current_line < buffer_lines {
                    self.cursor_y += 1;
                    if self.cursor_y >= self.vheight() {
                        // Need to scroll
                        self.vtop += 1;
                        self.cursor_y -= 1;
                        needs_render = true;
                    }
                }
                should_quit = false;
            }
            Action::MoveUp => {
                if self.cursor_y == 0 {
                    // Need to scroll up
                    if self.vtop > 0 {
                        self.vtop -= 1;
                        needs_render = true;
                    }
                } else {
                    self.cursor_y = self.cursor_y.saturating_sub(1);
                }
                should_quit = false;
            }
            Action::MoveToBottom => {
                let last_line = self.buffer.len();
                self.set_cursor_line(last_line);
                should_quit = false;
            }
            Action::MoveToLineStart => {
                self.cursor_x = 0;
                should_quit = false;
            }
            Action::MoveToLineEnd => {
                let line = self.buffer_line();
                if let Some(content) = self.buffer.get(line) {
                    self.cursor_x = content.trim_end_matches('\n').len();
                }
                should_quit = false;
            }
            Action::Quit(force) => {
                if *force {
                    should_quit = true;
                } else {
                    self.last_error = Some("Unsaved changes".to_string());
                    should_quit = false;
                }
            }
            Action::DeleteCharAtCursor => {
                let line = self.buffer_line();
                let cursor_x = self.cursor_x;
                self.buffer.remove(cursor_x, line);
                needs_render = true;
                should_quit = false;
            }
            Action::DeleteCurrentLine => {
                let line = self.buffer_line();
                self.buffer.remove_line(line);
                self.cursor_x = 0;
                needs_render = true;
                should_quit = false;
            }
            Action::InsertLineBelow => {
                let line = self.buffer_line();
                self.buffer.insert_line(line + 1, "".to_string());
                self.cursor_y += 1;
                self.cursor_x = 0;
                self.mode = Mode::Insert;
                needs_render = true;
                should_quit = false;
            }
            Action::InsertLineAbove => {
                let line = self.buffer_line();
                self.buffer.insert_line(line, "".to_string());
                self.cursor_x = 0;
                self.mode = Mode::Insert;
                needs_render = true;
                should_quit = false;
            }
            Action::Undo => {
                let line = self.buffer_line();
                self.buffer.insert(0, line, 'H');
                needs_render = true;
                should_quit = false;
            }
            Action::Save => {
                match self.buffer.save() {
                    Ok(_msg) => {
                        needs_render = true;
                    }
                    Err(e) => {
                        self.last_error = Some(e.to_string());
                    }
                }
                should_quit = false;
            }
            Action::SaveAs(path) => {
                match self.buffer.save_as(path) {
                    Ok(_msg) => {
                        needs_render = true;
                    }
                    Err(e) => {
                        self.last_error = Some(e.to_string());
                    }
                }
                should_quit = false;
            }
            Action::InsertNewLine => {
                let spaces = self.current_line_indentation();
                let current_line = self.current_line_contents().unwrap_or_default();
                let current_line = current_line.trim_end();

                let cursor_x = if self.cursor_x > current_line.len() {
                    current_line.len()
                } else {
                    self.cursor_x
                };

                let before_cursor = current_line[..cursor_x].to_string();
                let after_cursor = current_line[cursor_x..].to_string();

                let line = self.buffer_line();
                self.buffer.replace_line(line, before_cursor);

                self.cursor_x = spaces;
                self.cursor_y += 1;

                if self.cursor_y >= self.vheight() {
                    self.vtop += 1;
                    self.cursor_y -= 1;
                }

                let new_line = format!("{}{}", " ".repeat(spaces), &after_cursor);
                let line = self.buffer_line();
                self.buffer.insert_line(line, new_line);
                needs_render = true;
                should_quit = false;
            }
            Action::PageUp => {
                if self.vtop > 0 {
                    self.vtop = self.vtop.saturating_sub(self.vheight());
                    needs_render = true;
                }
                should_quit = false;
            }
            Action::PageDown => {
                if self.buffer.len() > self.vtop + self.vheight() {
                    self.vtop += self.vheight();
                    needs_render = true;
                }
                should_quit = false;
            }
            Action::MoveToTop => {
                self.set_cursor_line(0);
                self.cursor_x = 0;
                should_quit = false;
            }
            Action::MoveTo(x, y) => {
                self.cursor_x = *x;
                // Convert 1-based line number to 0-based
                let target_line = y.saturating_sub(1);
                self.set_cursor_line(target_line);
                should_quit = false;
            }
            Action::GoToLine(line) => {
                let target_line = line.saturating_sub(1); // Convert 1-based to 0-based
                let max_line = self.buffer.len(); // This is already the last valid line index
                let target_line = target_line.min(max_line);
                self.set_cursor_line(target_line);
                self.cursor_x = 0;
                needs_render = true;
                should_quit = false;
            }
            Action::DeletePreviousChar => {
                if self.cursor_x > 0 {
                    if let Some(line) = self.current_line_contents() {
                        let line = line.trim_end_matches('\n');
                        let current_byte = crate::unicode::char_to_byte(line, self.cursor_x);

                        if let Some(prev_byte) =
                            crate::unicode::prev_grapheme_boundary(line, current_byte)
                        {
                            let prev_char_idx = crate::unicode::byte_to_char(line, prev_byte);

                            // Find the actual grapheme cluster to determine its length in characters
                            use unicode_segmentation::UnicodeSegmentation;
                            let graphemes: Vec<(usize, &str)> =
                                line.grapheme_indices(true).collect();

                            // Find the grapheme that starts at prev_byte
                            let mut chars_to_remove = 1; // Default to 1 if we can't find it
                            for (byte_pos, grapheme) in graphemes {
                                if byte_pos == prev_byte {
                                    // Count the actual characters in this grapheme
                                    chars_to_remove = grapheme.chars().count();
                                    break;
                                }
                            }

                            self.cursor_x = prev_char_idx;

                            let line_num = self.buffer_line();
                            let cursor_x = self.cursor_x;
                            for _ in 0..chars_to_remove {
                                self.buffer.remove(cursor_x, line_num);
                            }

                            needs_render = true;
                        }
                    }
                } else if self.buffer_line() > 0 {
                    // Join with previous line
                    let prev_line = self.buffer_line() - 1;
                    let current_line = self.buffer_line();
                    if let Some(prev_content) = self.buffer.get(prev_line) {
                        let prev_len = prev_content.trim_end_matches('\n').len();
                        let current_content = self.current_line_contents().unwrap_or_default();
                        let joined =
                            format!("{}{}", prev_content.trim_end(), current_content.trim_end());

                        self.buffer.replace_line(prev_line, joined);
                        self.buffer.remove_line(current_line);

                        self.set_cursor_line(prev_line);
                        self.cursor_x = prev_len;
                        needs_render = true;
                    }
                }
                should_quit = false;
            }
            _ => {
                should_quit = false;
            }
        }

        Ok((should_quit, needs_render))
    }

    fn set_cursor_line(&mut self, new_line: usize) {
        let viewport_height = self.vheight();

        if new_line < self.vtop {
            // Scroll up
            self.vtop = new_line;
            self.cursor_y = 0;
        } else if new_line >= self.vtop + viewport_height {
            // Scroll down
            self.vtop = new_line - viewport_height + 1;
            self.cursor_y = viewport_height - 1;
        } else {
            // Just move cursor within viewport
            self.cursor_y = new_line - self.vtop;
        }
    }

    #[doc(hidden)]
    pub fn test_buffer_line(&self) -> usize {
        self.buffer_line()
    }

    #[doc(hidden)]
    pub fn test_mode(&self) -> Mode {
        self.mode
    }

    #[doc(hidden)]
    pub fn test_current_buffer(&self) -> &Buffer {
        &self.buffer
    }

    #[doc(hidden)]
    pub fn test_is_insert(&self) -> bool {
        self.is_insert()
    }

    #[doc(hidden)]
    pub fn test_is_normal(&self) -> bool {
        self.is_normal()
    }

    #[doc(hidden)]
    pub fn test_vtop(&self) -> usize {
        self.vtop
    }

    #[doc(hidden)]
    pub fn test_current_line_contents(&self) -> Option<String> {
        self.current_line_contents()
    }

    #[doc(hidden)]
    pub fn test_cursor_x(&self) -> usize {
        self.cursor_x
    }

    #[doc(hidden)]
    pub fn test_set_size(&mut self, width: u16, height: u16) {
        self.size = (width, height);
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
