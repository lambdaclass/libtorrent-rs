use std::sync::Arc;
use std::{io, thread::spawn};

use chrono::Duration;
use logger::{logger_error::LoggerError, logger_receiver::Logger, logger_sender::LoggerSender};

use crate::{
    http_server::server::Server, stats::stats_updater::StatsUpdater,
    tracker_status::atomic_tracker_status::AtomicTrackerStatus,
};

/// Struct that represents the Tracker itself.
///
/// Serves as a starting point for the application.
pub struct BtTracker {
    server: Server,
}

#[derive(Debug)]
pub enum BtTrackerError {
    LoggerInitError(LoggerError),
    CreatingServerError(io::Error),
    StartingServerError(io::Error),
}

const STATS_UPDATER_MINUTES_TIMEOUT: i64 = 1;

impl BtTracker {
    /// Creates a new BtTracker
    pub fn init(port: u16) -> Result<Self, BtTrackerError> {
        let logger = Logger::new("./logs", 1000000).map_err(BtTrackerError::LoggerInitError)?; // TODO: Sacar de configs
        let logger_sender = logger.new_sender();

        let tracker_status = Arc::new(AtomicTrackerStatus::default());

        let stats_updater =
            Self::spawn_stats_updater(tracker_status.clone(), logger_sender.clone());

        let server = Server::init(tracker_status, stats_updater, logger_sender.clone(), port)
            .map_err(BtTrackerError::CreatingServerError)?;

        logger_sender.info("Tracker started");

        Ok(Self { server })
    }

    /// Starts the server for handling requests.
    pub fn run(&self) -> Result<(), BtTrackerError> {
        self.server
            .serve()
            .map_err(BtTrackerError::StartingServerError)
    }

    fn spawn_stats_updater(
        tracker_status: Arc<AtomicTrackerStatus>,
        logger_sender: LoggerSender,
    ) -> Arc<StatsUpdater> {
        let stats_updater = Arc::new(StatsUpdater::new(
            tracker_status,
            Duration::minutes(STATS_UPDATER_MINUTES_TIMEOUT),
            logger_sender,
        ));
        let updater = stats_updater.clone();
        spawn(move || {
            updater.run();
        });
        stats_updater
    }
}
