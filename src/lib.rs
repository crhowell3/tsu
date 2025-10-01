pub mod action;
pub mod buffer;
pub mod color;
pub mod command;
pub mod config;
pub mod editor;
pub mod highlighter;
pub mod logger;
pub mod theme;
pub mod unicode;

#[doc(hidden)]
pub mod ext;

use once_cell::sync::OnceCell;

pub use logger::Logger;

/// Logger singleton
pub static LOGGER: OnceCell<Option<Logger>> = OnceCell::new();

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        {
            let log_message = format!($($arg)*);
            if let Some(logger) = $crate::LOGGER.get_or_init(|| Some($crate::Logger::new("tsu.log"))) {
                logger.log(&log_message);
            }
        }
    };
}
