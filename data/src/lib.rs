#![allow(clippy::large_enum_variant, clippy::too_many_arguments)]
pub use self::appearance::Theme;
pub use self::command::Command;
pub use self::config::Config;
pub use self::editor::Editor;
pub use self::pane::Pane;
pub use self::shortcut::Shortcut;
pub use self::window::Window;

pub mod appearance;
pub mod command;
mod compression;
pub mod config;
pub mod editor;
pub mod environment;
pub mod pane;
pub mod shortcut;
pub mod window;
