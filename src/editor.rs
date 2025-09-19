use std::io::Write;

use crossterm::{
    ExecutableCommand, QueueableCommand, cursor,
    event::{self, read},
    style::{self, Color, Stylize},
    terminal,
};

use crate::{buffer::Buffer, log};

#[derive(Debug, PartialEq, Eq)]
enum Action {
    // Movement
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    MoveToLineEnd,
    MoveToLineStart,

    // Text editing
    InsertCharAtCursor(char),
    NewLine,
    DeleteCharAtCursor,
    DeleteCurrentLine,

    // Misc
    EnterMode(Mode),
    SetComboCommand(char),

    // Buffer actions
    Save,
    Quit,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Command,
    Visual,
    Replace,
}

#[derive(Debug)]
pub struct Editor {
    buffer: Buffer,
    size: (u16, u16),
    stdout: std::io::Stdout,
    vtop: u16,
    vleft: u16,
    pos_x: u16,
    pos_y: u16,
    mode: Mode,
    combo_command: Option<char>,
}

impl Editor {
    pub fn new(buffer: Buffer) -> anyhow::Result<Self> {
        let mut stdout = std::io::stdout();
        terminal::enable_raw_mode()?;
        stdout
            .execute(terminal::EnterAlternateScreen)?
            .execute(terminal::Clear(terminal::ClearType::All))?;

        Ok(Self {
            buffer,
            stdout,
            vtop: 0,
            vleft: 0,
            pos_x: 0,
            pos_y: 0,
            mode: Mode::Normal,
            size: terminal::size()?,
            combo_command: None,
        })
    }

    fn vwidth(&self) -> u16 {
        self.size.0
    }

    fn vheight(&self) -> u16 {
        self.size.1 - 2
    }

    fn line_length(&self) -> u16 {
        if let Some(line) = self.view_line(self.pos_y) {
            return line.len() as u16;
        }
        0
    }

    fn buffer_line(&self) -> u16 {
        self.vtop + self.pos_y
    }

    fn view_line(&self, n: u16) -> Option<String> {
        let line = self.vtop + n;
        self.buffer.get(line as usize)
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        self.draw_view()?;
        self.draw_status_line()?;
        self.stdout.queue(cursor::MoveTo(self.pos_x, self.pos_y))?;
        self.stdout.flush()?;

        Ok(())
    }

    pub fn draw_view(&mut self) -> anyhow::Result<()> {
        let vwidth = self.vwidth() as usize;
        for line_num in 0..self.vheight() {
            let line = self.view_line(line_num).unwrap_or_default();

            self.stdout
                .queue(cursor::MoveTo(0, line_num))?
                .queue(style::Print(format!("{line:<width$}", width = vwidth)))?;
        }
        Ok(())
    }

    pub fn draw_status_line(&mut self) -> anyhow::Result<()> {
        let left_separator = "";
        let right_separator = "";
        let mode_str = format!(" {:?} ", self.mode).to_uppercase();
        let file_str = format!(" {}", self.buffer.file.as_deref().unwrap_or("[No Name]"));
        let position_str = format!(" {}:{} ", self.pos_y + 1, self.pos_x + 1);

        // Calculate file string width dynamically
        let file_str_width = self.size.0
            - mode_str.len() as u16
            - position_str.len() as u16
            - left_separator.len() as u16
            - right_separator.len() as u16;

        self.stdout.queue(cursor::MoveTo(0, self.size.1 - 2))?;

        // Editor mode
        self.stdout.queue(style::PrintStyledContent(
            mode_str
                .with(Color::Rgb {
                    r: 26,
                    g: 27,
                    b: 38,
                })
                .bold()
                .on(Color::Rgb {
                    r: 187,
                    g: 154,
                    b: 247,
                }),
        ))?;

        // Section separator
        self.stdout.queue(style::PrintStyledContent(
            left_separator
                .with(Color::Rgb {
                    r: 187,
                    g: 154,
                    b: 247,
                })
                .on(Color::Rgb {
                    r: 65,
                    g: 72,
                    b: 104,
                }),
        ))?;

        // File name
        self.stdout.queue(style::PrintStyledContent(
            format!("{:<width$}", file_str, width = file_str_width as usize)
                .with(Color::Rgb {
                    r: 192,
                    g: 202,
                    b: 245,
                })
                .bold()
                .on(Color::Rgb {
                    r: 65,
                    g: 72,
                    b: 104,
                }),
        ))?;

        // Section separator
        self.stdout.queue(style::PrintStyledContent(
            right_separator
                .with(Color::Rgb {
                    r: 187,
                    g: 154,
                    b: 247,
                })
                .on(Color::Rgb {
                    r: 65,
                    g: 72,
                    b: 104,
                }),
        ))?;

        // Cursor position
        self.stdout.queue(style::PrintStyledContent(
            position_str
                .with(Color::Rgb {
                    r: 26,
                    g: 27,
                    b: 38,
                })
                .bold()
                .on(Color::Rgb {
                    r: 187,
                    g: 154,
                    b: 247,
                }),
        ))?;

        Ok(())
    }

    fn check_bounds(&mut self) {
        let line_len = self.line_length();

        if self.pos_x >= line_len {
            if line_len > 0 {
                self.pos_x = self.line_length() - 1;
            } else {
                self.pos_x = 0;
            }
        }

        if self.pos_x >= self.vwidth() {
            self.pos_x = self.vwidth() - 1;
        }

        let line_in_buffer = self.pos_y + self.vtop;
        if line_in_buffer as usize > self.buffer.len() - 1 {
            self.pos_y = self.buffer.len() as u16 - self.vtop - 1;
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            self.check_bounds();
            self.draw()?;
            if let Some(action) = self.handle_event(read()?)? {
                match action {
                    Action::Quit => break,
                    Action::Save => break,
                    Action::MoveUp => {
                        if self.pos_y == 0 {
                            if self.vtop > 0 {
                                self.vtop -= 1;
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
                    Action::MoveToLineStart => {
                        self.pos_x = 0;
                    }
                    Action::MoveToLineEnd => {
                        self.pos_x = self.line_length().saturating_sub(1);
                    }
                    Action::EnterMode(mode) => {
                        self.mode = mode;
                    }
                    Action::InsertCharAtCursor(c) => {
                        self.buffer.insert(self.pos_x, self.buffer_line(), c);
                        self.pos_x += 1;
                    }
                    Action::DeleteCharAtCursor => {
                        self.buffer.remove(self.pos_x, self.buffer_line());
                    }
                    Action::NewLine => {
                        self.pos_x = 0;
                        self.pos_y += 1;
                    }
                    Action::DeleteCurrentLine => {
                        self.buffer.remove_line(self.buffer_line());
                    }
                    Action::SetComboCommand(cmd) => {
                        self.combo_command = Some(cmd);
                    }
                }
            }
        }
        self.stdout.execute(terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;

        Ok(())
    }

    fn handle_event(&mut self, ev: event::Event) -> anyhow::Result<Option<Action>> {
        if let event::Event::Resize(width, height) = ev {
            self.size = (width, height);
            return Ok(None);
        }

        match self.mode {
            Mode::Normal => self.handle_normal_event(ev),
            Mode::Insert => self.handle_insert_event(ev),
            Mode::Command => self.handle_command_event(ev),
            Mode::Visual => self.handle_visual_event(ev),
            Mode::Replace => self.handle_replace_event(ev),
        }
    }

    fn handle_normal_event(&mut self, ev: event::Event) -> anyhow::Result<Option<Action>> {
        log!("Event: {:?}", ev);

        if let Some(cmd) = self.combo_command {
            self.combo_command = None;
            return Ok(self.handle_combo_command(cmd, ev));
        }

        match ev {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Up | event::KeyCode::Char('k') => Ok(Some(Action::MoveUp)),
                event::KeyCode::Down | event::KeyCode::Char('j') => Ok(Some(Action::MoveDown)),
                event::KeyCode::Left | event::KeyCode::Char('h') => Ok(Some(Action::MoveLeft)),
                event::KeyCode::Right | event::KeyCode::Char('l') => Ok(Some(Action::MoveRight)),
                event::KeyCode::Home => Ok(Some(Action::MoveToLineStart)),
                event::KeyCode::End => Ok(Some(Action::MoveToLineEnd)),
                event::KeyCode::Delete => Ok(Some(Action::DeleteCharAtCursor)),
                event::KeyCode::Char('d') => Ok(Some(Action::SetComboCommand('d'))),
                event::KeyCode::Char('i') => Ok(Some(Action::EnterMode(Mode::Insert))),
                event::KeyCode::Char('v') => Ok(Some(Action::EnterMode(Mode::Visual))),
                event::KeyCode::Char(':') => Ok(Some(Action::EnterMode(Mode::Command))),
                event::KeyCode::Char('r') => Ok(Some(Action::EnterMode(Mode::Replace))),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    fn handle_combo_command(&self, cmd: char, ev: event::Event) -> Option<Action> {
        match cmd {
            'd' => match ev {
                event::Event::Key(event) => match event.code {
                    event::KeyCode::Char('d') => Some(Action::DeleteCurrentLine),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    fn handle_insert_event(&mut self, ev: event::Event) -> anyhow::Result<Option<Action>> {
        match ev {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Esc => Ok(Some(Action::EnterMode(Mode::Normal))),
                event::KeyCode::Char(c) => Ok(Some(Action::InsertCharAtCursor(c))),
                event::KeyCode::Enter => Ok(Some(Action::NewLine)),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    fn handle_command_event(&mut self, ev: event::Event) -> anyhow::Result<Option<Action>> {
        match ev {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Esc => Ok(Some(Action::EnterMode(Mode::Normal))),
                event::KeyCode::Char('q') => Ok(Some(Action::Quit)),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    fn handle_visual_event(&mut self, ev: event::Event) -> anyhow::Result<Option<Action>> {
        match ev {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Esc => Ok(Some(Action::EnterMode(Mode::Normal))),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    fn handle_replace_event(&mut self, ev: event::Event) -> anyhow::Result<Option<Action>> {
        match ev {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Esc => Ok(Some(Action::EnterMode(Mode::Normal))),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    pub fn cleanup(&mut self) -> anyhow::Result<()> {
        self.stdout.execute(terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;

        Ok(())
    }
}
