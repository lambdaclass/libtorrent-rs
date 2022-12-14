use super::status::{AtomicTorrentStatus, AtomicTorrentStatusError};
use crate::{
    config::cfg::Cfg,
    peer::{
        bt_peer::BtPeer,
        peer_session::{PeerSession, PeerSessionError},
    },
    torrent_parser::torrent::Torrent,
    tracker::{
        tracker_handler::{TrackerHandler, TrackerHandlerError},
        tracker_response::TrackerResponse,
    },
};
use logger::logger_sender::LoggerSender;
use std::{
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
    time::Duration,
};

/// Struct for handling the torrent download.
///
/// To create a new `TorrentHandler`, use TorrentHandler::new(torrent, config, logger_sender).
#[derive(Debug)]
pub struct TorrentHandler {
    torrent: Torrent,
    config: Cfg,
    logger_sender: LoggerSender,
    torrent_status: Arc<AtomicTorrentStatus>,
    torrent_status_receiver: Receiver<usize>,
    client_peer_id: String,
}

/// Posible torrent handler errors.
#[derive(Debug)]
pub enum TorrentHandlerError {
    TrackerError(TrackerHandlerError),
    TorrentStatusError(AtomicTorrentStatusError),
    PeerSessionError(PeerSessionError),
    TorrentStatusRecvError(mpsc::RecvError),
}

impl TorrentHandler {
    /// Creates a new `TorrentHandler` from a torrent, a config and a logger sender.
    pub fn new(
        torrent: Torrent,
        config: Cfg,
        logger_sender: LoggerSender,
        client_peer_id: String,
    ) -> Self {
        let (torrent_status, torrent_status_receiver) =
            AtomicTorrentStatus::new(&torrent, config.clone());

        Self {
            torrent_status: Arc::new(torrent_status),
            torrent,
            config,
            logger_sender,
            torrent_status_receiver,
            client_peer_id,
        }
    }

    /// Starts the torrent download.
    ///
    /// First it connects to the tracker and gets the peers. Then it connects to each peer and starts the download.
    ///
    /// # Errors
    ///
    /// - `TrackerErr` if there was a problem connecting to the tracker or getting the peers.
    /// - `TorrentStatusError` if there was a problem using the `Torrent Status`.
    /// - `TorrentStatusRecvError` if there was a problem receiving from the receiver of `Torrent Status`.
    pub fn handle(&mut self) -> Result<(), TorrentHandlerError> {
        let tracker_handler = TrackerHandler::new(
            self.torrent.clone(),
            self.config.tcp_port.into(),
            self.client_peer_id.clone(),
        )
        .map_err(TorrentHandlerError::TrackerError)?;
        self.logger_sender.info("Connected to tracker.");

        while !self.torrent_status.is_finished() {
            let peer_list = self.get_peers_list(&tracker_handler)?;
            self.logger_sender.info("Tracker peer list obtained.");

            // Start connection with each peer
            for peer in peer_list {
                let current_peers = self.torrent_status.all_current_peers();

                // If we reached the maximum number of simultaneous peers, wait until the status tells us that one disconnected.
                if current_peers >= self.config.max_peers_per_torrent as usize {
                    // This while loop is done to prevent creating more peers than allowed when multiple peers are disconnected at the same time.
                    self.torrent_status_receiver
                        .recv()
                        .map_err(TorrentHandlerError::TorrentStatusRecvError)?;
                    while self
                        .torrent_status_receiver
                        .recv_timeout(Duration::from_nanos(1))
                        .is_ok()
                    {
                        continue;
                    }
                }
                if self.torrent_status.is_finished() {
                    break;
                }

                let connected_peers = self
                    .torrent_status
                    .get_connected_peers()
                    .map_err(TorrentHandlerError::TorrentStatusError)?;

                // Avoid connecting to the same peer twice.
                if connected_peers.contains_key(&peer) {
                    continue;
                }

                let current_peers = self.torrent_status.all_current_peers();
                if current_peers < self.config.max_peers_per_torrent as usize {
                    self.connect_to_peer(peer)?;
                }
            }
        }
        self.logger_sender.info("Torrent download finished.");
        Ok(())
    }

    /// Gets the status of the torrent.
    pub fn status(&self) -> Arc<AtomicTorrentStatus> {
        self.torrent_status.clone()
    }

    fn get_peers_list(
        &self,
        tracker_handler: &TrackerHandler,
    ) -> Result<Vec<BtPeer>, TorrentHandlerError> {
        let tracker_response = tracker_handler
            .get_peers_list()
            .map_err(TorrentHandlerError::TrackerError)?;

        self.update_total_peers(&tracker_response);

        Ok(tracker_response.peers)
    }

    /// Updates the torrent status with the number of total peers.
    ///
    /// If the tracker response did not contain the number of total peers, it will be set to the number of peers in the response.
    fn update_total_peers(&self, tracker_response: &TrackerResponse) {
        if tracker_response.complete == 0 && tracker_response.incomplete == 0 {
            self.torrent_status
                .update_total_peers(tracker_response.peers.len(), 0);
        } else {
            self.torrent_status.update_total_peers(
                tracker_response.complete as usize,
                tracker_response.incomplete as usize,
            );
        }
    }

    fn connect_to_peer(&mut self, peer: BtPeer) -> Result<(), TorrentHandlerError> {
        self.torrent_status.peer_connecting();
        let peer_name = format!("{}:{}", peer.ip, peer.port);

        let mut peer_session = PeerSession::new(
            peer.clone(),
            self.torrent.clone(),
            self.torrent_status.clone(),
            self.config.clone(),
            self.logger_sender.clone(),
            self.client_peer_id.clone(),
        )
        .map_err(TorrentHandlerError::PeerSessionError)?;

        let builder = thread::Builder::new().name(format!(
            "Torrent: {} / Peer: {}",
            self.torrent.info.name, peer_name
        ));
        let peer_logger_sender = self.logger_sender.clone();

        let join = builder.spawn(move || match peer_session.start_outgoing_seeder() {
            Ok(_) => (),
            Err(err) => {
                peer_logger_sender.warn(&format!("{:?}", err));
            }
        });
        match join {
            Ok(_) => (),
            Err(err) => {
                self.logger_sender.error(&format!("{:?}", err));
                self.torrent_status
                    .peer_disconnected(&peer)
                    .map_err(TorrentHandlerError::TorrentStatusError)?;
            }
        }
        Ok(())
    }
}
