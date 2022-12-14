#[derive(Debug)]
pub enum FromHandshakeError {
    InvalidHandshake,
}

/// Represents a handshake message.
/// Is the first message sent to start a connection with a peer.
#[derive(Debug)]
pub struct Handshake {
    pub pstrlen: u8,
    pub pstr: String,
    pub reserved: [u8; 8],
    pub info_hash: Vec<u8>,
    pub peer_id: Vec<u8>,
}

const PSTR: &str = "BitTorrent protocol";

impl Handshake {
    /// Creates a new `Handshake` message.
    pub fn new(info_hash: Vec<u8>, peer_id: Vec<u8>) -> Self {
        Self {
            pstrlen: 19,
            pstr: PSTR.to_string(),
            reserved: [0; 8],
            info_hash,
            peer_id,
        }
    }

    /// Converts a `Handshake` message to a byte array.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![self.pstrlen];
        bytes.extend(self.pstr.as_bytes());
        bytes.extend(&self.reserved);
        bytes.extend(&self.info_hash);
        bytes.extend(&self.peer_id);
        bytes
    }

    /// Parses a byte array into a `Handshake` message.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FromHandshakeError> {
        if bytes.len() != 68 {
            return Err(FromHandshakeError::InvalidHandshake);
        }

        let pstrlen = bytes[0];
        if pstrlen != 19 {
            return Err(FromHandshakeError::InvalidHandshake);
        }

        let pstr = String::from_utf8(bytes[1..pstrlen as usize + 1].to_vec())
            .map_err(|_| FromHandshakeError::InvalidHandshake)?;
        let reserved = &bytes[pstrlen as usize + 1..pstrlen as usize + 9];
        let info_hash = &bytes[pstrlen as usize + 9..pstrlen as usize + 29];
        let peer_id = &bytes[pstrlen as usize + 29..];

        Ok(Self {
            pstrlen,
            pstr,
            reserved: [
                reserved[0],
                reserved[1],
                reserved[2],
                reserved[3],
                reserved[4],
                reserved[5],
                reserved[6],
                reserved[7],
            ],
            info_hash: info_hash.to_vec(),
            peer_id: peer_id.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_bytes() {
        let expected_handshake_len = 68;
        let expected_pstrlen = 19;
        let expected_pstr = b"BitTorrent protocol".to_vec();
        let expected_reserved = [0; 8];

        let info_hash: Vec<u8> = (1..=20).collect();
        let peer_id: Vec<u8> = (21..=40).collect();
        let handshake = Handshake::new(info_hash.clone(), peer_id.clone());

        let bytes = handshake.as_bytes();

        assert_eq!(bytes.len(), expected_handshake_len);
        assert_eq!(bytes[0], expected_pstrlen);
        assert_eq!(bytes[1..20], expected_pstr);
        assert_eq!(bytes[20..28], expected_reserved);
        assert_eq!(bytes[28..48], info_hash);
        assert_eq!(bytes[48..], peer_id);
    }

    #[test]
    fn test_from_bytes() {
        let expected_pstrlen = 19;
        let expected_pstr = "BitTorrent protocol";
        let expected_reserved = [0; 8];

        let info_hash: Vec<u8> = (1..=20).collect();
        let peer_id: Vec<u8> = (21..=40).collect();
        let handshake = Handshake::new(info_hash.clone(), peer_id.clone());
        let bytes = handshake.as_bytes();

        let handshake = Handshake::from_bytes(&bytes).unwrap();

        assert_eq!(handshake.pstrlen, expected_pstrlen);
        assert_eq!(handshake.pstr, expected_pstr);
        assert_eq!(handshake.reserved, expected_reserved);
        assert_eq!(handshake.info_hash, info_hash);
        assert_eq!(handshake.peer_id, peer_id);
    }
}
