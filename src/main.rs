use clap::Parser;
use crossterm::{ExecutableCommand, terminal};
use std::{env, io::stdout, panic};

use tsu_editor::buffer::Buffer;
use tsu_editor::config::Config;
use tsu_editor::editor::Editor;
use tsu_editor::theme::parse_vscode_theme;
use tsu_editor::{LOGGER, Logger, VERSION_AND_GIT_HASH};

#[derive(Parser, Debug)]
#[clap(name = "tsu")]
#[command(
    version = VERSION_AND_GIT_HASH,
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
    #[arg()]
    files: Vec<String>,
}

#[tokio::main(flavor = "multi_thread")]
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

    if let Some(log_file) = &config.log_file {
        LOGGER.get_or_init(|| Some(Logger::new(log_file)));
    } else {
        LOGGER.get_or_init(|| None);
    }

    let mut buffers = Vec::new();
    if args.files.is_empty() {
        let buffer = Buffer::new(None, String::new());
        buffers.push(buffer);
    } else {
        for file in args.files {
            let buffer = Buffer::from_file(Some(file)).await?;
            buffers.push(buffer);
        }
    }

    let theme_file = &Config::path("themes").join(&config.theme);
    if !theme_file.exists() {
        eprintln!("Theme file {} not found", config.theme);
        std::process::exit(1);
    }
    let theme = parse_vscode_theme(&theme_file.to_string_lossy())?;

    let mut editor = Editor::new(config, theme, buffers)?;

    panic::set_hook(Box::new(|info| {
        _ = stdout().execute(terminal::LeaveAlternateScreen);
        _ = terminal::disable_raw_mode();

        eprintln!("{info}");
    }));

    let result = editor.run().await;

    editor.cleanup()?;
    result?;

    Ok(())
}
