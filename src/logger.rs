use core::fmt;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    sync::Mutex,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Log severity levels
///
/// A coarser representation of the standard software log severity levels
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

impl LogLevel {
    /// Converts a string into a corresponding log severity level
    ///
    /// # Arguments
    /// - `s`: A string that may represent a log level
    ///
    /// # Returns
    /// - A log level enum variant if the string is able to be parsed, otherwise `None`
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "DEBUG" => Some(LogLevel::Debug),
            "INFO" => Some(LogLevel::Info),
            "WARN" => Some(LogLevel::Warn),
            "ERROR" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

/// The logger entity
///
/// Filters logs based on a provided severity level and writes those logs to a file
pub struct Logger {
    file: Mutex<File>,
    log_level: LogLevel,
}

impl Logger {
    /// Create a new logger with a given log file path
    ///
    /// # Arguments
    /// - `file`: A path to a file to which logs will be written
    ///
    pub fn new(file: &str) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)
            .expect("log file successfully opens");

        Logger {
            file: Mutex::new(file),
            log_level: LogLevel::Debug,
        }
    }

    /// Set the logger's log level
    ///
    /// This log level indicates the minimum log level allowed to be written to the log file
    ///
    /// # Arguments
    /// - `level`: The minimum log level for filtering
    ///
    pub fn set_level(&mut self, level: LogLevel) {
        self.log_level = level;
    }

    /// Log a message without a specified log level
    ///
    /// # Arguments
    /// - `message`: A message string to log to the log file
    ///
    pub fn log(&self, message: &str) {
        let mut file = self.file.lock().unwrap();
        writeln!(file, "{}", message).expect("write to file works");
    }

    /// Log a message with a specified log level
    ///
    /// # Arguments
    /// - `level`: The log level of the message
    /// - `message`: The message to log to the log file
    ///
    pub fn log_with_level(&self, level: LogLevel, message: &str) {
        if level < self.log_level {
            return;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let formatted = format!("[{}] [{}] {}", timestamp, level, message);

        let mut file = self.file.lock().unwrap();
        writeln!(file, "{}", formatted).expect("write to file works");
    }

    /// Write a debug log
    ///
    /// # Arguments
    /// - `message`: The message to log to the log file
    ///
    pub fn debug(&self, message: &str) {
        self.log_with_level(LogLevel::Debug, message);
    }

    /// Write an info log
    ///
    /// # Arguments
    /// - `message`: The message to log to the log file
    ///
    pub fn info(&self, message: &str) {
        self.log_with_level(LogLevel::Info, message);
    }

    /// Write a warning log
    ///
    /// # Arguments
    /// - `message`: The message to log to the log file
    ///
    pub fn warn(&self, message: &str) {
        self.log_with_level(LogLevel::Warn, message);
    }

    /// Write an error log
    ///
    /// # Arguments
    /// - `message`: The message to log to the log file
    ///
    pub fn error(&self, message: &str) {
        self.log_with_level(LogLevel::Error, message);
    }
}
