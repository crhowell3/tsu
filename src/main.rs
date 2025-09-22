use crate::config::Config;
use crate::editor::Editor;
use crate::theme::parse_vscode_theme;
use crate::{buffer::Buffer, logger::Logger};

use clap::Parser;
use crossterm::{ExecutableCommand, terminal};
use once_cell::sync::OnceCell;
use std::{env, io::stdout, panic};

mod buffer;
mod color;
mod config;
mod editor;
mod highlighter;
mod logger;
mod theme;
mod unicode;

#[allow(dead_code)]
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    let config_file = std::env::home_dir()
        .unwrap()
        .join(".config/tsu/config.toml");
    if !config_file.exists() {
        eprintln!("Config file {} not found", config_file.display());
        std::process::exit(1);
    }

    let toml = std::fs::read_to_string(config_file)?;
    let config: Config = toml::from_str(&toml)?;

    let filename = args.file;
    let buffer = Buffer::from_file(filename);

    let theme_file = &Config::path("themes").join(&config.theme);
    if !theme_file.exists() {
        eprintln!("Theme file {} not found", config.theme);
        std::process::exit(1);
    }
    let theme = parse_vscode_theme(&theme_file.to_string_lossy())?;

    let mut editor = Editor::new(config, theme, buffer)?;

    panic::set_hook(Box::new(|info| {
        _ = stdout().execute(terminal::LeaveAlternateScreen);
        _ = terminal::disable_raw_mode();

        eprintln!("{}", info);
    }));

    let result = editor.run().await;

    editor.cleanup()?;
    result?;

    Ok(())
}
