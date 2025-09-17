use std::io::{self, Write, stdout};

use crossterm::{ExecutableCommand, QueueableCommand, cursor, event, style, terminal};

#[derive(Debug, PartialEq, Eq)]
enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    EnterMode(Mode),
    Quit,
}

#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Normal,
    Insert,
}

fn handle_event(
    mode: &Mode,
    stdout: &mut io::Stdout,
    ev: event::Event,
) -> anyhow::Result<Option<Action>> {
    match mode {
        Mode::Normal => handle_normal_event(ev),
        Mode::Insert => handle_insert_event(stdout, ev),
    }
}

fn handle_normal_event(ev: event::Event) -> anyhow::Result<Option<Action>> {
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

fn handle_insert_event(
    stdout: &mut io::Stdout,
    ev: event::Event,
) -> anyhow::Result<Option<Action>> {
    match ev {
        event::Event::Key(event) => match event.code {
            event::KeyCode::Esc => Ok(Some(Action::EnterMode(Mode::Normal))),
            event::KeyCode::Char(c) => {
                stdout.queue(style::Print(c))?;
                Ok(None)
            }
            _ => Ok(None),
        },
        _ => Ok(None),
    }
}

fn main() -> anyhow::Result<()> {
    let mut stdout = stdout();
    let mut current_mode = Mode::Normal;
    let mut pos_x = 0;
    let mut pos_y = 0;

    terminal::enable_raw_mode()?;
    stdout.execute(terminal::EnterAlternateScreen)?;

    stdout.execute(terminal::Clear(terminal::ClearType::All))?;

    loop {
        stdout.queue(cursor::MoveTo(pos_x, pos_y))?;
        stdout.flush()?;

        if let Some(action) = handle_event(&current_mode, &mut stdout, event::read()?)? {
            match action {
                Action::Quit => break,
                Action::MoveUp => {
                    pos_y = pos_y.saturating_sub(1);
                }
                Action::MoveDown => {
                    pos_y += 1u16;
                }
                Action::MoveLeft => {
                    pos_x = pos_x.saturating_sub(1);
                }
                Action::MoveRight => {
                    pos_x += 1u16;
                }
                Action::EnterMode(mode) => {
                    current_mode = mode;
                }
            }
        }
    }

    stdout.execute(terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    Ok(())
}
