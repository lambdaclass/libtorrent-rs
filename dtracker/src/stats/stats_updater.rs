use chrono::Duration;
use std::sync::{Mutex, MutexGuard};
use std::{sync::Arc, thread::sleep};

use logger::logger_sender::LoggerSender;

use crate::tracker_status::atomic_tracker_status::AtomicTrackerStatus;
use crate::tracker_status::current_tracker_stats::CurrentTrackerStats;

// for 1 month it takes 0.5 miliseconds to update the stats. And 0.5 Megabytes to store the stats.
const MAX_DAYS_TO_KEEP_STATS: u64 = 30;

/// Struct that represents the current status of the stats.
#[derive(Debug)]
pub struct StatsUpdater {
    stats_history: Mutex<Vec<CurrentTrackerStats>>,
    duration: chrono::Duration,
    tracker_status: Arc<AtomicTrackerStatus>,
    logger_sender: Mutex<LoggerSender>,
}

impl StatsUpdater {
    /// Creates a new `StatsUpdater`.
    pub fn new(
        tracker_status: Arc<AtomicTrackerStatus>,
        timeout: Duration,
        logger_sender: LoggerSender,
    ) -> Self {
        Self {
            duration: timeout,
            tracker_status,
            stats_history: Mutex::new(Vec::new()),
            logger_sender: Mutex::new(logger_sender),
        }
    }

    /// Starts the loop that updates the stats every `duration` seconds and saves them in the history.
    pub fn run(&self) {
        loop {
            self.tracker_status.remove_inactive_peers();
            let mut stats_history = self.lock_stats_history();

            // If we reached the maximum number of days to keep stats, remove the oldest one.
            let max_secs_to_keep_stats = MAX_DAYS_TO_KEEP_STATS * 24 * 60 * 60;
            if self.duration.num_seconds() * stats_history.len() as i64
                > max_secs_to_keep_stats as i64
            {
                stats_history.rotate_left(1);
                stats_history.pop();
            }

            stats_history.push(self.tracker_status.get_global_statistics());
            let logger = self.lock_logger_sender();
            logger.info("Stats updated");
            let std_duration = match self.duration.to_std() {
                Ok(std_duration) => std_duration,
                Err(_) => {
                    logger.warn("Error converting duration to std::time::Duration");
                    continue;
                }
            };
            // Drop lock before sleeping.
            drop(stats_history);
            sleep(std_duration);
        }
    }

    /// Gets the history of the stats since a given time. If the is less than `since` histories, all the histories are returned.
    ///
    /// ## Returns
    /// * `Vec<CurrentTrackerStats>`: The history of the stats. The total number of torrents, seeders and leechers at a given time.
    pub fn get_history(&self, since: chrono::Duration) -> Vec<CurrentTrackerStats> {
        let stats_history = self.lock_stats_history();
        let since_secs = since.num_seconds();
        let timeout_secs = self.duration.num_seconds();

        let number_of_histories_wanted = since_secs / timeout_secs;

        if number_of_histories_wanted > stats_history.len() as i64 {
            return stats_history.clone();
        }
        stats_history[stats_history.len() - number_of_histories_wanted as usize..].to_vec()
    }

    /// Gets the duration timeout of the stats.
    pub fn get_timeout(&self) -> chrono::Duration {
        self.duration
    }

    fn lock_stats_history(&self) -> MutexGuard<Vec<CurrentTrackerStats>> {
        self.stats_history.lock().unwrap() // unwrap is safe because we are the only one who can modify the stats_history
    }

    fn lock_logger_sender(&self) -> MutexGuard<LoggerSender> {
        self.logger_sender.lock().unwrap() // unwrap is safe because we are the only one who use the logger_sender
    }
}
