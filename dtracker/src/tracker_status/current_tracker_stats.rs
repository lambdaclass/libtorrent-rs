use serde::{Deserialize, Serialize};

/// Struct containing the current stats of the tracker.
///
/// ## Fields
/// * `torrents`: The total number of torrents in the tracker.
/// * `seeders`: The total number of seeders in the tracker.
/// * `leechers`: The total number of leechers in the tracker.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CurrentTrackerStats {
    pub torrents: u32,
    pub seeders: u32,
    pub leechers: u32,
}

impl CurrentTrackerStats {
    /// Creates a new `CurrentTrackerStats`.
    pub fn new(torrents: u32, seeders: u32, leechers: u32) -> Self {
        Self {
            torrents,
            seeders,
            leechers,
        }
    }
}
