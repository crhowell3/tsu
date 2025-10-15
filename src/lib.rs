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
pub mod window;
pub mod window_manager;

#[doc(hidden)]
pub mod ext;

use once_cell::sync::OnceCell;

pub use logger::Logger;

pub const VERSION_AND_GIT_HASH: &str = env!("VERSION_AND_GIT_HASH");

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

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        {
            let log_message = format!($($arg)*);
            if let Some(logger) = $crate::LOGGER.get_or_init(|| Some($crate::Logger::new("tsu.log"))) {
                logger.debug(&log_message);
            }
        }
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        {
            let log_message = format!($($arg)*);
            if let Some(logger) = $crate::LOGGER.get_or_init(|| Some($crate::Logger::new("tsu.log"))) {
                logger.info(&log_message);
            }
        }
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        {
            let log_message = format!($($arg)*);
            if let Some(logger) = $crate::LOGGER.get_or_init(|| Some($crate::Logger::new("tsu.log"))) {
                logger.warn(&log_message);
            }
        }
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        {
            let log_message = format!($($arg)*);
            if let Some(logger) = $crate::LOGGER.get_or_init(|| Some($crate::Logger::new("tsu.log"))) {
                logger.error(&log_message);
            }
        }
    };
}
