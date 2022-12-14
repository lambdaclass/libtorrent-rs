use bencoder::bencode::Bencode;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;

use super::handshake::Handshake;

/// `BtPeer` struct containing individual BtPeer information.
///
/// To create a new `BtPeer` use the method builder `from()`.
#[derive(Debug, Clone)]
pub struct BtPeer {
    pub peer_id: Option<Vec<u8>>,
    pub ip: String,
    pub port: i64,
    pub info_hash: Option<Vec<u8>>,
}

impl PartialEq for BtPeer {
    fn eq(&self, other: &Self) -> bool {
        self.ip == other.ip && self.port == other.port
    }
}

impl Eq for BtPeer {}

impl std::hash::Hash for BtPeer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ip.hash(state);
        self.port.hash(state);
    }
}

/// Posible `BtPeer` errors
#[derive(Debug)]
pub enum BtPeerError {
    InvalidPeerId,
    InvalidIp,
    InvalidPort,
    NotADict,
    HandshakeError,
}

impl BtPeer {
    /// Builds a new `BtPeer` decoding a bencoded Vec<u8> cointaining the BtPeer information.
    pub fn new(ip: String, port: i64) -> Self {
        Self {
            peer_id: None,
            ip,
            port,
            info_hash: None,
        }
    }

    /// Builds a new `BtPeer` from a bencoded peer from the tracker response peer list.
    ///
    ///
    /// It returns an `BtPeerError` if:
    /// - The peer ID is invalid.
    /// - The peer IP is invalid.
    /// - The peer Port is invalid.
    /// - The bencoded peer is not a Dict.
    pub fn from(bencode: Bencode) -> Result<BtPeer, BtPeerError> {
        let mut peer_id: Vec<u8> = Vec::new();
        let mut ip: String = String::new();
        let mut port: i64 = 0;

        let d = match bencode {
            Bencode::BDict(d) => d,
            _ => return Err(BtPeerError::NotADict),
        };

        for (k, v) in d.iter() {
            if k == b"peer id" {
                peer_id = Self::create_peer_id(v)?;
            } else if k == b"ip" {
                ip = Self::create_ip(v)?;
            } else if k == b"port" {
                port = Self::create_port(v)?;
            }
        }

        Ok(BtPeer {
            peer_id: Some(peer_id),
            ip,
            port,
            info_hash: None,
        })
    }

    fn create_peer_id(bencode: &Bencode) -> Result<Vec<u8>, BtPeerError> {
        let peer_id = match bencode {
            Bencode::BString(s) => s.clone(),
            _ => return Err(BtPeerError::InvalidPeerId),
        };

        Ok(peer_id)
    }

    fn create_ip(bencode: &Bencode) -> Result<String, BtPeerError> {
        let ip = match bencode {
            Bencode::BString(s) => s,
            _ => return Err(BtPeerError::InvalidIp),
        };

        let ip = match String::from_utf8(ip.to_vec()) {
            Ok(s) => s,
            Err(_) => return Err(BtPeerError::InvalidIp),
        };

        Ok(ip)
    }

    fn create_port(bencode: &Bencode) -> Result<i64, BtPeerError> {
        let port = match bencode {
            Bencode::BNumber(n) => *n,
            _ => return Err(BtPeerError::InvalidPort),
        };

        Ok(port)
    }

    /// Reads a handshake from the peer and returns the info hash.
    ///
    /// It returns an error if the handshake could not be read or the handshake was not successful.
    pub fn receive_handshake(&mut self, stream: &mut TcpStream) -> Result<Vec<u8>, BtPeerError> {
        let mut buffer = [0; 68];
        stream
            .read_exact(&mut buffer)
            .map_err(|_| BtPeerError::HandshakeError)?;

        let handshake = Handshake::from_bytes(&buffer).map_err(|_| BtPeerError::HandshakeError)?;

        self.info_hash = Some(handshake.info_hash.clone());
        self.peer_id = Some(handshake.peer_id);

        Ok(handshake.info_hash)
    }

    /// Sends a handshake to the peer.
    ///
    /// It returns an error if the handshake could not be sent or the handshake was not successful.
    pub fn send_handshake(
        &mut self,
        stream: &mut TcpStream,
        info_hash: Vec<u8>,
        client_peer_id: String,
    ) -> Result<(), BtPeerError> {
        let handshake = Handshake::new(info_hash, client_peer_id.as_bytes().to_vec());
        stream
            .write_all(&handshake.as_bytes())
            .map_err(|_| BtPeerError::HandshakeError)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_from_bt_peer() {
        let mut dict = BTreeMap::new();
        dict.insert(b"peer id".to_vec(), Bencode::BString(b"peer id".to_vec()));
        dict.insert(b"ip".to_vec(), Bencode::BString(b"127.0.0.1".to_vec()));
        dict.insert(b"port".to_vec(), Bencode::BNumber(6868));

        let bencode = Bencode::BDict(dict);

        let bt_peer = BtPeer::from(bencode).unwrap();

        assert_eq!(bt_peer.peer_id, Some(b"peer id".to_vec()));
        assert_eq!(bt_peer.ip, "127.0.0.1");
        assert_eq!(bt_peer.port, 6868);
    }

    #[test]
    fn test_new_peer() {
        let bt_peer = BtPeer::new("127.0.0.1".to_string(), 6868);

        assert_eq!(bt_peer.peer_id, None);
        assert_eq!(bt_peer.ip, "127.0.0.1");
        assert_eq!(bt_peer.port, 6868);
    }
}
