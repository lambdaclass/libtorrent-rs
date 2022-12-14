use crate::{
    config::cfg::Cfg,
    peer::{bt_peer::BtPeer, peer_message::Bitfield, session_status::SessionStatus},
    storage_manager::manager::{retrieve_block, save_piece},
    torrent_parser::torrent::Torrent,
};
use rand::{self, prelude::IteratorRandom};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{sync_channel, Receiver, SyncSender},
        {Mutex, MutexGuard},
    },
};

/// A Struct that represents the current status of a torrent.
///
/// It contains the following information:
///
/// - The current number of peers that are downloading the torrent.
/// - The current state of the pieces of the torrent.
///
/// It is `Atomic`, meaning that it can be accessed from multiple threads at the same time.
///
/// To create a new `AtomicTorrentStatus`, use the `new()` method.
///
/// The `new()` method also returns a `Receiver` that can be used to know when a peer disconnects. This is useful if there is a limit for how many peers can be created,
/// so the thread can be blocked until the status notifies that a peer has disconnected.
#[derive(Debug)]
pub struct AtomicTorrentStatus {
    pub torrent: Torrent, //TODO: resolver encqapsulamiento en statistics.rs
    pieces_status: Mutex<HashMap<u32, PieceStatus>>,
    current_peers: AtomicUsize,
    config: Cfg,
    torrent_status_sender: SyncSender<usize>,
    sessions_status: Mutex<HashMap<BtPeer, SessionStatus>>,
    finished_pieces: AtomicUsize,
    downloading_pieces: AtomicUsize,
    free_pieces: AtomicUsize,
    total_seeders_count: AtomicUsize,
    total_leechers_count: AtomicUsize,
    all_current_peers: AtomicUsize,
}

/// Possible states of a piece.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PieceStatus {
    Finished,
    Downloading,
    Free,
}

/// Totrrent status possible errors.
#[derive(Debug)]
pub enum AtomicTorrentStatusError {
    PoisonedPiecesStatusLock,
    PoisonedCurrentPeersLock,
    PoisonedSessionsStatusLock,
    InvalidPieceIndex,
    NoPeersConnected,
    PieceWasNotDownloading,
    SavePieceError(std::io::Error),
    RetrievingPieceError(std::io::Error),
    PieceWasNotFinished,
}

impl AtomicTorrentStatus {
    /// Creates a new `AtomicTorrentStatus` from a `Torrent` and a `config`.
    ///
    /// Returns a tuple with the `AtomicTorrentStatus` and a channel `Receiver` that can be used optionally to receive when a peer disconects from the torrent status.
    /// The value sent on the channel is the current number of peers connected.
    pub fn new(torrent: &Torrent, config: Cfg) -> (Self, Receiver<usize>) {
        let mut pieces_status: HashMap<u32, PieceStatus> = HashMap::new();
        let sessions_status: HashMap<BtPeer, SessionStatus> = HashMap::new();

        let (torrent_status_sender, torrent_status_receiver): (SyncSender<usize>, Receiver<usize>) =
            sync_channel((config.max_peers_per_torrent * 100) as usize);

        let total_pieces = torrent.total_pieces();

        for index in 0..total_pieces {
            pieces_status.insert(index as u32, PieceStatus::Free);
        }

        (
            Self {
                torrent: torrent.clone(),
                pieces_status: Mutex::new(pieces_status),
                current_peers: AtomicUsize::new(0),
                config,
                torrent_status_sender,
                sessions_status: Mutex::new(sessions_status),
                finished_pieces: AtomicUsize::new(0),
                downloading_pieces: AtomicUsize::new(0),
                free_pieces: AtomicUsize::new(total_pieces as usize),
                total_seeders_count: AtomicUsize::new(0),
                total_leechers_count: AtomicUsize::new(0),
                all_current_peers: AtomicUsize::new(0),
            },
            torrent_status_receiver,
        )
    }

    /// Returns true if the torrent download finished.
    pub fn is_finished(&self) -> bool {
        self.finished_pieces.load(Ordering::Relaxed) == self.torrent.total_pieces() as usize
    }

    /// Returns the number of ramaining pieces to download.
    pub fn remaining_pieces(&self) -> usize {
        self.torrent.total_pieces() as usize - self.finished_pieces.load(Ordering::Relaxed)
    }

    /// Returns the number of pieces that are currently downloading.
    pub fn downloading_pieces(&self) -> usize {
        self.downloading_pieces.load(Ordering::Relaxed)
    }

    /// Returns the number of pieces that are already downloaded.
    pub fn downloaded_pieces(&self) -> usize {
        self.finished_pieces.load(Ordering::Relaxed)
    }

    /// Adds a new peer to the current number of peers.
    ///
    /// # Errors
    /// - `PoisonedSessionsStatusLock` if the lock on the `session_status` field is poisoned.
    pub fn peer_connected(&self, peer: &BtPeer) -> Result<(), AtomicTorrentStatusError> {
        self.current_peers.fetch_add(1, Ordering::Relaxed);
        let mut peer_status = self.lock_session_status()?;
        peer_status.insert(peer.clone(), SessionStatus::new(Bitfield::new(vec![])));
        Ok(())
    }

    /// Adds a new peer to the current number of peers.
    pub fn peer_connecting(&self) {
        self.all_current_peers.fetch_add(1, Ordering::Relaxed);
    }

    /// Removes a peer from the current number of peers.
    ///
    /// # Errors
    /// - `PoisonedSessionsStatusLock` if the lock on the `session_status` field is poisoned.
    /// - `NoPeersConnected` if there are no peers connected.
    pub fn peer_disconnected(&self, peer: &BtPeer) -> Result<(), AtomicTorrentStatusError> {
        let mut peer_status = self.lock_session_status()?;
        if self.current_peers.load(Ordering::Relaxed) == 0 {
            return Err(AtomicTorrentStatusError::NoPeersConnected);
        }
        self.current_peers.fetch_sub(1, Ordering::Relaxed);
        self.all_current_peers.fetch_sub(1, Ordering::Relaxed);

        peer_status.remove(peer);

        self.notify_peer_disconnected();
        Ok(())
    }

    /// Removes a peer from the current number of connecting peers.
    pub fn peer_connecting_failed(&self) {
        self.all_current_peers.fetch_sub(1, Ordering::Relaxed);
        self.notify_peer_disconnected();
    }

    /// Notifies the torrent status receiver that a peer has disconnected.
    fn notify_peer_disconnected(&self) {
        // If the value couldn't be sent, it means the channel was closed.
        if self
            .torrent_status_sender
            .send(self.all_current_peers.load(Ordering::Relaxed))
            .is_ok()
        {}
    }

    /// Returns the current number of peers.
    pub fn current_peers(&self) -> usize {
        self.current_peers.load(Ordering::Relaxed)
    }

    /// Returns the number of peers connected and connecting to the torrent.
    pub fn all_current_peers(&self) -> usize {
        self.all_current_peers.load(Ordering::Relaxed)
    }

    /// Updates the peer session status of a peer.
    ///
    /// # Errors
    /// - `PoisonedSessionsStatusLock` if the lock on the `session_status` field is poisoned.
    pub fn update_peer_session_status(
        &self,
        peer: &BtPeer,
        status: &SessionStatus,
    ) -> Result<(), AtomicTorrentStatusError> {
        let mut peer_status = self.lock_session_status()?;
        peer_status.remove(peer);
        peer_status.insert(peer.clone(), status.clone());
        Ok(())
    }

    /// Updates the count of seeders and leechers.
    pub fn update_total_peers(&self, seeders_count: usize, leechers_count: usize) {
        self.total_seeders_count
            .store(seeders_count, Ordering::Relaxed);
        self.total_leechers_count
            .store(leechers_count, Ordering::Relaxed);
    }

    /// Returns a tuple containing the number of seeders and leechers.
    ///
    /// The sum of the two values is the total number of peers.
    pub fn get_total_peers(&self) -> (usize, usize) {
        let seeders = self.total_seeders_count.load(Ordering::Relaxed);
        let leechers = self.total_leechers_count.load(Ordering::Relaxed);
        (seeders, leechers)
    }

    /// Returns the connected peers.
    ///
    /// # Errors
    /// - `PoisonedSessionsStatusLock` if the lock on the `session_status` field is poisoned.
    pub fn get_connected_peers(
        &self,
    ) -> Result<HashMap<BtPeer, SessionStatus>, AtomicTorrentStatusError> {
        Ok(self.lock_session_status()?.clone())
    }

    /// Returns the current download speed of the torrent in kilobits per second.
    ///
    /// # Errors
    /// - `PoisonedSessionsStatusLock` if the lock on the `session_status` field is poisoned.
    pub fn torrent_download_speed(&self) -> Result<f64, AtomicTorrentStatusError> {
        Ok(self
            .lock_session_status()?
            .values()
            .map(|peer_session| peer_session.download_speed)
            .sum())
    }

    /// Returns the current upload speed of the torrent in kilobits per second.
    ///
    /// # Errors
    /// - `PoisonedSessionsStatusLock` if the lock on the `session_status` field is poisoned.
    pub fn torrent_upload_speed(&self) -> Result<f64, AtomicTorrentStatusError> {
        Ok(self
            .lock_session_status()?
            .values()
            .map(|peer_session| peer_session.upload_speed)
            .sum())
    }

    /// Returns the index of a piece that can be downloaded from a peer `Bitfield` passed by parameter.
    ///
    /// If none of the pieces can be downloaded, returns `None`.
    ///
    /// # Errors
    /// - `PoisonedPiecesStatusLock` if the lock on the `pieces_status` field is poisoned.
    pub fn select_piece(
        &self,
        bitfield: &Bitfield,
    ) -> Result<Option<u32>, AtomicTorrentStatusError> {
        let mut pieces_status = self.lock_pieces_status()?;

        // If there are no free pieces do the 'EndGame' strategy, otherwise do the normal piece selection.
        let index = if pieces_status
            .values()
            .filter(|status| **status == PieceStatus::Free)
            .count()
            == 0
        {
            pieces_status
                .clone()
                .iter()
                .filter(|(_, status)| **status == PieceStatus::Downloading)
                .choose(&mut rand::thread_rng())
                .map(|(index, _)| *index)
        } else {
            pieces_status
                .clone()
                .iter()
                .filter(|(_, status)| **status == PieceStatus::Free)
                .find(|(index, _)| bitfield.has_piece(**index))
                .map(|(index, _)| *index)
        };

        Ok(match index {
            Some(index) => {
                pieces_status.insert(index, PieceStatus::Downloading);
                self.downloading_pieces.fetch_add(1, Ordering::Relaxed);
                self.free_pieces.fetch_sub(1, Ordering::Relaxed);
                Some(index)
            }
            None => None,
        })
    }

    /// Saves a downlaoded piece to the disk.
    ///
    /// # Errors
    /// - `PoisonedPiecesStatusLock` if the lock on the `pieces_status` field is poisoned.
    /// - `InvalidPieceIndex` if the piece index is invalid.
    /// - `PieceWasNotDownloading` if the piece was not downloading.
    /// - `SavePieceError` if the piece could not be saved.
    pub fn piece_downloaded(
        &self,
        index: u32,
        piece: &[u8],
    ) -> Result<(), AtomicTorrentStatusError> {
        let mut piece_status = self.lock_pieces_status()?;
        match piece_status.get(&index) {
            Some(value) => {
                if *value != PieceStatus::Downloading {
                    return Err(AtomicTorrentStatusError::PieceWasNotDownloading);
                }
            }
            None => return Err(AtomicTorrentStatusError::InvalidPieceIndex),
        }
        save_piece(
            self.torrent.info.name.clone(),
            piece,
            (index * self.torrent.info.piece_length as u32) as u64,
            self.config.clone(),
        )
        .map_err(AtomicTorrentStatusError::SavePieceError)?;

        piece_status.insert(index, PieceStatus::Finished);
        self.downloading_pieces.fetch_sub(1, Ordering::Relaxed);
        self.finished_pieces.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Gets a piece already downloaded from the disk.
    ///
    /// # Errors
    /// - `PoisonedPiecesStatusLock` if the lock on the `pieces_status` field is poisoned.
    /// - `InvalidPieceIndex` if the piece index is invalid.
    /// - `PieceWasNotFinished` if the piece was not donwloaded.
    pub fn get_piece(
        &self,
        index: u32,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>, AtomicTorrentStatusError> {
        let pieces_status = self.lock_pieces_status()?;

        match pieces_status.get(&index) {
            Some(value) => {
                if *value != PieceStatus::Finished {
                    return Err(AtomicTorrentStatusError::PieceWasNotFinished);
                }
            }
            None => return Err(AtomicTorrentStatusError::InvalidPieceIndex),
        }

        retrieve_block(
            self.torrent.info.name.clone(),
            offset,
            length,
            self.config.clone(),
        )
        .map_err(AtomicTorrentStatusError::RetrievingPieceError)
    }

    /// Aborts a piece download.
    ///
    /// This must be called when a piece obteined from `select_piece` can not longer be downloaded.
    ///
    /// # Errors
    /// - `PoisonedPiecesStatusLock` if the lock on the `pieces_status` field is poisoned.
    /// - `InvalidPieceIndex` if the piece index is invalid.
    /// - `PieceWasNotDownloading` if the piece was not downloading.
    pub fn piece_aborted(&self, index: u32) -> Result<(), AtomicTorrentStatusError> {
        let mut piece_status = self.lock_pieces_status()?;
        match piece_status.get(&index) {
            Some(value) => {
                if *value != PieceStatus::Downloading {
                    return Err(AtomicTorrentStatusError::PieceWasNotDownloading);
                }
            }
            None => return Err(AtomicTorrentStatusError::InvalidPieceIndex),
        }
        piece_status.insert(index, PieceStatus::Free);
        self.downloading_pieces.fetch_sub(1, Ordering::Relaxed);
        self.free_pieces.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Returns the current bitfield of the torrent.
    ///
    /// # Errors
    /// - `PoisonedPiecesStatusLock` if the lock on the `pieces_status` field is poisoned.
    pub fn get_bitfield(&self) -> Result<Bitfield, AtomicTorrentStatusError> {
        let pieces_status = self.lock_pieces_status()?;
        Ok(Bitfield::from(&pieces_status))
    }

    fn lock_pieces_status(
        &self,
    ) -> Result<MutexGuard<HashMap<u32, PieceStatus>>, AtomicTorrentStatusError> {
        self.pieces_status
            .lock()
            .map_err(|_| AtomicTorrentStatusError::PoisonedPiecesStatusLock)
    }

    fn lock_session_status(
        &self,
    ) -> Result<MutexGuard<HashMap<BtPeer, SessionStatus>>, AtomicTorrentStatusError> {
        self.sessions_status
            .lock()
            .map_err(|_| AtomicTorrentStatusError::PoisonedSessionsStatusLock)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc, thread};

    use crate::torrent_parser::info::Info;

    use super::*;

    const CONFIG_PATH: &str = "config.cfg";

    #[test]
    fn test_is_not_finished() {
        let torrent = create_test_torrent("test_is_not_finished");
        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        assert!(!status.is_finished());
    }

    #[test]
    fn test_is_finished() {
        let torrent = create_test_torrent("test_is_finished");

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        for _ in 0..(torrent.info.length / torrent.info.piece_length) {
            let index = status
                .select_piece(&Bitfield::new(vec![0b11111111, 0b11111111]))
                .unwrap()
                .unwrap();
            status.piece_downloaded(index as u32, &[]).unwrap();
        }
        assert!(status.is_finished());
        fs::remove_file(format!(
            "{}/{}",
            config.download_directory, torrent.info.name
        ))
        .unwrap();
    }

    #[test]
    fn test_starting_current_peers() {
        let torrent = create_test_torrent("test_starting_current_peers");

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        assert_eq!(0, status.current_peers());
    }

    #[test]
    fn test_peer_connected() {
        let torrent = create_test_torrent("test_peer_connected");
        let peer = create_test_peer("192.0".to_string());

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        status.peer_connected(&peer).unwrap();
        assert_eq!(1, status.current_peers());
    }

    #[test]
    fn test_peer_disconnected() {
        let torrent = create_test_torrent("test_peer_disconnected");
        let peer = create_test_peer("192.0".to_string());

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        status.peer_connected(&peer).unwrap();
        status.peer_connected(&peer).unwrap();
        status.peer_disconnected(&peer).unwrap();
        assert_eq!(1, status.current_peers());
    }

    #[test]
    fn test_peer_disconnected_error() {
        let torrent = create_test_torrent("test_peer_disconnected_error");
        let peer = create_test_peer("192.0".to_string());

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        assert!(status.peer_disconnected(&peer).is_err());
    }

    #[test]
    fn test_select_piece() {
        let torrent = create_test_torrent("test_piece_downloaded");

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        let index = status
            .select_piece(&Bitfield::new(vec![0b11111111, 0b11111111]))
            .unwrap()
            .unwrap();
        assert_eq!(
            *status.pieces_status.lock().unwrap().get(&index).unwrap(),
            PieceStatus::Downloading
        );
    }

    #[test]
    fn test_no_pieces_to_select() {
        let torrent = create_test_torrent("test_no_pieces_to_select");

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        let index = status
            .select_piece(&Bitfield::new(vec![0b00000000, 0b00000000]))
            .unwrap();
        assert!(index.is_none());
    }

    #[test]
    fn test_piece_downloaded() {
        let torrent = create_test_torrent("test_piece_downloaded");

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        let index = status
            .select_piece(&Bitfield::new(vec![0b11111111, 0b11111111]))
            .unwrap()
            .unwrap();
        status.piece_downloaded(index as u32, &[]).unwrap();
        assert_eq!(
            *status.pieces_status.lock().unwrap().get(&index).unwrap(),
            PieceStatus::Finished
        );
        fs::remove_file(format!(
            "{}/{}",
            config.download_directory, torrent.info.name
        ))
        .unwrap();
    }

    #[test]
    fn test_piece_aborted() {
        let torrent = create_test_torrent("test_piece_aborted");

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        let index = status
            .select_piece(&Bitfield::new(vec![0b11111111, 0b11111111]))
            .unwrap()
            .unwrap();
        status.piece_aborted(index).unwrap();
        assert_eq!(
            *status.pieces_status.lock().unwrap().get(&index).unwrap(),
            PieceStatus::Free
        );
    }

    #[test]
    fn test_bad_index() {
        let torrent = create_test_torrent("test_bad_index");

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        let index = 1000;
        assert!(status.piece_downloaded(index, &[]).is_err());
    }

    #[test]
    fn test_multiple_threads_current_peers() {
        let torrent = create_test_torrent("test_multiple_threads");

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = Arc::new(create_status_whitout_receiver(&torrent, config.clone()));
        let mut joins = Vec::new();

        for _ in 0..10 {
            let status_cloned = status.clone();
            let peer = create_test_peer("192.0".to_string());
            let join = thread::spawn(move || status_cloned.peer_connected(&peer).unwrap());
            joins.push(join);
        }
        for join in joins {
            join.join().unwrap();
        }
        assert_eq!(10, status.current_peers());
    }

    #[test]
    fn test_multiple_threads_piece_status() {
        let torrent = create_test_torrent("test_multiple_threads_piece_status");

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = Arc::new(create_status_whitout_receiver(&torrent, config.clone()));
        let mut joins = Vec::new();

        for _ in 0..10 {
            let status_cloned = status.clone();
            let join = thread::spawn(move || {
                let index = status_cloned
                    .select_piece(&Bitfield::new(vec![0b11111111, 0b11111111]))
                    .unwrap()
                    .unwrap();
                status_cloned.piece_downloaded(index, &[]).unwrap();
            });
            joins.push(join);
        }
        for join in joins {
            join.join().unwrap();
        }
        assert!(status.is_finished());
        fs::remove_file(format!(
            "{}/{}",
            config.download_directory, torrent.info.name
        ))
        .unwrap();
    }

    #[test]
    fn test_bad_downloaded() {
        let torrent = create_test_torrent("test_bad_downloaded");

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        let index = 0;
        assert!(status.piece_downloaded(index, &[]).is_err());
    }

    #[test]
    fn test_bad_abort() {
        let torrent = create_test_torrent("test_bad_abort");

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        let index = 0;
        assert!(status.piece_aborted(index).is_err());
    }

    #[test]
    fn test_remaining_pieces() {
        let torrent = create_test_torrent("test_remaining_pieces");

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());

        let total_pieces = (torrent.info.length / torrent.info.piece_length) as usize;

        let remaining_starting_pieces = status.remaining_pieces();

        let index = status
            .select_piece(&Bitfield::new(vec![0b11111111, 0b11111111]))
            .unwrap()
            .unwrap();
        status.piece_downloaded(index, &[]).unwrap();

        assert_eq!(remaining_starting_pieces, total_pieces);
        assert_eq!(status.remaining_pieces(), total_pieces - 1);
        fs::remove_file(format!(
            "{}/{}",
            config.download_directory, torrent.info.name
        ))
        .unwrap();
    }

    #[test]
    fn test_downloading_pieces() {
        let torrent = create_test_torrent("test_downloading_pieces");
        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());

        let _ = status
            .select_piece(&Bitfield::new(vec![0b11111111, 0b11111111]))
            .unwrap()
            .unwrap();

        assert_eq!(status.downloading_pieces(), 1);
    }

    #[test]
    fn test_downloaded_pieces() {
        let torrent = create_test_torrent("test_downloaded_pieces");
        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());

        let index = status
            .select_piece(&Bitfield::new(vec![0b11111111, 0b11111111]))
            .unwrap()
            .unwrap();
        status.piece_downloaded(index, &[]).unwrap();

        assert_eq!(status.downloaded_pieces(), 1);
        fs::remove_file(format!(
            "{}/{}",
            config.download_directory, torrent.info.name
        ))
        .unwrap();
    }

    #[test]
    fn test_torrent_status_channel() {
        let torrent = create_test_torrent("test_torrent_status_channel");
        let peer = create_test_peer("192.0".to_string());

        let (status, receiver) = AtomicTorrentStatus::new(&torrent, Cfg::new(CONFIG_PATH).unwrap());
        status.peer_connecting();
        status.peer_connecting();
        status.peer_connected(&peer).unwrap();
        status.peer_connected(&peer).unwrap();
        status.peer_disconnected(&peer).unwrap();
        assert_eq!(receiver.recv().unwrap(), 1);
    }

    #[test]
    fn test_torrent_download_speed() {
        let torrent = create_test_torrent("test_torrent_download_speed");
        let peer1 = create_test_peer("192.0".to_string());
        let peer2 = create_test_peer("932.0".to_string());

        let mut peer_session1 = create_test_peer_session_status();
        peer_session1.download_speed = 1500.0;

        let mut peer_session2 = create_test_peer_session_status();
        peer_session2.download_speed = 2500.0;

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        status.peer_connected(&peer1).unwrap();
        status.peer_connected(&peer2).unwrap();
        status
            .update_peer_session_status(&peer1, &peer_session1)
            .unwrap();
        status
            .update_peer_session_status(&peer2, &peer_session2)
            .unwrap();
        assert_eq!(status.torrent_download_speed().unwrap(), 4000.0);
    }

    #[test]
    fn test_torrent_upload_speed() {
        let torrent = create_test_torrent("test_torrent_upload_speed");
        let peer1 = create_test_peer("192.0".to_string());
        let peer2 = create_test_peer("932.0".to_string());

        let mut peer_session1 = create_test_peer_session_status();
        peer_session1.upload_speed = 100.0;

        let mut peer_session2 = create_test_peer_session_status();
        peer_session2.upload_speed = 200.0;

        let config = Cfg::new(CONFIG_PATH).unwrap();
        let status = create_status_whitout_receiver(&torrent, config.clone());
        status.peer_connected(&peer1).unwrap();
        status.peer_connected(&peer2).unwrap();
        status
            .update_peer_session_status(&peer1, &peer_session1)
            .unwrap();
        status
            .update_peer_session_status(&peer2, &peer_session2)
            .unwrap();
        assert_eq!(status.torrent_upload_speed().unwrap(), 300.0);
    }

    // Auxiliary functions

    fn create_test_torrent(name: &str) -> Torrent {
        let info = Info {
            length: 10,
            name: name.to_string(),
            piece_length: 1,
            pieces: vec![],
        };

        Torrent {
            announce_url: "announce".to_string(),
            info,
            info_hash: "info_hash".to_string(),
        }
    }

    fn create_test_peer(ip: String) -> BtPeer {
        BtPeer {
            peer_id: Some(vec![0x00]),
            ip: ip,
            port: 0,
            info_hash: None,
        }
    }

    fn create_test_peer_session_status() -> SessionStatus {
        SessionStatus::new(Bitfield::new(vec![]))
    }

    fn create_status_whitout_receiver(torrent: &Torrent, config: Cfg) -> AtomicTorrentStatus {
        let (status, _) = AtomicTorrentStatus::new(&torrent, config);
        status
    }
}
