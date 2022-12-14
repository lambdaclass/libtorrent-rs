use std::fmt::Write;
use std::{collections::BTreeMap, num::ParseIntError};

use sha1::{Digest, Sha1};

use bencoder::bencode::{Bencode, ToBencode};

use super::info::{FromInfoError, Info};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Torrent {
    pub announce_url: String,
    pub info: Info,
    pub info_hash: String,
}

#[derive(Debug, PartialEq)]
pub enum FromTorrentError {
    MissingAnnounce,
    MissingInfo,
    FromInfoError(FromInfoError),
    InfoHashError,
    NotADict,
}

impl Torrent {
    pub fn from(bencode: Bencode) -> Result<Torrent, FromTorrentError> {
        let mut announce_url = String::new();
        let mut info: Option<Info> = None;

        let d = match bencode {
            Bencode::BDict(s) => s,
            _ => return Err(FromTorrentError::NotADict),
        };

        for (k, v) in d.iter() {
            if k == b"announce" {
                announce_url = Torrent::create_announce(v)?;
            } else if k == b"info" {
                info = Some(Torrent::create_info(v)?);
            }
        }

        if announce_url.is_empty() {
            return Err(FromTorrentError::MissingAnnounce);
        }

        let info = match info {
            Some(x) => x,
            None => return Err(FromTorrentError::MissingInfo),
        };

        let info_hash = Torrent::create_info_hash(&info)?;

        Ok(Torrent {
            announce_url,
            info,
            info_hash,
        })
    }

    fn create_announce(bencode: &Bencode) -> Result<String, FromTorrentError> {
        let announce_url = match bencode {
            Bencode::BString(s) => s,
            _ => return Err(FromTorrentError::MissingAnnounce),
        };

        let announce_url = match String::from_utf8(announce_url.to_vec()) {
            Ok(s) => s,
            Err(_) => return Err(FromTorrentError::MissingAnnounce),
        };

        Ok(announce_url)
    }

    fn create_info(bencode: &Bencode) -> Result<Info, FromTorrentError> {
        let info = match Info::from(bencode) {
            Ok(x) => x,
            Err(err) => return Err(FromTorrentError::FromInfoError(err)),
        };

        Ok(info)
    }

    pub fn create_info_hash(info: &Info) -> Result<String, FromTorrentError> {
        let bencoded_info = Bencode::encode(info);
        let hash = Sha1::digest(bencoded_info);

        let mut hex_string = String::with_capacity(hash.len() * 2);

        for b in hash {
            match write!(&mut hex_string, "{:02x}", b) {
                Ok(_) => (),
                Err(_) => return Err(FromTorrentError::InfoHashError),
            }
        }

        Ok(hex_string)
    }

    /// Returns the info hash of the torrent as a byte array.
    pub fn get_info_hash_as_bytes(&self) -> Result<Vec<u8>, ParseIntError> {
        Self::decode_hex(self.info_hash.as_str())
    }

    fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect()
    }

    /// Returns the name of the torrent.
    pub fn name(&self) -> String {
        self.info.name.clone()
    }

    /// Returns the size of pieces of the torrent.
    pub fn piece_length(&self) -> u32 {
        self.info.piece_length as u32
    }

    /// Returns the length in bytes of the torrent.
    pub fn length(&self) -> u32 {
        self.info.length as u32
    }

    /// Returns the number of pieces of the torrent.
    pub fn total_pieces(&self) -> u32 {
        (self.info.length as f64 / self.info.piece_length as f64).ceil() as u32
    }

    /// Returns the size of the last piece of the torrent.
    pub fn last_piece_size(&self) -> u32 {
        self.info.length as u32 % self.info.piece_length as u32
    }

    pub fn info_hash(&self) -> String {
        self.info_hash.clone()
    }
}

impl ToBencode for Torrent {
    fn to_bencode(&self) -> Bencode {
        let mut m = BTreeMap::new();
        m.insert(b"announce_url".to_vec(), self.announce_url.to_bencode());
        m.insert(b"info".to_vec(), self.info.to_bencode());
        Bencode::BDict(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_torrent_full() {
        let announce = String::from("http://example.com/announce");
        let info_len = 10;
        let info_name = String::from("example");
        let info_piece_len = 20;
        let info_pieces = String::from("test").into_bytes();

        let info_bencode = build_info_bencode(
            info_len,
            info_name.clone().into_bytes(),
            info_piece_len,
            info_pieces.clone(),
        );
        let torrent_bencode =
            build_torrent_bencode(announce.clone().into_bytes(), info_bencode.clone());

        let info = Info::from(&Bencode::BDict(info_bencode)).unwrap();
        let info_hash = Torrent::create_info_hash(&info).unwrap();

        let torrent = Torrent::from(torrent_bencode).unwrap();

        assert_eq!(torrent.announce_url, announce);
        assert_eq!(torrent.info.length, info_len);
        assert_eq!(torrent.info.name, info_name);
        assert_eq!(torrent.info.piece_length, info_piece_len);
        assert_eq!(torrent.info.pieces, info_pieces);
        assert_eq!(torrent.info_hash, info_hash);
    }

    #[test]
    fn test_from_torrent_empty() {
        let torrent_bencode = Bencode::BDict(BTreeMap::new());

        let actual_err = Torrent::from(torrent_bencode).unwrap_err();
        let expected_err = FromTorrentError::MissingAnnounce;

        assert_eq!(actual_err, expected_err);
    }

    #[test]
    fn test_from_torrent_missing_announce() {
        let mut m = BTreeMap::new();
        m.insert(b"info".to_vec(), Bencode::BDict(BTreeMap::new()));
        let torrent_bencode = Bencode::BDict(m);

        let actual_err = Torrent::from(torrent_bencode).unwrap_err();
        let expected_err = FromTorrentError::MissingAnnounce;

        assert_eq!(actual_err, expected_err);
    }

    #[test]
    fn test_from_torrent_missing_info() {
        let announce = String::from("http://example.com/announce").into_bytes();
        let mut m = BTreeMap::new();
        m.insert(b"announce".to_vec(), Bencode::BString(announce));
        let torrent_bencode = Bencode::BDict(m);

        let actual_err = Torrent::from(torrent_bencode).unwrap_err();
        let expected_err = FromTorrentError::MissingInfo;

        assert_eq!(actual_err, expected_err);
    }

    #[test]
    fn test_from_torrent_not_a_dict() {
        let torrent_bencode = Bencode::BString(String::from("test").into_bytes());

        let actual_err = Torrent::from(torrent_bencode).unwrap_err();
        let expected_err = FromTorrentError::NotADict;

        assert_eq!(actual_err, expected_err);
    }

    #[test]
    fn test_get_info_hash_as_bytes() {
        let info_hash = String::from("2c6b6858d61da9543d4231a71db4b1c9264b0685");
        let info_hash_bytes = [
            44, 107, 104, 88, 214, 29, 169, 84, 61, 66, 49, 167, 29, 180, 177, 201, 38, 75, 6, 133,
        ];

        let torrent = Torrent {
            announce_url: String::from("http://example.com/announce"),
            info: Info {
                length: 10,
                name: String::from("example"),
                piece_length: 20,
                pieces: String::from("test").into_bytes(),
            },
            info_hash,
        };

        assert_eq!(torrent.get_info_hash_as_bytes().unwrap(), info_hash_bytes);
    }

    #[test]
    fn test_name() {
        let torrent = build_test_torrent();
        assert_eq!(torrent.name(), "example");
    }

    #[test]
    fn test_piece_length() {
        let torrent = build_test_torrent();
        assert_eq!(torrent.piece_length(), 10);
    }

    #[test]
    fn test_length() {
        let torrent = build_test_torrent();
        assert_eq!(torrent.length(), 105);
    }

    #[test]
    fn test_total_pieces() {
        let torrent = build_test_torrent();
        assert_eq!(torrent.total_pieces(), 11);
    }

    #[test]
    fn test_last_piece_size() {
        let torrent = build_test_torrent();
        assert_eq!(torrent.last_piece_size(), 5);
    }

    fn build_info_bencode(
        length: i64,
        name: Vec<u8>,
        pieces_len: i64,
        pieces: Vec<u8>,
    ) -> BTreeMap<Vec<u8>, Bencode> {
        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), Bencode::BNumber(length));
        info.insert(b"name".to_vec(), Bencode::BString(name));
        info.insert(b"piece length".to_vec(), Bencode::BNumber(pieces_len));
        info.insert(b"pieces".to_vec(), Bencode::BString(pieces));

        info
    }

    fn build_torrent_bencode(announce: Vec<u8>, info: BTreeMap<Vec<u8>, Bencode>) -> Bencode {
        let mut dict = BTreeMap::new();

        dict.insert(b"announce".to_vec(), Bencode::BString(announce));
        dict.insert(b"info".to_vec(), Bencode::BDict(info));

        Bencode::BDict(dict)
    }

    fn build_test_torrent() -> Torrent {
        Torrent {
            announce_url: String::from("http://example.com/announce"),
            info: Info {
                length: 105,
                name: String::from("example"),
                piece_length: 10,
                pieces: String::from("test").into_bytes(),
            },
            info_hash: "info_hash".to_string(),
        }
    }
}
