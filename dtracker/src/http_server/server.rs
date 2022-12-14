use std::{net::TcpListener, sync::Arc};

use logger::logger_sender::LoggerSender;

use crate::http_server::request_handler::RequestHandler;
use crate::stats::stats_updater::StatsUpdater;
use crate::{
    http_server::thread_pool::pool::ThreadPool,
    tracker_status::atomic_tracker_status::AtomicTrackerStatus,
};

/// Struct that represents the HTTP Server that will listen to connections to the Tracker.
///
/// ## Fields
/// * `listener`: The TCP server binded to the socket, responsible of listening for connections.
/// * `pool`: A thread pool that provides worker threads, in order to favor parallel execution.
/// * `status`: Current status of the tracker.
/// * `logger_sender`: To log using the Logger.
pub struct Server {
    listener: TcpListener,
    pool: ThreadPool,
    status: Arc<AtomicTrackerStatus>,
    stats_updater: Arc<StatsUpdater>,
    logger_sender: LoggerSender,
    port: u16,
}

impl Server {
    /// Creates a new `Server`.
    pub fn init(
        status: Arc<AtomicTrackerStatus>,
        stats_updater: Arc<StatsUpdater>,
        logger_sender: LoggerSender,
        port: u16,
    ) -> std::io::Result<Server> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        Ok(Server {
            listener,
            pool: ThreadPool::new(1000, logger_sender.clone()),
            status,
            logger_sender,
            stats_updater,
            port,
        })
    }

    /// Handles new connections to the server
    pub fn serve(&self) -> std::io::Result<()> {
        let started_msg = format!("Serving on http://0.0.0.0:{}", self.port);
        self.logger_sender.info(&started_msg);
        println!("{}", started_msg);

        for stream in self.listener.incoming() {
            let stream = stream?;
            let mut request_handler = RequestHandler::new(stream);
            let logger = self.logger_sender.clone();
            let status_clone = self.status.clone();
            let stats_updater = self.stats_updater.clone();
            let _ = self.pool.execute(move || {
                if let Err(error) = request_handler.handle(status_clone, stats_updater) {
                    logger.error(&format!(
                        "An error occurred while attempting to handle a request: {:?}",
                        error
                    ));
                }
            });
        }
        Ok(())
    }
}
