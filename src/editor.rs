use std::io::Write;

use crossterm::{
    ExecutableCommand, QueueableCommand, cursor,
    event::{self, read},
    style, terminal,
};

#[derive(Debug, PartialEq, Eq)]
enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    EnterMode(Mode),
    AddChar(char),
    NewLine,
    Quit,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
}

#[derive(Debug)]
pub struct Editor {
    pub size: (u16, u16),
    pub stdout: std::io::Stdout,
    pub pos_x: u16,
    pub pos_y: u16,
    pub mode: Mode,
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
        self.stdout.queue(style::Print("status line"))?;

        Ok(())
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            self.draw()?;
            if let Some(action) = self.handle_event(read()?)? {
                match action {
                    Action::Quit => break,
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
        }
    }

    fn handle_normal_event(&mut self, ev: event::Event) -> anyhow::Result<Option<Action>> {
        match ev {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Char('q') => Ok(Some(Action::Quit)),
                event::KeyCode::Up | event::KeyCode::Char('k') => Ok(Some(Action::MoveUp)),
                event::KeyCode::Down | event::KeyCode::Char('j') => Ok(Some(Action::MoveDown)),
                event::KeyCode::Left | event::KeyCode::Char('h') => Ok(Some(Action::MoveLeft)),
                event::KeyCode::Right | event::KeyCode::Char('l') => Ok(Some(Action::MoveRight)),
                event::KeyCode::Char('i') => Ok(Some(Action::EnterMode(Mode::Insert))),
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
}
