use super::event::PeerEvent;
use chrono::{DateTime, Local};

/// Struct that represents a peer status.
///
/// ## Fields
/// * `uploaded`: The number of bytes uploaded by the peer.
/// * `downloaded`: The number of bytes downloaded by the peer.
/// * `left`: The number of bytes left to download.
/// * `event`: The last event that the peer has sent *(Optional)*.
/// * `last_seen`: The last time the peer status was updated.
#[derive(Debug, Clone)]
pub struct PeerStatus {
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub event: Option<PeerEvent>,
    pub last_seen: DateTime<Local>,
}

impl PeerStatus {
    /// Creates a new peer status.
    pub fn new(uploaded: u64, downloaded: u64, left: u64, event: Option<PeerEvent>) -> PeerStatus {
        PeerStatus {
            uploaded,
            downloaded,
            left,
            event,
            last_seen: Local::now(),
        }
    }

    pub fn last_seen(&self) -> DateTime<Local> {
        self.last_seen
    }
}
