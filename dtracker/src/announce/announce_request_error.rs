/// Possible errors that can occur when creating an AnnounceRequest.
#[derive(Debug)]
pub enum AnnounceRequestError {
    InvalidInfoHash,
    InvalidPeerId,
    InvalidPort,
    InvalidUploaded,
    InvalidDownloaded,
    InvalidLeft,
    InvalidIp,
    InvalidNumwant,
    InvalidKey,
    InvalidTrackerId,
    InvalidEvent,
}

impl ToString for AnnounceRequestError {
    fn to_string(&self) -> String {
        match self {
            AnnounceRequestError::InvalidInfoHash => "Invalid info_hash".to_string(),
            AnnounceRequestError::InvalidPeerId => "Invalid peer_id".to_string(),
            AnnounceRequestError::InvalidPort => "Invalid port".to_string(),
            AnnounceRequestError::InvalidUploaded => "Invalid uploaded".to_string(),
            AnnounceRequestError::InvalidDownloaded => "Invalid downloaded".to_string(),
            AnnounceRequestError::InvalidLeft => "Invalid left".to_string(),
            AnnounceRequestError::InvalidIp => "Invalid ip".to_string(),
            AnnounceRequestError::InvalidNumwant => "Invalid numwant".to_string(),
            AnnounceRequestError::InvalidKey => "Invalid key".to_string(),
            AnnounceRequestError::InvalidTrackerId => "Invalid tracker_id".to_string(),
            AnnounceRequestError::InvalidEvent => "Invalid event".to_string(),
        }
    }
}
