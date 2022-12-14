use std::collections::BTreeMap;

use bencoder::bencode::ToBencode;
use chrono::{DateTime, Local};

use crate::announce::announce_request::AnnounceRequest;

use super::{event::PeerEvent, peer_status::PeerStatus};

/// Struct that represents a peer.
///
/// ## Fields
/// * `id`: The id of the peer.
/// * `ip`: The ip of the peer.
/// * `port`: The port of the peer.
/// * `status`: The current status of the peer.
/// * `key`: The key to use to differentiate between other peers *(Optional)*.
#[derive(Debug, Clone)]
pub struct Peer {
    pub id: [u8; 20],
    pub ip: String,
    pub port: u16,
    pub status: PeerStatus,
    pub key: Option<String>, //link a wiki.theory.org:  https://bit.ly/3aTXQ3u
}
impl Peer {
    /// Creates a new peer.
    pub fn new(
        id: [u8; 20],
        ip: String,
        port: u16,
        key: Option<String>,
        status: PeerStatus,
    ) -> Peer {
        Peer {
            id,
            ip,
            port,
            status,
            key,
        }
    }

    /// Creates a new peer from an AnnounceRequest.
    pub fn from_request(request: AnnounceRequest, peer_ip: String) -> Self {
        let id = request.peer_id;
        let ip = match request.ip {
            Some(ip) => ip,
            None => peer_ip,
        };
        let port = request.port;
        let key = request.key;

        let status = PeerStatus::new(
            request.uploaded,
            request.downloaded,
            request.left,
            request.event,
        );

        Self::new(id, ip, port, key, status)
    }

    pub fn get_last_seen(&self) -> DateTime<Local> {
        self.status.last_seen()
    }

    /// Returns `true` if the given peer is acting as a leecher, `false` on the contrary.
    pub fn is_leecher(&self) -> bool {
        self.status.left > 0
            || (self.status.event != Some(PeerEvent::Completed) && self.status.event != None)
    }
    /// Returns `true` if the given peer is acting as a seeder, `false` on the contrary.
    pub fn is_seeder(&self) -> bool {
        self.status.left == 0 || self.status.event == Some(PeerEvent::Completed)
    }
}

impl ToBencode for Peer {
    fn to_bencode(&self) -> bencoder::bencode::Bencode {
        let mut peer = BTreeMap::new();
        peer.insert(b"peer_id".to_vec(), self.id.to_vec().to_bencode());
        peer.insert(b"ip".to_vec(), self.ip.to_bencode());
        peer.insert(b"port".to_vec(), self.port.to_bencode());
        peer.to_bencode()
    }
}
