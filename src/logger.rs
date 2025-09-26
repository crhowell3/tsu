use core::fmt;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    sync::Mutex,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
pub struct Logger {
    file: Mutex<File>,
    log_level: LogLevel,
}

impl Logger {
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

    pub fn set_level(&mut self, level: LogLevel) {
        self.log_level = level;
    }

    pub fn log(&self, message: &str) {
        let mut file = self.file.lock().unwrap();
        writeln!(file, "{}", message).expect("write to file works");
    }

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

    pub fn debug(&self, message: &str) {
        self.log_with_level(LogLevel::Debug, message);
    }

    pub fn info(&self, message: &str) {
        self.log_with_level(LogLevel::Info, message);
    }

    pub fn warn(&self, message: &str) {
        self.log_with_level(LogLevel::Warn, message);
    }

    pub fn error(&self, message: &str) {
        self.log_with_level(LogLevel::Error, message);
    }
}
