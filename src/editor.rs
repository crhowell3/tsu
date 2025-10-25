#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

pub mod render;
pub mod render_buffer;

use std::{cmp::Ordering, collections::HashMap, mem};

use crossterm::{
    ExecutableCommand,
    event::{self, Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};
use futures::{StreamExt, future::FutureExt, select};
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    action::Action,
    buffer::Buffer,
    color::Color,
    command,
    config::{Config, KeyAction},
    debug,
    editor::render_buffer::RenderBuffer,
    graphics::Style,
    highlighter::Highlighter,
    log,
    theme::Theme,
    unicode::{self, byte_to_char, char_to_byte, next_grapheme_boundary, prev_grapheme_boundary},
    window_manager::WindowManager,
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
    #[must_use]
    pub fn contains(&self, pos: usize) -> bool {
        pos >= self.start && pos < self.end
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

impl Point {
    #[must_use]
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.y.cmp(&other.y) {
            Ordering::Equal => self.x.partial_cmp(&other.x),
            ordering => Some(ordering),
        }
    }
}

pub struct Editor {
    config: Config,
    pub theme: Theme,
    highlighter: Highlighter,
    buffers: Vec<Buffer>,
    current_buffer_index: usize,
    size: (u16, u16),
    window_manager: WindowManager,
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
    /// Creates a new editor with a given size
    ///
    /// # Arguments
    /// - `width`: The desired width of the editor viewport
    /// - `height`: The desired height of the editor viewport
    /// - `config`: A configuration constructed from a parsed config file
    /// - `theme`: A theme for coloring, styling, and decorating the editor
    /// - `buffers`: A buffer which will contain the contents of the file being edited
    ///
    /// # Errors
    /// Can return errors if `width` and `height` are larger than `u16::MAX` or if creating a new
    /// `Highlighter` with the provided `theme` fails
    ///
    /// # Panics
    /// This function may panic if `width` and `height` are larger than `u16::MAX`
    pub fn with_size(
        width: usize,
        height: usize,
        config: Config,
        theme: Theme,
        buffers: Vec<Buffer>,
    ) -> anyhow::Result<Self> {
        let stdout = std::io::stdout();
        let vx = buffers.first().map_or(0, |b| b.len().to_string().len()) + 2;
        let w = u16::try_from(width).expect("value too large to fit in u16");
        let h = u16::try_from(height).expect("value too large to fit in u16");
        let size = (w, h);
        let highlighter = Highlighter::new(&theme)?;

        let window_manager = WindowManager::new(0, (width, height));

        Ok(Self {
            config,
            theme,
            buffers,
            current_buffer_index: 0,
            highlighter,
            window_manager,
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

    /// Creates a new editor with a configuration, theme, and buffer
    ///
    /// # Arguments
    /// - `config`: A configuration constructed from a parsed config file
    /// - `theme`: A theme for coloring, styling, and decorating the editor
    /// - `buffers`: A buffer which will contain the contents of the file being edited
    ///
    /// # Errors
    /// Can return an `IoError` if the call to query the terminal's size fails
    pub fn new(config: Config, theme: Theme, buffers: Vec<Buffer>) -> anyhow::Result<Self> {
        let size = terminal::size()?;
        Self::with_size(size.0 as usize, size.1 as usize, config, theme, buffers)
    }

    #[allow(unused)]
    fn sync_with_window(&mut self) {
        if let Some(window) = self.window_manager.active_window() {
            self.current_buffer_index = window.buffer_index;
            self.vtop = window.vtop;
            self.vleft = window.vleft;
            self.cursor_x = window.cursor_x;
            self.cursor_y = window.cursor_y;
            self.vx = window.vx;
        }
    }

    fn sync_to_window(&mut self) {
        if let Some(window) = self.window_manager.active_window_mut() {
            window.buffer_index = self.current_buffer_index;
            window.vtop = self.vtop;
            window.vleft = self.vleft;
            window.cursor_x = self.cursor_x;
            window.cursor_y = self.cursor_y;
            window.vx = self.vx;
        }
    }

    #[must_use]
    pub fn vwidth(&self) -> usize {
        self.size.0 as usize
    }

    #[must_use]
    pub fn vheight(&self) -> usize {
        self.size.1 as usize - 2
    }

    #[must_use]
    pub fn window_to_terminal_x(&self, window: &crate::window::Window, x: usize) -> usize {
        window.position.x + x
    }

    #[must_use]
    pub fn window_to_terminal_y(&self, window: &crate::window::Window, y: usize) -> usize {
        window.position.y + y
    }

    #[must_use]
    pub fn buffer_to_window_coords(
        &self,
        window: &crate::window::Window,
        buf_x: usize,
        buf_y: usize,
    ) -> Option<(usize, usize)> {
        if buf_y < window.vtop || buf_y >= window.vtop + window.inner_height() {
            return None;
        }

        if buf_x < window.vleft || buf_x >= window.vleft + window.inner_width() {
            return None;
        }

        let window_x = buf_x - window.vleft;
        let window_y = buf_y - window.vtop;

        Some((window_x, window_y))
    }

    #[must_use]
    pub fn window_vwidth(&self, window: &crate::window::Window) -> usize {
        window.inner_width()
    }

    #[must_use]
    pub fn window_vheight(&self, window: &crate::window::Window) -> usize {
        window.inner_height()
    }

    #[must_use]
    pub fn cursor_position(&self) -> (usize, usize) {
        (self.vx + self.cursor_x, self.cursor_y)
    }

    fn line_length(&self) -> usize {
        if let Some(line) = self.view_line(self.cursor_y) {
            let line = line.trim_end_matches('\n');
            return line.chars().count();
        }
        0
    }

    fn buffer_line(&self) -> usize {
        self.vtop + self.cursor_y
    }

    fn view_line(&self, n: usize) -> Option<String> {
        let line = self.vtop + n;

        self.current_buffer().get(line)
    }

    fn current_buffer(&self) -> &Buffer {
        &self.buffers[self.current_buffer_index]
    }

    fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.current_buffer_index]
    }

    fn modified_buffers(&self) -> Vec<&str> {
        self.buffers
            .iter()
            .filter(|b| b.is_dirty())
            .map(super::buffer::Buffer::name)
            .collect()
    }

    fn gutter_width(&self) -> usize {
        self.current_buffer().len().to_string().len() + 1
    }

    /// Wrapper for the editor's highlighter functionality
    ///
    /// # Arguments
    /// - `code`: The contents of a file represented as a string and assumed to be "code", i.e.,
    ///   pertaining to a programming language supported by the tree sitter
    ///
    /// # Errors
    /// Could potentially return an error if the highlighter throws an error
    pub fn highlight(&mut self, code: &str) -> anyhow::Result<Vec<StyleInfo>> {
        self.highlighter.highlight(code)
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

        if self.cursor_x >= line_len && self.is_normal() {
            if line_len > 0 {
                self.cursor_x = self.line_length() - 1;
            } else if self.is_normal() {
                self.cursor_x = 0;
            }
        }

        if self.cursor_x >= self.vwidth() {
            self.cursor_x = self.vwidth() - 1;
        }

        let line_in_buffer = self.cursor_y + self.vtop;
        if line_in_buffer > self.current_buffer().len().saturating_sub(1) {
            self.cursor_y = self.current_buffer().len() - self.vtop - 1;
        }
    }

    /// Executes the primary run loop for the application
    ///
    /// # Errors
    /// Can return an error if any failures occur with enabling raw mode in the terminal or
    /// executing some command to stdout
    pub async fn run(&mut self) -> anyhow::Result<()> {
        terminal::enable_raw_mode()?;
        self.stdout
            .execute(event::EnableMouseCapture)?
            .execute(terminal::EnterAlternateScreen)?
            .execute(terminal::Clear(terminal::ClearType::All))?;

        let mut buffer = RenderBuffer::new(
            self.size.0 as usize,
            self.size.1 as usize,
            &Style::default(),
        );

        self.render(&mut buffer)?;

        let mut reader = EventStream::new();

        loop {
            let mut event = reader.next().fuse();

            select! {
                maybe_event = event => {
                    match maybe_event {
                        Some(Ok(ev)) => {
                            self.check_bounds();

                            if let event::Event::Resize(width, height) = ev {
                                self.size = (width, height);
                                let max_y = height as usize - 2;
                                if self.cursor_y > max_y - 1 {
                                    self.cursor_y = max_y - 1;
                                }

                                self.window_manager.resize((width as usize, height as usize));
                                self.sync_to_window();
                                buffer = RenderBuffer::new(
                                    self.size.0 as usize,
                                    self.size.1 as usize,
                                    &Style::default(),
                                );

                                self.render(&mut buffer)?;
                                continue;
                            }

                            if let Some(action) = self.handle_event(&ev)
                                && self.handle_key_action(&ev, &action, &mut buffer).await? {
                                    break;
                                }
                            self.render(&mut buffer)?;
                        },
                        Some(Err(error)) => {
                            log!("error: {error}");
                        },
                        None => {}
                    }
                }
            }
        }

        Ok(())
    }

    #[async_recursion::async_recursion]
    async fn handle_key_action(
        &mut self,
        ev: &event::Event,
        action: &KeyAction,
        buffer: &mut RenderBuffer,
    ) -> anyhow::Result<bool> {
        let quit = match action {
            KeyAction::None => false,
            KeyAction::Single(action) => self.execute(action, buffer).await?,
            KeyAction::Multiple(actions) => {
                let mut quit = false;
                for action in actions {
                    if self.execute(action, buffer).await? {
                        quit = true;
                        break;
                    }
                }
                quit
            }
            KeyAction::Nested(actions) => {
                if let Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    ..
                }) = ev
                {
                    self.waiting_command = Some(format!("{c}"));
                }
                self.waiting_key_action = Some(KeyAction::Nested(actions.clone()));
                false
            }
            KeyAction::Repeating(times, action) => {
                let mut quit = false;
                for _ in 0..*times as usize {
                    if self.handle_key_action(ev, action, buffer).await? {
                        quit = true;
                        break;
                    }
                }
                quit
            }
        };

        Ok(quit)
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
        let normal = self.config.keys.normal.clone();
        self.event_to_key_action(&normal, ev)
    }

    fn handle_insert_event(&mut self, ev: &event::Event) -> Option<KeyAction> {
        let insert = self.config.keys.insert.clone();
        if let Some(key_action) = self.event_to_key_action(&insert, ev) {
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

    #[allow(clippy::unused_self)]
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

    #[allow(clippy::unused_self)]
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

    /// Safely cleans up the terminal before exiting
    ///
    /// # Errors
    /// Can return an error if any of the subfunctions fail
    pub fn cleanup(&mut self) -> anyhow::Result<()> {
        self.stdout
            .execute(terminal::LeaveAlternateScreen)?
            .execute(event::DisableMouseCapture)?;
        terminal::disable_raw_mode()?;

        Ok(())
    }

    fn current_line_contents(&self) -> Option<String> {
        self.current_buffer().get(self.buffer_line())
    }

    async fn execute(
        &mut self,
        action: &Action,
        buffer: &mut RenderBuffer,
    ) -> anyhow::Result<bool> {
        self.execute_with_tracking(action, buffer, true).await
    }

    #[allow(clippy::too_many_lines)]
    #[async_recursion::async_recursion]
    async fn execute_with_tracking(
        &mut self,
        action: &Action,
        buffer: &mut RenderBuffer,
        _tracking: bool,
    ) -> anyhow::Result<bool> {
        self.last_error = None;

        match action {
            Action::Quit(force) => {
                if *force {
                    return Ok(true);
                }

                let modified_buffers = self.modified_buffers();
                if modified_buffers.is_empty() {
                    return Ok(true);
                }

                self.last_error = Some("Buffer has unwritten changes".to_string());
                return Ok(false);
            }
            Action::Save => match self.current_buffer_mut().save() {
                Ok(msg) => {
                    self.last_error = Some(msg);
                }
                Err(e) => {
                    self.last_error = Some(e.to_string());
                }
            },
            Action::SaveAs(new_file_name) => match self.current_buffer_mut().save_as(new_file_name)
            {
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
                let cursor_y = isize::try_from(self.cursor_y)
                    .expect("value of cursor_y will not fit in isize");
                let vc = isize::try_from(view_center)
                    .expect("value of view_center will not fit in isize");
                let distance_to_center = cursor_y - vc;

                if distance_to_center > 0 {
                    let distance_to_center = distance_to_center.unsigned_abs();
                    self.vtop += distance_to_center;
                    self.cursor_y = view_center;
                } else if distance_to_center < 0 {
                    let distance_to_center = distance_to_center.unsigned_abs();
                    let new_vtop = self.vtop.saturating_sub(distance_to_center);
                    if self.current_buffer().len() > self.vtop + distance_to_center
                        && new_vtop != self.vtop
                    {
                        self.vtop = new_vtop;
                        self.cursor_y = view_center;
                    }
                }
            }
            Action::MoveUp => {
                if self.cursor_y == 0 {
                    if self.vtop > 0 {
                        self.vtop -= 1;
                    }
                } else {
                    self.cursor_y = self.cursor_y.saturating_sub(1);
                    self.draw_cursor()?;
                }
            }
            Action::MoveDown => {
                if self.vtop + self.cursor_y < self.current_buffer().len() - 1 {
                    self.cursor_y += 1;
                    if self.cursor_y >= self.vheight() {
                        self.vtop += 1;
                        self.cursor_y -= 1;
                    }
                } else {
                    self.draw_cursor()?;
                }
            }
            Action::MoveLeft => {
                if let Some(line) = self.current_line_contents() {
                    let line = line.trim_end_matches('\n');

                    let current_byte = self
                        .current_buffer()
                        .column_to_char_index(self.cursor_x, self.buffer_line());
                    let byte_offset = char_to_byte(line, current_byte);

                    if let Some(prev_byte) = prev_grapheme_boundary(line, byte_offset) {
                        let char_idx = byte_to_char(line, prev_byte);
                        self.cursor_x = char_idx;
                    } else if self.cursor_x > 0 {
                        self.cursor_x = 0;
                    }

                    if self.cursor_x < self.vleft {
                        self.cursor_x = self.vleft;
                    }
                }
            }
            Action::MoveRight => {
                if let Some(line) = self.current_line_contents() {
                    let line = line.trim_end_matches('\n');
                    let max_chars = line.chars().count();

                    if self.cursor_x < max_chars {
                        let current_byte = char_to_byte(line, self.cursor_x);

                        if let Some(next_byte) = next_grapheme_boundary(line, current_byte) {
                            let char_idx = byte_to_char(line, next_byte);
                            self.cursor_x = char_idx.min(max_chars);
                        } else {
                            self.cursor_x = max_chars;
                        }
                    }
                }
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
            }
            Action::MoveToBottomOfBuffer => {
                self.vtop = self.current_buffer().len() - self.vheight();
                self.cursor_y = self.vheight() - 1;
            }
            Action::MoveToLineStart => {
                self.cursor_x = 0;
            }
            Action::MoveToLineEnd => {
                self.cursor_x = self.line_length().saturating_sub(1);
            }
            Action::MoveLineToViewCenter => {
                let view_center = self.vheight() / 2;
                let cursor_y = isize::try_from(self.cursor_y)
                    .expect("value of cursor_y will not fit in isize");
                let vc = isize::try_from(view_center)
                    .expect("value of view_center will not fit in isize");
                let distance_to_center = cursor_y - vc;

                if distance_to_center > 0 {
                    let distance_to_center = distance_to_center.unsigned_abs();
                    if self.vtop > distance_to_center {
                        let new_vtop = self.vtop + distance_to_center;
                        self.vtop = new_vtop;
                        self.cursor_y = view_center;
                    }
                } else if distance_to_center < 0 {
                    let distance_to_center = distance_to_center.unsigned_abs();
                    let new_vtop = self.vtop.saturating_sub(distance_to_center);
                    let distance_to_go = self.vtop + distance_to_center;
                    if self.current_buffer().len() > distance_to_go && new_vtop != self.vtop {
                        self.vtop = new_vtop;
                        self.cursor_y = view_center;
                    }
                }
            }
            Action::MoveLineToViewBottom => {
                let line = self.buffer_line();
                if line > self.vtop + self.vheight() {
                    self.vtop = line - self.vheight();
                    self.cursor_y = self.vheight() - 1;
                }
            }
            Action::MoveViewDownOneLine => {
                if self.vtop < self.current_buffer().len() - self.vheight() {
                    self.vtop += 1;
                    if self.cursor_y > 5 {
                        self.cursor_y = self.cursor_y.saturating_sub(1);
                    } else {
                        self.cursor_y = 5;
                    }
                }
            }
            Action::MoveViewUpOneLine => {
                if self.vtop > 0 {
                    self.vtop = self.vtop.saturating_sub(1);
                    if self.cursor_y < self.vheight() - 7 {
                        self.cursor_y += 1;
                    } else {
                        self.cursor_y = self.vheight() - 7;
                    }
                }
            }
            Action::PageUp => {
                if self.vtop > 0 {
                    self.vtop = self.vtop.saturating_sub(self.vheight());
                }
            }
            Action::PageDown => {
                if self.current_buffer().len() > (self.vtop + self.vheight()) {
                    self.vtop += self.vheight();
                }
            }
            Action::EnterMode(new_mode) => {
                debug!("Entering mode: {:?}", new_mode);
                if self.is_normal() && matches!(new_mode, Mode::Insert) {
                    self.insert_undo_actions = Vec::new();
                }

                if self.is_insert()
                    && matches!(new_mode, Mode::Normal)
                    && !self.insert_undo_actions.is_empty()
                {
                    let actions = mem::take(&mut self.insert_undo_actions);
                    self.undoable_actions.push(Action::UndoMultiple(actions));
                }

                self.mode = *new_mode;
            }
            Action::InsertCharAtCursor(c) => {
                self.insert_undo_actions
                    .push(Action::RemoveCharAt(self.cursor_x, self.buffer_line()));
                let line = self.buffer_line();
                let cursor_x = self.cursor_x;

                crate::log!(
                    "InsertCharAtCursor - char: '{}' (U+{:04x}), cursor_x: {}, line: {}",
                    c,
                    *c as u32,
                    cursor_x,
                    line
                );

                if let Some(line_content) = self.current_buffer().get(line) {
                    crate::log!("Line content before insert: {:?}", line_content);
                    crate::log!("Line char count: {}", line_content.chars().count());
                }

                self.current_buffer_mut().insert(cursor_x, line, *c);
                self.cursor_x += 1;
            }
            Action::RemoveCharAt(x, y) => {
                self.current_buffer_mut().remove(*x, *y);
            }
            Action::DeleteCharAtCursor => {
                let cursor_x = self.cursor_x;
                let line = self.buffer_line();

                self.current_buffer_mut().remove(cursor_x, line);
            }
            Action::ReplaceLineAt(y, contents) => {
                self.current_buffer_mut().replace_line(*y, contents);
            }
            Action::InsertNewLine => {
                self.insert_undo_actions.extend(vec![
                    Action::MoveTo(self.cursor_x, self.buffer_line() + 1),
                    Action::DeleteLineAt(self.buffer_line() + 1),
                    Action::ReplaceLineAt(
                        self.buffer_line(),
                        self.current_line_contents().unwrap_or_default(),
                    ),
                ]);
                let spaces = self.current_line_indentation();

                let current_line = self.current_line_contents().unwrap_or_default();
                let current_line = current_line.trim_end();
                if self.cursor_x > current_line.len() {
                    self.cursor_x = current_line.len();
                }
                let before_cursor = current_line[..self.cursor_x].to_string();
                let after_cursor = current_line[self.cursor_x..].to_string();

                let line = self.buffer_line();
                self.current_buffer_mut().replace_line(line, &before_cursor);

                self.cursor_x = spaces;
                self.cursor_y += 1;

                if self.cursor_y >= self.vheight() {
                    self.vtop += 1;
                    self.cursor_y -= 1;
                }

                let new_line = format!("{}{}", " ".repeat(spaces), &after_cursor);
                let line = self.buffer_line();

                self.current_buffer_mut().insert_line(line, &new_line);
            }
            Action::InsertLineAbove => {
                self.undoable_actions
                    .push(Action::DeleteLineAt(self.buffer_line()));

                let leading_spaces = if let Some(line) = self.current_line_contents() {
                    if line.is_empty() {
                        self.previous_line_indentation()
                    } else {
                        self.current_line_indentation()
                    }
                } else {
                    self.previous_line_indentation()
                };

                let line = self.buffer_line();
                self.current_buffer_mut()
                    .insert_line(line, &" ".repeat(leading_spaces));
                self.cursor_x = leading_spaces;
            }
            Action::InsertLineBelow => {
                self.undoable_actions
                    .push(Action::DeleteLineAt(self.buffer_line() + 1));

                let leading_spaces = self.current_line_indentation();
                let line = self.buffer_line();

                self.current_buffer_mut()
                    .insert_line(line + 1, &" ".repeat(leading_spaces));
                self.cursor_y += 1;
                self.cursor_x = leading_spaces;

                if self.cursor_y >= self.vheight() {
                    self.vtop += 1;
                    self.cursor_y -= 1;
                }
            }
            Action::InsertLineAt(line, contents) => {
                self.undoable_actions
                    .push(Action::DeleteLineAt(self.buffer_line()));
                if let Some(contents) = contents {
                    self.current_buffer_mut().insert_line(*line, contents);
                }
            }
            Action::DeleteCurrentLine => {
                let line = self.buffer_line();
                let contents = self.current_line_contents();

                self.current_buffer_mut().remove_line(line);
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
                            self.current_buffer_mut().remove(pos_x, line_num);
                        }
                    }
                }
            }
            Action::SetWaitingKey(key_action) => {
                self.waiting_key_action = Some(*(key_action.clone()));
            }
            Action::DeleteLineAt(y) => {
                self.current_buffer_mut().remove_line(*y);
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
                    .await?;
            }
        }

        self.sync_to_window();

        if self.window_manager.windows().len() > 1 {
            self.render(buffer)?;
        }

        Ok(false)
    }

    fn previous_line_indentation(&self) -> usize {
        if self.buffer_line() > 0 {
            self.current_buffer()
                .get(self.buffer_line() - 1)
                .unwrap_or_default()
                .chars()
                .position(|c| !c.is_whitespace())
                .unwrap_or(0)
        } else {
            0
        }
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

        if line <= self.current_buffer().len() {
            let y = line - 1;

            if self.is_within_view(y) {
                self.cursor_y = y - self.vtop;
            } else if self.is_within_first_page(y) {
                self.vtop = 0;
                self.cursor_y = y;
                self.render(buffer)?;
            } else if self.is_within_last_page(y) {
                self.vtop = self.current_buffer().len() - self.vheight();
                self.cursor_y = y - self.vtop;
                self.render(buffer)?;
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

                self.render(buffer)?;
            }
        }
        Ok(())
    }

    fn is_within_view(&self, y: usize) -> bool {
        (self.vtop..self.vtop + self.vheight()).contains(&y)
    }

    fn is_within_last_page(&self, y: usize) -> bool {
        y > self.current_buffer().len() - self.vheight()
    }

    fn is_within_first_page(&self, y: usize) -> bool {
        y < self.vheight()
    }

    #[allow(clippy::unused_self)]
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

    fn fix_cursor_position(&mut self) {
        let line_len = self.line_length();

        if self.is_normal() && line_len > 0 {
            if self.cursor_x >= line_len {
                self.cursor_x = line_len.saturating_sub(1);
            }
        } else if self.cursor_x > line_len {
            self.cursor_x = line_len;
        }
    }
}

impl Editor {
    #[doc(hidden)]
    #[allow(clippy::too_many_lines)]
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
                    if let Some(line_content) = self.current_buffer().get(line) {
                        println!("  Line content before: {:?}", line_content);
                    }
                }

                self.current_buffer_mut().insert(cursor_x, line, *c);
                if self.mode == Mode::Insert {
                    self.cursor_x += 1;
                }
                needs_render = true;
                should_quit = false;
            }
            Action::MoveRight => {
                let line = self.current_buffer().get(self.buffer_line());
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
                let buffer_lines = self.current_buffer().len();
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
                let last_line = self.current_buffer().len();
                self.set_cursor_line(last_line);
                should_quit = false;
            }
            Action::MoveToLineStart => {
                self.cursor_x = 0;
                should_quit = false;
            }
            Action::MoveToLineEnd => {
                let line = self.buffer_line();
                if let Some(content) = self.current_buffer().get(line) {
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
                self.current_buffer_mut().remove(cursor_x, line);
                needs_render = true;
                should_quit = false;
            }
            Action::DeleteCurrentLine => {
                let line = self.buffer_line();
                self.current_buffer_mut().remove_line(line);
                self.cursor_x = 0;
                needs_render = true;
                should_quit = false;
            }
            Action::InsertLineBelow => {
                let line = self.buffer_line();
                self.current_buffer_mut().insert_line(line + 1, "");
                self.cursor_y += 1;
                self.cursor_x = 0;
                self.mode = Mode::Insert;
                needs_render = true;
                should_quit = false;
            }
            Action::InsertLineAbove => {
                let line = self.buffer_line();
                self.current_buffer_mut().insert_line(line, "");
                self.cursor_x = 0;
                self.mode = Mode::Insert;
                needs_render = true;
                should_quit = false;
            }
            Action::Undo => {
                let line = self.buffer_line();
                self.current_buffer_mut().insert(0, line, 'H');
                needs_render = true;
                should_quit = false;
            }
            Action::Save => {
                match self.current_buffer_mut().save() {
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
                match self.current_buffer_mut().save_as(path) {
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
                self.current_buffer_mut().replace_line(line, &before_cursor);

                self.cursor_x = spaces;
                self.cursor_y += 1;

                if self.cursor_y >= self.vheight() {
                    self.vtop += 1;
                    self.cursor_y -= 1;
                }

                let new_line = format!("{}{}", " ".repeat(spaces), &after_cursor);
                let line = self.buffer_line();
                self.current_buffer_mut().insert_line(line, &new_line);
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
                if self.current_buffer().len() > self.vtop + self.vheight() {
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
                let target_line = line.saturating_sub(1);
                let max_line = self.current_buffer().len();
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
                                self.current_buffer_mut().remove(cursor_x, line_num);
                            }

                            needs_render = true;
                        }
                    }
                } else if self.buffer_line() > 0 {
                    // Join with previous line
                    let prev_line = self.buffer_line() - 1;
                    let current_line = self.buffer_line();
                    if let Some(prev_content) = self.current_buffer().get(prev_line) {
                        let prev_len = prev_content.trim_end_matches('\n').len();
                        let current_content = self.current_line_contents().unwrap_or_default();
                        let joined =
                            format!("{}{}", prev_content.trim_end(), current_content.trim_end());

                        self.current_buffer_mut().replace_line(prev_line, &joined);
                        self.current_buffer_mut().remove_line(current_line);

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
    #[must_use]
    pub fn test_buffer_line(&self) -> usize {
        self.buffer_line()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn test_mode(&self) -> Mode {
        self.mode
    }

    #[doc(hidden)]
    #[must_use]
    pub fn test_current_buffer(&self) -> &Buffer {
        self.current_buffer()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn test_is_insert(&self) -> bool {
        self.is_insert()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn test_is_normal(&self) -> bool {
        self.is_normal()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn test_vtop(&self) -> usize {
        self.vtop
    }

    #[doc(hidden)]
    #[must_use]
    pub fn test_current_line_contents(&self) -> Option<String> {
        self.current_line_contents()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn test_cursor_x(&self) -> usize {
        self.cursor_x
    }

    #[doc(hidden)]
    pub fn test_set_size(&mut self, width: u16, height: u16) {
        self.size = (width, height);
    }
}

/// Determines the style of a given position based on computed `StyleInfo` data
///
/// # Arguments
/// - `style_info`: A slice of a `Vec<StyleInfo>` to use to lookup the `Style` for a given position
/// - `pos`: The position in question
///
/// # Returns
/// - A `Style` if one was able to be determined, `None` if otherwise
fn determine_style_for_position(style_info: &[StyleInfo], pos: usize) -> Option<Style> {
    if let Some(s) = style_info
        .iter()
        .find(|style_info| style_info.contains(pos))
    {
        return Some(s.style.clone());
    }

    None
}

#[allow(unused)]
/// Adjusts the opacity of a color given some percentage
///
/// # Arguments
/// - `color`: The color to adjust
/// - `percentage`: The desired brightness adjustment percentage of the color
///
/// # Returns
/// - An adjusted color based on the given percentage
fn adjust_color_brightness(color: Option<Color>, percentage: i32) -> Option<Color> {
    let color = color?;

    if let Color::Rgb { r, g, b } = color {
        let adjust = |component: u8| -> u8 {
            let delta = (255.0 * (percentage as f32 / 100.0)) as i32;
            let new_component = i32::from(component) + delta;
            if new_component > 255 {
                255
            } else if new_component < 0 {
                0
            } else {
                new_component as u8
            }
        };

        let r = adjust(r);
        let g = adjust(g);
        let b = adjust(b);

        let new_color = Color::Rgb { r, g, b };

        Some(new_color)
    } else {
        Some(color)
    }
}
