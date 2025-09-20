use crate::editor::Editor;
use crate::{buffer::Buffer, logger::Logger};

use clap::Parser;
use crossterm::{ExecutableCommand, terminal};
use once_cell::sync::OnceCell;
use std::{env, io::stdout, panic};

mod buffer;
mod editor;
mod logger;
mod theme;

static LOGGER: OnceCell<Logger> = OnceCell::new();

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        {
            let log_message = format!($($arg)*);
            $crate::LOGGER.get_or_init(|| $crate::Logger::new("tsu.log")).log(&log_message);
        }
    };
}

#[derive(Parser, Debug)]
#[clap(name = "tsu")]
#[command(
    version,
    about = "A vimlike modal text editor.",
    author = "Cameron Howell <me@crhowell.com>",
    display_name = "tsu ツ",
    help_template = "{name} {version}
author: {author-with-newline}{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
"
)]
struct Args {
    /// Log level
    #[arg(short, action = clap::ArgAction::Count, help="Increases logging verbosity (-v, -vv, -vvv)")]
    verbose: u8,
    /// File to open
    #[arg(default_value = "")]
    file: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let user_level = match args.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    let crate_name = env!("CARGO_CRATE_NAME");
    let filter = tracing_subscriber::EnvFilter::new(format!("{crate_name}={user_level}"));

    tracing_subscriber::fmt::fmt()
        .with_env_filter(filter)
        .init();

    let filename = args.file;
    let buffer = Buffer::from_file(filename);
    let mut editor = Editor::new(buffer)?;

    panic::set_hook(Box::new(|info| {
        _ = stdout().execute(terminal::LeaveAlternateScreen);
        _ = terminal::disable_raw_mode();

        eprintln!("{}", info);
    }));

    editor.run()?;
    editor.cleanup()
}
