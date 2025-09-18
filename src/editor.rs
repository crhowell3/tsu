use std::io::Write;

use crossterm::{
    ExecutableCommand, QueueableCommand, cursor,
    event::{self, read},
    style, terminal,
};

#[derive(Debug, PartialEq, Eq)]
enum Action {
    // Movement
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,

    // Mode changes
    EnterMode(Mode),
    AddChar(char),
    NewLine,

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
    size: (u16, u16),
    stdout: std::io::Stdout,
    pos_x: u16,
    pos_y: u16,
    mode: Mode,
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = self.stdout.flush();

        let _ = self.stdout.execute(terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

impl Editor {
    pub fn new() -> anyhow::Result<Self> {
        let mut stdout = std::io::stdout();
        terminal::enable_raw_mode()?;
        stdout
            .execute(terminal::EnterAlternateScreen)?
            .execute(terminal::Clear(terminal::ClearType::All))?;
        Ok(Self {
            stdout,
            pos_x: 0,
            pos_y: 0,
            mode: Mode::Normal,
            size: terminal::size()?,
        })
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        self.draw_status_line()?;
        self.stdout.queue(cursor::MoveTo(self.pos_x, self.pos_y))?;
        self.stdout.flush()?;

        Ok(())
    }

    pub fn draw_status_line(&mut self) -> anyhow::Result<()> {
        self.stdout.queue(cursor::MoveTo(0, self.size.1 - 2))?;
        self.stdout
            .queue(style::Print(format!(" {:?} ", self.mode)))?;

        Ok(())
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            self.draw()?;
            if let Some(action) = self.handle_event(read()?)? {
                match action {
                    Action::Quit => break,
                    Action::Save => break,
                    Action::MoveUp => {
                        self.pos_y = self.pos_y.saturating_sub(1);
                    }
                    Action::MoveDown => {
                        self.pos_y += 1u16;
                    }
                    Action::MoveLeft => {
                        self.pos_x = self.pos_x.saturating_sub(1);
                    }
                    Action::MoveRight => {
                        self.pos_x += 1u16;
                    }
                    Action::EnterMode(mode) => {
                        self.mode = mode;
                    }
                    Action::AddChar(c) => {
                        self.stdout.queue(cursor::MoveTo(self.pos_x, self.pos_y))?;
                        self.stdout.queue(style::Print(c))?;
                        self.pos_x += 1;
                    }
                    Action::NewLine => {
                        self.pos_x = 0;
                        self.pos_y += 1;
                    }
                }
            }
        }
        self.stdout.execute(terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;

        Ok(())
    }

    fn handle_event(&mut self, ev: event::Event) -> anyhow::Result<Option<Action>> {
        match self.mode {
            Mode::Normal => self.handle_normal_event(ev),
            Mode::Insert => self.handle_insert_event(ev),
            Mode::Command => self.handle_command_event(ev),
            Mode::Visual => self.handle_visual_event(ev),
            Mode::Replace => self.handle_replace_event(ev),
        }
    }

    fn handle_normal_event(&mut self, ev: event::Event) -> anyhow::Result<Option<Action>> {
        match ev {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Up | event::KeyCode::Char('k') => Ok(Some(Action::MoveUp)),
                event::KeyCode::Down | event::KeyCode::Char('j') => Ok(Some(Action::MoveDown)),
                event::KeyCode::Left | event::KeyCode::Char('h') => Ok(Some(Action::MoveLeft)),
                event::KeyCode::Right | event::KeyCode::Char('l') => Ok(Some(Action::MoveRight)),
                event::KeyCode::Char('i') => Ok(Some(Action::EnterMode(Mode::Insert))),
                event::KeyCode::Char('v') => Ok(Some(Action::EnterMode(Mode::Visual))),
                event::KeyCode::Char(':') => Ok(Some(Action::EnterMode(Mode::Command))),
                event::KeyCode::Char('r') => Ok(Some(Action::EnterMode(Mode::Replace))),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    fn handle_insert_event(&mut self, ev: event::Event) -> anyhow::Result<Option<Action>> {
        match ev {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Esc => Ok(Some(Action::EnterMode(Mode::Normal))),
                event::KeyCode::Char(c) => Ok(Some(Action::AddChar(c))),
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
}
