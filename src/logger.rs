use std::{
    fs::{File, OpenOptions},
    io::Write,
    sync::Mutex,
};

pub struct Logger {
    #[allow(dead_code)]
    file: Mutex<File>,
}

impl Logger {
    #[allow(dead_code)]
    pub fn new(file: &str) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)
            .expect("log file successfully opens");

        Logger {
            file: Mutex::new(file),
        }
    }

    #[allow(dead_code)]
    pub fn log(&self, message: &str) {
        let mut file = self.file.lock().unwrap();
        writeln!(file, "{}", message).expect("write to file works");
    }
}
