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
    // Buffer actions
    Quit,
    Undo,
    CenterView,

    // Movement
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    MoveTop,
    MoveBottom,
    MoveToLineEnd,
    MoveToLineStart,
    PageUp,
    PageDown,

    // Text editing
    InsertCharAtCursor(char),
    RemoveCharAt(u16, usize),
    InsertLineAbove,
    InsertLineBelow,
    InsertLineAt(usize, Option<String>),
    NewLine,
    DeleteCharAtCursor,
    DeleteCurrentLine,

    // Misc
    EnterMode(Mode),
    SetComboCommand(char),
}

impl Action {
    pub fn execute(self, editor: &mut Editor) {
        match self {
            Action::Quit => {}
            Action::Undo => {
                if let Some(undoable_action) = editor.undoable_actions.pop() {
                    undoable_action.execute(editor);
                }
            }
            Action::CenterView => {
                let view_center = editor.vheight() / 2;
                let distance_to_center = editor.pos_y as isize - view_center as isize;

                if distance_to_center > 0 {
                    let distance_to_center = distance_to_center.unsigned_abs();
                    if editor.vtop > distance_to_center {
                        editor.vtop += distance_to_center;
                        editor.pos_y = view_center;
                    }
                } else if distance_to_center < 0 {
                    let distance_to_center = distance_to_center.unsigned_abs();
                    let new_vtop = editor.vtop.saturating_sub(distance_to_center);
                    if editor.buffer.len() > editor.vtop + distance_to_center
                        && new_vtop != editor.vtop
                    {
                        editor.vtop = new_vtop;
                        editor.pos_y = view_center;
                    }
                }
            }
            Action::MoveUp => {
                if editor.pos_y == 0 {
                    if editor.vtop > 0 {
                        editor.vtop -= 1;
                    }
                } else {
                    editor.pos_y = editor.pos_y.saturating_sub(1);
                }
            }
            Action::MoveDown => {
                editor.pos_y += 1;
                if editor.pos_y >= editor.vheight() {
                    editor.vtop += 1;
                    editor.pos_y -= 1;
                }
            }
            Action::MoveLeft => {
                _ = editor.pos_x.saturating_sub(1);
                if editor.pos_x < editor.vleft {
                    editor.pos_x = editor.vleft;
                }
            }
            Action::MoveRight => {
                editor.pos_x += 1;
            }
            Action::MoveTop => {
                editor.vtop = 0;
                editor.pos_y = 0;
            }
            Action::MoveBottom => {
                if editor.buffer.len() > editor.vheight() as usize {
                    editor.pos_y = editor.vheight() - 1;
                    editor.vtop = editor.buffer.len() - editor.vheight() as usize;
                } else {
                    editor.pos_y = editor.buffer.len() as u16 - 1u16;
                }
            }
            Action::MoveToLineStart => {
                editor.pos_x = 0;
            }
            Action::MoveToLineEnd => {
                editor.pos_x = editor.line_length().saturating_sub(1);
            }
            Action::PageUp => {
                if editor.vtop > 0 {
                    editor.vtop = editor.vtop.saturating_sub(editor.vheight() as usize);
                }
            }
            Action::PageDown => {
                if editor.buffer.len() > (editor.vtop + editor.vheight() as usize) {
                    editor.vtop += editor.vheight() as usize;
                }
            }
            Action::EnterMode(mode) => {
                editor.mode = mode;
            }
            Action::InsertCharAtCursor(c) => {
                editor
                    .undoable_actions
                    .push(Action::RemoveCharAt(editor.pos_x, editor.buffer_line()));
                editor.buffer.insert(editor.pos_x, editor.buffer_line(), c);
                editor.pos_x += 1;
            }
            Action::RemoveCharAt(x, y) => {
                editor.buffer.remove(x, y);
            }
            Action::DeleteCharAtCursor => {
                editor.buffer.remove(editor.pos_x, editor.buffer_line());
            }
            Action::NewLine => {
                editor.pos_x = 0;
                editor.pos_y += 1;
            }
            Action::InsertLineAbove => {
                editor
                    .buffer
                    .insert_line(editor.buffer_line(), String::new());
                editor.pos_y = editor.pos_y.saturating_sub(1);
                editor.mode = Mode::Insert;
            }
            Action::InsertLineBelow => {
                editor
                    .buffer
                    .insert_line(editor.buffer_line() + 1, String::new());
                editor.pos_y += 1;
                editor.pos_x = 0;
                editor.mode = Mode::Insert;
            }
            Action::InsertLineAt(line, contents) => {
                if let Some(contents) = contents {
                    editor.buffer.insert_line(line, contents);
                }
            }
            Action::DeleteCurrentLine => {
                let line = editor.buffer_line();
                let contents = editor.current_line_contents();

                editor.buffer.remove_line(editor.buffer_line());
                editor
                    .undoable_actions
                    .push(Action::InsertLineAt(line, contents));
            }
            Action::SetComboCommand(cmd) => {
                editor.combo_command = Some(cmd);
            }
        }
    }
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
    vtop: usize,
    vleft: u16,
    pos_x: u16,
    pos_y: u16,
    mode: Mode,
    combo_command: Option<char>,
    undoable_actions: Vec<Action>,
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
            undoable_actions: vec![],
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

    fn buffer_line(&self) -> usize {
        self.vtop + self.pos_y as usize
    }

    fn view_line(&self, n: u16) -> Option<String> {
        let line = self.vtop + n as usize;
        self.buffer.get(line)
    }

    fn set_cursor_style(&mut self) -> anyhow::Result<()> {
        self.stdout.queue(match self.combo_command {
            Some(_) => cursor::SetCursorStyle::SteadyUnderScore,
            _ => cursor::SetCursorStyle::DefaultUserShape,
        })?;

        Ok(())
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        self.stdout.queue(cursor::Hide)?;
        self.draw_view()?;
        self.draw_status_line()?;
        self.stdout.queue(cursor::MoveTo(self.pos_x, self.pos_y))?;
        self.set_cursor_style()?;
        self.stdout.queue(cursor::Show)?;
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
        let position_str = format!(
            " {}:{} ",
            self.pos_y as usize + self.vtop + 1,
            self.pos_x + self.vleft + 1
        );

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

    fn is_insert(&self) -> bool {
        matches!(self.mode, Mode::Insert)
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

        let line_in_buffer = self.pos_y as usize + self.vtop;
        if line_in_buffer > self.buffer.len() - 1 {
            self.pos_y = (self.buffer.len() - self.vtop - 1) as u16;
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            self.check_bounds();
            self.draw()?;
            if let Some(action) = self.handle_event(read()?)? {
                if matches!(action, Action::Quit) {
                    break;
                }
                action.execute(self);
            }
        }

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

        let action = match ev {
            event::Event::Key(event) => {
                let code = event.code;
                match code {
                    event::KeyCode::Up | event::KeyCode::Char('k') => Some(Action::MoveUp),
                    event::KeyCode::Down | event::KeyCode::Char('j') => Some(Action::MoveDown),
                    event::KeyCode::Left | event::KeyCode::Char('h') => Some(Action::MoveLeft),
                    event::KeyCode::Right | event::KeyCode::Char('l') => Some(Action::MoveRight),
                    event::KeyCode::Home => Some(Action::MoveToLineStart),
                    event::KeyCode::End => Some(Action::MoveToLineEnd),
                    event::KeyCode::Delete => Some(Action::DeleteCharAtCursor),
                    event::KeyCode::PageUp => Some(Action::PageUp),
                    event::KeyCode::PageDown => Some(Action::PageDown),
                    event::KeyCode::Char('o') => Some(Action::InsertLineBelow),
                    event::KeyCode::Char('O') => Some(Action::InsertLineAbove),
                    event::KeyCode::Char('d') => Some(Action::SetComboCommand('d')),
                    event::KeyCode::Char('g') => Some(Action::SetComboCommand('g')),
                    event::KeyCode::Char('z') => Some(Action::SetComboCommand('z')),
                    event::KeyCode::Char('i') => Some(Action::EnterMode(Mode::Insert)),
                    event::KeyCode::Char('u') => Some(Action::Undo),
                    event::KeyCode::Char('v') => Some(Action::EnterMode(Mode::Visual)),
                    event::KeyCode::Char(':') => Some(Action::EnterMode(Mode::Command)),
                    event::KeyCode::Char('r') => Some(Action::EnterMode(Mode::Replace)),
                    _ => None,
                }
            }
            _ => None,
        };
        Ok(action)
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
            'g' => match ev {
                event::Event::Key(event) => match event.code {
                    event::KeyCode::Char('h') => Some(Action::MoveToLineStart),
                    event::KeyCode::Char('l') => Some(Action::MoveToLineEnd),
                    event::KeyCode::Char('g') => Some(Action::MoveTop),
                    event::KeyCode::Char('e') => Some(Action::MoveBottom),
                    _ => None,
                },
                _ => None,
            },
            'z' => match ev {
                event::Event::Key(event) => match event.code {
                    event::KeyCode::Char('z') => Some(Action::CenterView),
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

    fn current_line_contents(&self) -> Option<String> {
        self.buffer.get(self.buffer_line())
    }
}
