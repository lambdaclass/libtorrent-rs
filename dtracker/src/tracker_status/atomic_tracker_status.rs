use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

use chrono::Duration;

use crate::{
    torrent_swarm::swarm::{ActivePeers, Swarm},
    tracker_peer::peer::Peer,
};

use super::current_tracker_stats::CurrentTrackerStats;

const PEER_HOURS_TIMEOUT: i64 = 1;
type InfoHash = [u8; 20];

/// Struct that represents the current status of the tracker.
///
/// ## Fields
/// * `torrents`: The current torrents supported by the tracker. The key is the torrent `Info Hash`. The value is the `Torrent Status`.
#[derive(Debug)]
pub struct AtomicTrackerStatus {
    torrent_swarms: Mutex<HashMap<InfoHash, Swarm>>,
}

impl Default for AtomicTrackerStatus {
    /// Creates a new tracker status.
    fn default() -> Self {
        AtomicTrackerStatus {
            torrent_swarms: Mutex::new(HashMap::new()),
        }
    }
}

impl AtomicTrackerStatus {
    /// Adds or updates a peer for a torrent in the tracker status and returns an `ActivePeers` struct.
    ///
    /// ## Arguments
    /// * `info_hash`: The info hash of the torrent.
    /// * `peer`: The peer to add or update.
    /// * `numwant`: The number of peers wanted by the client.
    ///
    /// ## Returns
    /// * `ActivePeers`: Struct containing the peers of the torrent requested, the number of seeders and leechers.
    pub fn incoming_peer(&self, info_hash: InfoHash, peer: Peer, wanted_peers: u32) -> ActivePeers {
        let mut swarms = self.lock_swarms();
        let torrent_swarm = swarms
            .entry(info_hash)
            .or_insert_with(|| Swarm::new(Duration::hours(PEER_HOURS_TIMEOUT)));

        torrent_swarm.announce(peer);

        torrent_swarm.get_active_peers(wanted_peers)
    }

    /// Gets the current statistics of the tracker.
    ///
    /// ## Returns
    /// * `CurrentTrackerStats`: Struct containing the total number of torrents, seeders and leechers.
    pub fn get_global_statistics(&self) -> CurrentTrackerStats {
        let swarms = self.lock_swarms();

        let total_torrents = swarms.len() as u32;
        let mut global_seeders = 0;
        let mut global_leechers = 0;

        for swarm in swarms.values() {
            let (seeders, leechers) = swarm.get_current_seeders_and_leechers();
            global_seeders += seeders;
            global_leechers += leechers;
        }

        CurrentTrackerStats::new(total_torrents, global_seeders, global_leechers)
    }

    /// Removes any inactive peers from each swarm.
    pub fn remove_inactive_peers(&self) {
        for swarm in self.lock_swarms().values_mut() {
            swarm.remove_inactive_peers();
        }
    }

    fn lock_swarms(&self) -> MutexGuard<HashMap<InfoHash, Swarm>> {
        self.torrent_swarms.lock().unwrap() // Unwrap is safe here because we're the only ones who call this function.
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Sub;

    use chrono::Local;

    use crate::tracker_peer::peer_status::PeerStatus;

    use super::*;

    #[test]
    fn test_incoming_seeder() {
        let tracker_status = AtomicTrackerStatus::default();
        let a_seeder = create_test_seeder([0; 20]);
        let info_hash = [0; 20];

        tracker_status.incoming_peer(info_hash, a_seeder, 50);

        assert_there_is_only_one_seeder(&tracker_status, info_hash);
    }

    #[test]
    fn test_incoming_leecher() {
        let tracker_status = AtomicTrackerStatus::default();
        let a_leecher = create_test_leecher([0; 20]);
        let info_hash = [0; 20];

        tracker_status.incoming_peer(info_hash, a_leecher, 50);

        assert_there_is_only_one_leecher(&tracker_status, info_hash);
    }

    #[test]
    fn test_multiple_incoming_peers_on_the_same_torrent() {
        let tracker_status = AtomicTrackerStatus::default();
        let a_peer = create_test_seeder([0; 20]);
        let another_peer = create_test_leecher([1; 20]);
        let info_hash = [0; 20];

        tracker_status.incoming_peer(info_hash, a_peer, 50);
        tracker_status.incoming_peer(info_hash, another_peer, 50);

        assert_there_are_only_these_peers(&tracker_status, info_hash, 1, 1);
    }

    #[test]
    fn test_returning_peer() {
        let tracker_status = AtomicTrackerStatus::default();
        let peer_id = [0; 20];
        let a_peer = create_test_leecher(peer_id);
        let info_hash = [0; 20];

        tracker_status.incoming_peer(info_hash, a_peer, 50);
        tracker_status.incoming_peer(info_hash, create_test_seeder(peer_id), 50);

        assert_there_is_only_one_seeder(&tracker_status, info_hash);
    }

    #[test]
    fn test_peers_on_multiple_torrents() {
        let tracker_status = AtomicTrackerStatus::default();
        let a_peer = create_test_leecher([0; 20]);
        let another_peer = create_test_leecher([1; 20]);
        let an_info_hash = [0; 20];
        let another_info_hash = [1; 20];

        tracker_status.incoming_peer(an_info_hash, a_peer, 50);
        tracker_status.incoming_peer(another_info_hash, another_peer, 50);

        assert_there_is_only_one_leecher(&tracker_status, an_info_hash);
        assert_there_is_only_one_leecher(&tracker_status, another_info_hash);
    }

    #[test]
    fn test_peer_can_get_inactive() {
        let tracker_status = AtomicTrackerStatus::default();
        let peer_id = [0; 20];
        let a_peer = create_test_seeder(peer_id);
        let an_info_hash = [0; 20];
        tracker_status.incoming_peer(an_info_hash, a_peer, 50);

        let inactive_peer = create_inactive_peer(peer_id);
        tracker_status.incoming_peer(an_info_hash, inactive_peer, 50);
        tracker_status.remove_inactive_peers();

        assert_there_are_only_these_peers(&tracker_status, an_info_hash, 0, 0);
    }

    fn assert_there_are_only_these_peers(
        status: &AtomicTrackerStatus,
        info_hash: [u8; 20],
        expected_seeders: u32,
        expected_leechers: u32,
    ) {
        let (active_peers, seeders, leechers) =
            get_active_peers_for(status, info_hash, 50).unwrap();
        assert_eq!(
            active_peers.len(),
            (expected_seeders + expected_leechers) as usize
        );
        assert_eq!(seeders, expected_seeders);
        assert_eq!(leechers, expected_leechers);
    }

    fn assert_there_is_only_one_seeder(status: &AtomicTrackerStatus, info_hash: [u8; 20]) {
        assert_there_are_only_these_peers(status, info_hash, 1, 0);
        let (active_peers, _, _) = get_active_peers_for(status, info_hash, 50).unwrap();
        assert!(active_peers[0].is_seeder());
    }

    fn assert_there_is_only_one_leecher(status: &AtomicTrackerStatus, info_hash: [u8; 20]) {
        assert_there_are_only_these_peers(status, info_hash, 0, 1);
        let (active_peers, _, _) = get_active_peers_for(status, info_hash, 50).unwrap();
        assert!(active_peers[0].is_leecher());
    }

    pub fn get_active_peers_for(
        status: &AtomicTrackerStatus,
        info_hash: [u8; 20],
        wanted_peers: u32,
    ) -> Option<(Vec<Peer>, u32, u32)> {
        let all_swarms = status.lock_swarms();
        let swarm = all_swarms.get(&info_hash)?;

        let active_peers = swarm.get_active_peers(wanted_peers);

        Some((
            active_peers.peers,
            active_peers.seeders,
            active_peers.leechers,
        ))
    }

    fn create_test_seeder(peer_id: [u8; 20]) -> Peer {
        let peer_status = PeerStatus {
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: None,
            last_seen: Local::now(),
        };

        Peer::new(peer_id, "0".to_string(), 0, None, peer_status)
    }

    fn create_test_leecher(peer_id: [u8; 20]) -> Peer {
        let peer_status = PeerStatus {
            uploaded: 0,
            downloaded: 0,
            left: 3000,
            event: None,
            last_seen: Local::now(),
        };

        Peer::new(peer_id, "0".to_string(), 0, None, peer_status)
    }

    fn create_inactive_peer(peer_id: [u8; 20]) -> Peer {
        let old_date = Local::now().sub(Duration::hours(PEER_HOURS_TIMEOUT) * 2);
        let peer_status = PeerStatus {
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: None,
            last_seen: old_date,
        };

        Peer::new(peer_id, "0".to_string(), 0, None, peer_status)
    }
}
