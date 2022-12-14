use std::collections::HashMap;

use chrono::{Duration, Local};
use rand::{seq::IteratorRandom, thread_rng};

use crate::tracker_peer::peer::Peer;

type PeerId = [u8; 20];

/// Struct that represents the status of a torrent.
///
/// ## Fields
/// * `peer_timeout`: The time after which a peer is considered as inactive.
/// * `seeders`: The current amount of seeders of the torrent.
/// * `leechers`: The current amount of leechers of the torrent.
#[derive(Debug, Clone)]
pub struct Swarm {
    peers: HashMap<PeerId, Peer>,
    peer_timeout: Duration,
    seeders: u32,
    leechers: u32,
}

/// Struct that represents the response to an active peers request.
///
/// ## Fields
/// * `peers`: The current peers of the swarm.
/// * `seeders`: The current amount of seeders of the swarm.
/// * `leechers`: The current amount of leechers of the swarm.
#[derive(Debug, Clone)]
pub struct ActivePeers {
    pub peers: Vec<Peer>,
    pub seeders: u32,
    pub leechers: u32,
}

impl Swarm {
    /// Creates a new swarm.
    ///
    /// ## Arguments
    /// * `peer_timeout`: The timeout for a peer to be considered inactive.
    pub fn new(peer_timeout: Duration) -> Self {
        Self {
            peers: HashMap::new(),
            peer_timeout,
            seeders: 0,
            leechers: 0,
        }
    }

    pub fn announce(&mut self, incoming_peer: Peer) {
        let old_peer = self.peers.insert(incoming_peer.id, incoming_peer.clone());
        // If the peer was already in the swarm, we update it accordingly.

        if let Some(old_peer) = old_peer {
            if old_peer.is_leecher() {
                self.leechers -= 1;
            } else {
                self.seeders -= 1;
            }
        };

        if incoming_peer.is_leecher() {
            self.leechers += 1;
        } else {
            self.seeders += 1;
        }
    }
    /// Returns an `ActivePeers` Struct containing a vector of active peers, the amount of seeders in the swarm and the amount of leechers in the swarm.
    ///
    /// ## Arguments
    /// * `wanted_peers`: The amount of active peers to include in the vector, unless the swarm does not contain as many active peers, in which case it equals the number of elements available.
    pub fn get_active_peers(&self, wanted_peers: u32) -> ActivePeers {
        let peers = self.peers.values().cloned();

        let mut rng = thread_rng();
        let active_peers = peers
            .into_iter()
            .choose_multiple(&mut rng, wanted_peers as usize);

        ActivePeers {
            peers: active_peers,
            seeders: self.seeders,
            leechers: self.leechers,
        }
    }

    /// Returns the current amount of seeders and leechers in the swarm.
    pub fn get_current_seeders_and_leechers(&self) -> (u32, u32) {
        (self.seeders, self.leechers)
    }

    /// Removes any inactive peers from the swarm.
    pub fn remove_inactive_peers(&mut self) {
        self.peers.retain(|_, peer| {
            let last_seen = peer.get_last_seen();
            if Local::now().signed_duration_since(last_seen) > self.peer_timeout {
                if peer.is_leecher() {
                    self.leechers -= 1;
                } else {
                    self.seeders -= 1;
                }
                false
            } else {
                true
            }
        });
    }
}
