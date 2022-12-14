use super::constants::LOGGER_THREAD_NAME;
use super::logger_error::LoggerError;
use super::logger_sender::LoggerSender;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::{io, thread};

use std::fs;
use std::fs::File;
use std::io::Write;

use chrono::prelude::*;
/// A logger to log into a file
///
/// The logger works with channels. It has one channel to receive the information
/// and as many channels to send it. It can be used with multiple threads at the same time.
///
/// To clone the sender's channel it has a new_sender() method which returns a LoggerSender struct.
#[derive(Debug)]
pub struct Logger {
    sender: LoggerSender,
}

impl Logger {
    /// Constructs a new Logger to log
    ///
    /// In case of success it returns a Logger struct and creates a new log file at the directory path.
    ///
    /// It returns an LoggerError if:
    /// - There was a problem creating the logging directory.
    /// - A new file could not be created at the logging directory.
    /// - There was a problem creating a new thread for the logger receiver.
    pub fn new(dir_path: &str, max_log_file_size: u32) -> Result<Self, LoggerError> {
        let (sender, receiver): (Sender<String>, Receiver<String>) = channel();

        Self::create_log_directory(dir_path)?;
        let file = Self::create_log_file(dir_path)?;
        Self::spawn_log_receiver(receiver, file, max_log_file_size)?;

        Ok(Self {
            sender: LoggerSender::new(sender),
        })
    }

    /// Creates a new LoggerSender for the current Logger
    pub fn new_sender(&self) -> LoggerSender {
        self.sender.clone()
    }

    fn create_log_directory(dir_path: &str) -> Result<(), LoggerError> {
        match fs::create_dir_all(dir_path) {
            Ok(_) => Ok(()),
            Err(error) => {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(LoggerError::LogDirectoryError(format!("{}", error)))
                }
            }
        }
    }

    fn spawn_log_receiver(
        receiver: Receiver<String>,
        file: File,
        max_file_size: u32,
    ) -> Result<(), LoggerError> {
        let builder = thread::Builder::new().name(LOGGER_THREAD_NAME.to_string());
        let result = builder.spawn(move || {
            let mut file = file;

            while let Ok(msg) = receiver.recv() {
                match file.write_all(msg.as_bytes()) {
                    Ok(_) => {}
                    Err(err) => eprintln!("Error({err}) writing to the log"),
                }
                match file.metadata() {
                    Ok(metadata) => {
                        if metadata.len() > max_file_size as u64 {
                            let err_msg = format!(
                                "Max log file size of {}kb has been reached. Closing logger receiver.",
                                max_file_size
                            );
                            eprintln!("{}", err_msg);
                            match file.write_all(msg.as_bytes()) {
                                Ok(_) => {}
                                Err(err) => eprintln!("Error({err}) writing to the log"),
                            }
                            break;
                        }
                    }
                    Err(err) => eprintln!("Error({err}) writing to the log"),
                }
            }
            eprintln!("Logger receiver cloced");
        });
        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(LoggerError::SpawnThreadError),
        }
    }

    fn create_log_file(dir_path: &str) -> Result<File, LoggerError> {
        let time = Local::now();

        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(format!(
                "{}/{}.log",
                dir_path,
                time.format("%Y-%m-%d_%H-%M-%S")
            ));

        match file {
            Ok(file) => Ok(file),
            Err(_) => Err(LoggerError::LogFileError(dir_path.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader};
    use std::thread::sleep;
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_logging_to_existing_directory() {
        let path = "./test_good_log";
        fs::create_dir(path).unwrap();
        let logging = "log_test".to_string();
        let log_type = "info".to_string();
        assert_logging(path, logging, log_type);
    }

    #[test]
    fn test_logging_to_non_existant_directory() {
        let path = "non_existant_directory";
        let logging = "[INFO]";
        let log_type = "info".to_string();

        assert_logging(path, logging.to_string(), log_type);
    }

    #[test]
    fn test_info_log() {
        let path = "./test_info_log";
        let logging = "[INFO]";
        let log_type = "info".to_string();
        assert_logging(path, logging.to_string(), log_type);
    }

    #[test]
    fn test_warn_log() {
        let path = "./test_warn_log";
        let logging = "[WARN]";
        let log_type = "warn".to_string();
        assert_logging(path, logging.to_string(), log_type);
    }

    #[test]
    fn test_error_log() {
        let path = "./test_error_log";
        let logging = "[ERROR]";
        let log_type = "error".to_string();
        assert_logging(path, logging.to_string(), log_type);
    }

    #[test]
    fn test_multiple_loggin() {
        let path = "./test_multiple_loggin";
        let max_log_file_size = 10000;
        let logging = ["log_test_1", "log_test_2", "log_test_3"];

        let logger = Logger::new(path, max_log_file_size).unwrap();

        let logger_sender_1 = logger.new_sender();
        let logger_sender_2 = logger.new_sender();
        let logger_sender_3 = logger.new_sender();

        thread::spawn(move || logger_sender_1.info(logging[0]));
        sleep(Duration::from_millis(100));
        thread::spawn(move || logger_sender_2.info(logging[1]));
        sleep(Duration::from_millis(100));
        thread::spawn(move || logger_sender_3.info(logging[2]));

        let paths = fs::read_dir(path).unwrap();
        for log_path in paths {
            let log = File::open(log_path.unwrap().path()).unwrap();
            let reader = BufReader::new(log);

            let mut counter = 0;
            for line in reader.lines() {
                let current_line = line.unwrap();

                assert!(current_line.contains(logging[counter]));
                counter += 1;
            }
        }

        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn test_multiple_loggin_same_thread() {
        let path = "./test_multiple_loggin_same_thread";
        let max_log_file_size = 10000;
        let logging = ["log_test_1", "log_test_2", "log_test_3"];

        let logger = Logger::new(path, max_log_file_size).unwrap();

        let logger_sender = logger.new_sender();

        logger_sender.info(logging[0]);
        logger_sender.info(logging[1]);
        logger_sender.info(logging[2]);

        let paths = fs::read_dir(path).unwrap();
        for log_path in paths {
            let log = File::open(log_path.unwrap().path()).unwrap();
            let reader = BufReader::new(log);

            let mut counter = 0;
            for line in reader.lines() {
                let current_line = line.unwrap();

                assert!(current_line.contains(logging[counter]));
                counter += 1;
            }
        }

        fs::remove_dir_all(path).unwrap();
    }

    // Auxiliary functions

    fn assert_logging(path: &str, logging: String, log_type: String) {
        let max_log_file_size = 10000;
        let logger = Logger::new(path, max_log_file_size).unwrap();
        let logger_sender = logger.new_sender();

        let logging_assert = logging.clone();

        thread::spawn(move || match log_type.as_str() {
            "info" => logger_sender.info(logging.as_str()),
            "warn" => logger_sender.warn(logging.as_str()),
            "error" => logger_sender.error(logging.as_str()),
            _ => panic!("Unknown log type"),
        });

        let paths = fs::read_dir(path).unwrap();
        for log_path in paths {
            let log = File::open(log_path.unwrap().path()).unwrap();
            let reader = BufReader::new(log);

            for line in reader.lines() {
                let current_line = line.unwrap();

                assert!(current_line.contains(logging_assert.as_str()));
            }
        }

        fs::remove_dir_all(path).unwrap();
    }
}
