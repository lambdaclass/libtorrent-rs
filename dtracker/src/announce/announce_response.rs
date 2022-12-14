use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use bencoder::bencode::ToBencode;

use crate::{tracker_peer::peer::Peer, tracker_status::atomic_tracker_status::AtomicTrackerStatus};

use super::announce_request::AnnounceRequest;

/// Struct representing the response of a tracker announce request.
///
/// # Fields
/// * `failure_reason`:  If present, then no other keys may be present. The value is a human-readable error message as to why the request failed.
/// * `warning_message`: Similar to failure reason, but the response still gets processed normally. The warning message is shown just like an error.
/// * `interval`: Interval in seconds that the client should wait between sending regular requests to the tracker.
/// * `min_interval`: Minimum announce interval. If present clients must not reannounce more frequently than this.
/// * `tracker_id`: A string that the client should send back on its next announcements. If absent and a previous announce sent a tracker id, do not discard the old value; keep using it.
/// * `complete`: number of peers with the entire file, i.e. seeders.
/// * `incomplete`: number of non-seeder peers, aka "leechers".
/// * `peers`: (dictionary model) The value is a list of dictionaries, each with the following keys:
///    - **peer_id**: peer's self-selected ID, as described above for the tracker request (string)
///    - **ip**: peer's IP address either IPv6 (hexed) or IPv4 (dotted quad) or DNS name (string)
///    - **port**: peer's port number (integer)
/// * `peers_binary`: peers: (binary model) Instead of using the dictionary model described above, the peers value may be a string consisting of multiples of 6 bytes. First 4 bytes are the IP address and last 2 bytes are the port number. All in network (big endian) notation.
#[derive(Debug)]
pub struct AnnounceResponse {
    pub failure_reason: Option<String>,
    pub warning_message: Option<String>,
    pub interval: u32,
    pub min_interval: Option<u32>,
    pub tracker_id: Option<String>,
    pub complete: u32,
    pub incomplete: u32,
    pub peers: Vec<Peer>,
}

impl AnnounceResponse {
    /// Creates a new AnnounceResponse from a HashMap containing the query parameters of the announce request.
    pub fn from(
        query_params: HashMap<String, String>,
        tracker_status: Arc<AtomicTrackerStatus>,
        peer_ip: String,
    ) -> Self {
        let announce_request = match AnnounceRequest::new_from(query_params) {
            Ok(announce_request) => announce_request,
            Err(announce_request_error) => {
                return Self::create_error_response(announce_request_error.to_string())
            }
        };

        let peer = Peer::from_request(announce_request.clone(), peer_ip);

        let active_peers = tracker_status.incoming_peer(
            announce_request.info_hash,
            peer,
            announce_request.numwant,
        );

        // TODO: Handle announce_request.compact == true case.

        Self::create_success_response(
            active_peers.peers,
            active_peers.seeders,
            active_peers.leechers,
        )
    }

    fn create_error_response(failure_reason: String) -> Self {
        Self {
            failure_reason: Some(failure_reason),
            warning_message: None,
            interval: 0,
            min_interval: None,
            tracker_id: None,
            complete: 0,
            incomplete: 0,
            peers: Vec::new(),
        }
    }

    fn create_success_response(peers_list: Vec<Peer>, complete: u32, incomplete: u32) -> Self {
        Self {
            failure_reason: None,
            warning_message: None,
            interval: 0,
            min_interval: None,
            tracker_id: None,
            complete,
            incomplete,
            peers: peers_list,
        }
    }
}

impl ToBencode for AnnounceResponse {
    fn to_bencode(&self) -> bencoder::bencode::Bencode {
        let mut announce_response = BTreeMap::new();
        if let Some(failure_reason) = &self.failure_reason {
            announce_response.insert(b"failure reason".to_vec(), failure_reason.to_bencode());
        }
        if let Some(warning_message) = &self.warning_message {
            announce_response.insert(b"warning message".to_vec(), warning_message.to_bencode());
        }
        announce_response.insert(b"interval".to_vec(), self.interval.to_bencode());
        if let Some(min_interval) = &self.min_interval {
            announce_response.insert(b"min interval".to_vec(), min_interval.to_bencode());
        }
        if let Some(tracker_id) = &self.tracker_id {
            announce_response.insert(b"tracker id".to_vec(), tracker_id.to_bencode());
        }
        announce_response.insert(b"complete".to_vec(), self.complete.to_bencode());
        announce_response.insert(b"incomplete".to_vec(), self.incomplete.to_bencode());
        announce_response.insert(b"peers".to_vec(), self.peers.to_bencode());
        announce_response.to_bencode()
    }
}
