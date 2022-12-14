use std::str::FromStr;

/// Posible announce events that can be sent to the tracker.
///
/// ## Fields
/// * `started`: The peer has started downloading the torrent.
/// * `stopped`: The peer has stopped downloading the torrent.
/// * `completed`: The peer has completed downloading the torrent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerEvent {
    Started,
    Stopped,
    Completed,
}

impl FromStr for PeerEvent {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "started" => Ok(PeerEvent::Started),
            "stopped" => Ok(PeerEvent::Stopped),
            "completed" => Ok(PeerEvent::Completed),
            _ => Err(()),
        }
    }
}
