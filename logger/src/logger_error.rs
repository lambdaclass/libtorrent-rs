#[derive(Debug)]

/// Logger posible errors
pub enum LoggerError {
    SpawnThreadError,
    SendError(String),
    LogDirectoryError(String),
    LogFileError(String),
}
