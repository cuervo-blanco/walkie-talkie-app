use chrono::Local;
use lazy_static::lazy_static;
use std::fs::{OpenOptions, File};
use std::sync::Mutex;
use std::io::Write;

lazy_static! {
    static ref LOGGER: Mutex<Logger> = Mutex::new(Logger::new("log.txt").unwrap());
}

pub struct Logger {
    file: File,
}

impl Logger {
    pub fn new(file_path: &str) -> std::io::Result<Logger> {
        // Check if the log file exists and reset it or create a new one
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(file_path)?;

        // Empty the log file if it exists
        file.set_len(0)?;

        // Log the creation time
        writeln!(file, "Log file created at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))?;

        Ok(Logger { file })
    }

    pub fn log_message(&mut self, message: &str) -> std::io::Result<()> {
        let now = Local::now();
        writeln!(self.file, "{} - {}", now.format("%Y-%m-%d %H:%M:%S"), message)?;
        Ok(())
    }
}

pub fn log_message(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log_message(message).expect("Failed to log message");
    } else {
        eprintln!("Failed to acquire logger lock");
    }
}
