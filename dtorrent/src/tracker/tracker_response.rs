use bencoder::bencode::{Bencode, BencodeError};

use crate::peer::bt_peer::{BtPeer, BtPeerError};

/// `TrackerResponse` struct containing a tracker response.
///
/// To create a new `TrackerResponse` use the method builder `from()`.
#[derive(Debug)]
pub struct TrackerResponse {
    pub interval: i64,
    pub complete: i64,
    pub incomplete: i64,
    pub peers: Vec<BtPeer>,
}

/// Posible `TrackerResponse` errors.
#[derive(Debug)]
pub enum FromTrackerResponseError {
    DecodeResponseError(BencodeError),
    InvalidInterval,
    InvalidComplete,
    InvalidIncomplete,
    InvalidPeers(BtPeerError),
    NotADict,
    NotAList,
}

impl TrackerResponse {
    /// Builds a new `TrackerResponse` decoding a bencoded Vec<u8> cointaining the tracker's response.
    ///
    /// It returns an `FromTrackerResponseError` if:
    /// - There was a problem decoding the parser response.
    /// - The bencoded response is not a dict.
    /// - The bencoded peers are not a list.
    /// - The tracker response interval is invalid.
    /// - The tracker response complete is invalid.
    /// - The tracker response incomplete is invalid.
    /// - The tracker response peers are invalid.
    pub fn from(response: Vec<u8>) -> Result<TrackerResponse, FromTrackerResponseError> {
        let mut interval = 0;
        let mut complete = 0;
        let mut incomplete = 0;
        let mut peers = Vec::new();

        let decoded_res = match Bencode::decode(&response) {
            Ok(decoded_res) => decoded_res,
            Err(err) => return Err(FromTrackerResponseError::DecodeResponseError(err)),
        };

        let d = match decoded_res {
            Bencode::BDict(d) => d,
            _ => return Err(FromTrackerResponseError::NotADict),
        };

        for (k, v) in d.iter() {
            if k == b"interval" {
                interval = Self::create_interval(v)?;
            } else if k == b"complete" {
                complete = Self::create_complete(v)?;
            } else if k == b"incomplete" {
                incomplete = Self::create_incomplete(v)?;
            } else if k == b"peers" {
                peers = Self::create_peers(v)?;
            }
        }

        Ok(TrackerResponse {
            interval,
            complete,
            incomplete,
            peers,
        })
    }

    fn create_interval(bencode: &Bencode) -> Result<i64, FromTrackerResponseError> {
        let interval = match bencode {
            Bencode::BNumber(n) => *n,
            _ => return Err(FromTrackerResponseError::InvalidInterval),
        };

        Ok(interval)
    }

    fn create_complete(bencode: &Bencode) -> Result<i64, FromTrackerResponseError> {
        let complete = match bencode {
            Bencode::BNumber(n) => *n,
            _ => return Err(FromTrackerResponseError::InvalidComplete),
        };

        Ok(complete)
    }

    fn create_incomplete(bencode: &Bencode) -> Result<i64, FromTrackerResponseError> {
        let incomplete = match bencode {
            Bencode::BNumber(n) => *n,
            _ => return Err(FromTrackerResponseError::InvalidIncomplete),
        };

        Ok(incomplete)
    }

    fn create_peers(bencode: &Bencode) -> Result<Vec<BtPeer>, FromTrackerResponseError> {
        match bencode {
            Bencode::BList(list) => Self::create_peers_from_dict(list),
            Bencode::BString(str) => Self::create_peers_from_bstring(str),
            _ => Err(FromTrackerResponseError::NotAList),
        }
    }

    fn create_peers_from_dict(list: &[Bencode]) -> Result<Vec<BtPeer>, FromTrackerResponseError> {
        let mut peers = Vec::new();

        for p in list {
            let peer = BtPeer::from(p.clone()).map_err(FromTrackerResponseError::InvalidPeers)?;
            peers.push(peer);
        }

        Ok(peers)
    }

    fn create_peers_from_bstring(bstring: &[u8]) -> Result<Vec<BtPeer>, FromTrackerResponseError> {
        Ok(bstring
            .chunks(6)
            .map(|chunk| {
                let ip = format!("{}.{}.{}.{}", chunk[0], chunk[1], chunk[2], chunk[3]);
                let port = u16::from_be_bytes([chunk[4], chunk[5]]) as i64;

                BtPeer::new(ip, port)
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_from_tracker_response() {
        let peer_dict = build_peer_dict(b"id1".to_vec(), b"127.0.0.1".to_vec(), 6868);
        let peer_dict2 = build_peer_dict(b"id2".to_vec(), b"127.0.0.2".to_vec(), 4242);

        let peers_list = vec![Bencode::BDict(peer_dict), Bencode::BDict(peer_dict2)];

        let mut dict = BTreeMap::new();
        dict.insert(b"interval".to_vec(), Bencode::BNumber(10));
        dict.insert(b"complete".to_vec(), Bencode::BNumber(10));
        dict.insert(b"incomplete".to_vec(), Bencode::BNumber(10));
        dict.insert(b"peers".to_vec(), Bencode::BList(peers_list));

        let response = Bencode::encode(&dict);
        let response_decoded = TrackerResponse::from(response).unwrap();

        assert_eq!(response_decoded.interval, 10);
        assert_eq!(response_decoded.complete, 10);
        assert_eq!(response_decoded.incomplete, 10);
        assert_eq!(response_decoded.peers.len(), 2);
    }

    fn build_peer_dict(peer_id: Vec<u8>, ip: Vec<u8>, port: i64) -> BTreeMap<Vec<u8>, Bencode> {
        let mut peer_dict = BTreeMap::new();
        peer_dict.insert(b"peer id".to_vec(), Bencode::BString(peer_id));
        peer_dict.insert(b"ip".to_vec(), Bencode::BString(ip));
        peer_dict.insert(b"port".to_vec(), Bencode::BNumber(port));
        peer_dict
    }
}
