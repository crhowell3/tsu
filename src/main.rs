use crate::editor::Editor;

use clap::Parser;
use std::env;

mod editor;

#[derive(Parser, Debug)]
#[clap(name = "tsu")]
#[command(
    version,
    about,
    author = "Cameron Howell <me@crhowell.com>",
    display_name = "tsu",
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
    file: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let user_level = match args.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    let filename = args.file;

    let crate_name = env!("CARGO_CRATE_NAME");
    let filter = tracing_subscriber::EnvFilter::new(format!("{crate_name}={user_level}"));

    tracing_subscriber::fmt::fmt()
        .with_env_filter(filter)
        .init();

    let mut editor = Editor::new()?;

    editor.run()
}
