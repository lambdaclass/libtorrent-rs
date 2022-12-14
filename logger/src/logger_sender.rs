use std::sync::mpsc::Sender;
use std::thread;

use chrono::Local;

/// A LoggerSender representing the sender channel connected to a Logger
///
/// There are three ways to write to the log:
///  - `info()` to log information.
///  - `warn()` to log a non critical warning.
///  - `error()` to log a critical error.
///
/// To clone the LoggerSender simply call the `clone()` method.
#[derive(Debug, Clone)]
pub struct LoggerSender {
    sender_clone: Sender<String>,
}

impl LoggerSender {
    /// Creates a new LoggerSender from a clone of an existing sender.
    pub fn new(sender_clone: Sender<String>) -> Self {
        Self { sender_clone }
    }

    /// Writes an Info type log to the connected logger
    ///
    /// It prints an error if:
    /// - Couldn't send the information to the receiver
    pub fn info(&self, value: &str) {
        let formatted_msg = self.format_msg(value, "INFO");
        self.send(formatted_msg)
    }

    /// Writes a Warn type log to the connected logger
    ///
    /// It prints an error if:
    /// - Couldn't send the information to the receiver
    pub fn warn(&self, value: &str) {
        let formatted_msg = self.format_msg(value, "WARN");
        self.send(formatted_msg)
    }

    /// Writes an Error type log to the connected logger
    ///
    /// It prints an error if:
    /// - Couldn't send the information to the receiver
    pub fn error(&self, value: &str) {
        let formatted_msg = self.format_msg(value, "ERROR");
        self.send(formatted_msg)
    }

    fn send(&self, value: String) {
        match self.sender_clone.send(value.to_string()) {
            Ok(_) => (),
            Err(err) => eprintln!(
                "Error({err}) writing: {value} to the log. The logger receiver is probably dead."
            ),
        }
    }

    fn get_thread_name(&self) -> String {
        let current_thread = thread::current();
        match current_thread.name() {
            Some(name) => name.to_string(),
            None => "unnamed-thread".to_string(),
        }
    }

    fn format_msg(&self, value: &str, log_type: &str) -> String {
        let time = Local::now();
        let time_formated = time.format("[%Y/%m/%d %H:%M:%S]");
        return format!(
            "[{}] [{}] [{}] - {}\n",
            time_formated,
            self.get_thread_name(),
            log_type,
            value
        );
    }
}
