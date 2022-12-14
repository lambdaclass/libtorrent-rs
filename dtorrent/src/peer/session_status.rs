use super::peer_message::Bitfield;

/// Represents our status in the peer session.
#[derive(Debug, Clone)]
pub struct SessionStatus {
    /// We are choked
    pub choked: bool,
    /// We are interested
    pub interested: bool,
    /// The other peer is choked by us
    pub peer_choked: bool,
    /// The other peer is interested in us
    pub peer_interested: bool,
    pub bitfield: Bitfield,
    pub download_speed: f64,
    pub upload_speed: f64,
}

impl SessionStatus {
    pub fn new(bitfield: Bitfield) -> Self {
        Self {
            choked: true,
            interested: false,
            peer_choked: true,
            peer_interested: false,
            bitfield,
            download_speed: 0.0,
            upload_speed: 0.0,
        }
    }
}
