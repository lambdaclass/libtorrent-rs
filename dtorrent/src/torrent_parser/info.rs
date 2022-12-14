use std::collections::BTreeMap;

use bencoder::bencode::{Bencode, ToBencode};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Info {
    pub length: i64,
    pub name: String,
    pub piece_length: i64,
    pub pieces: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub enum FromInfoError {
    MissingLength,
    MissingName,
    MissingPieceLength,
    MissingPieces,
    NotADict,
    MultipleFilesNotSupported,
}

impl Info {
    pub fn from(bencode: &Bencode) -> Result<Info, FromInfoError> {
        let mut name = String::new();
        let mut length = 0;
        let mut piece_length = 0;
        let mut pieces = Vec::new();

        let d = match bencode {
            Bencode::BDict(s) => s,
            _ => return Err(FromInfoError::NotADict),
        };

        for (k, v) in d.iter() {
            if k == b"name" {
                name = Info::create_name(v)?;
            } else if k == b"length" {
                length = Info::create_length(v)?;
            } else if k == b"piece length" {
                piece_length = Info::create_piece_length(v)?;
            } else if k == b"pieces" {
                pieces = Info::create_pieces(v)?;
            } else if k == b"files" {
                return Err(FromInfoError::MultipleFilesNotSupported);
            }
        }

        Ok(Info {
            length,
            name,
            piece_length,
            pieces,
        })
    }

    fn create_name(bencode: &Bencode) -> Result<String, FromInfoError> {
        let c = match bencode {
            &Bencode::BString(ref s) => s,
            _ => return Err(FromInfoError::MissingName),
        };

        let name = match String::from_utf8(c.to_vec()) {
            Ok(s) => s,
            Err(_) => return Err(FromInfoError::MissingName),
        };

        Ok(name)
    }

    fn create_length(bencode: &Bencode) -> Result<i64, FromInfoError> {
        let c = match bencode {
            &Bencode::BNumber(ref s) => s,
            _ => return Err(FromInfoError::MissingLength),
        };
        Ok(*c)
    }

    fn create_piece_length(bencode: &Bencode) -> Result<i64, FromInfoError> {
        let c = match bencode {
            &Bencode::BNumber(ref s) => s,
            _ => return Err(FromInfoError::MissingPieceLength),
        };
        Ok(*c)
    }

    fn create_pieces(bencode: &Bencode) -> Result<Vec<u8>, FromInfoError> {
        let c = match bencode {
            &Bencode::BString(ref s) => s,
            _ => return Err(FromInfoError::MissingPieces),
        };
        Ok(c.to_vec())
    }
}

impl ToBencode for Info {
    fn to_bencode(&self) -> Bencode {
        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), self.length.to_bencode());
        info.insert(b"name".to_vec(), self.name.to_bencode());
        info.insert(b"piece length".to_vec(), self.piece_length.to_bencode());
        info.insert(b"pieces".to_vec(), self.pieces.to_bencode());
        Bencode::BDict(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_info_empty() {
        let bencode = Bencode::BDict(BTreeMap::new());
        let info = Info::from(&bencode).unwrap();
        assert_eq!(info.length, 0);
        assert_eq!(info.name, String::new());
        assert_eq!(info.piece_length, 0);
        assert_eq!(info.pieces, Vec::new());
    }

    #[test]
    fn test_from_info_full() {
        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), Bencode::BNumber(1));
        info.insert(b"name".to_vec(), Bencode::BString(b"test1".to_vec()));
        info.insert(b"piece length".to_vec(), Bencode::BNumber(2));
        info.insert(b"pieces".to_vec(), Bencode::BString(b"test2".to_vec()));
        let bencode = Bencode::BDict(info);

        let response = Info::from(&bencode).unwrap();
        assert_eq!(response.length, 1);
        assert_eq!(response.name, "test1");
        assert_eq!(response.piece_length, 2);
        assert_eq!(response.pieces, b"test2");
    }

    #[test]
    fn test_from_info_with_multiple_files() {
        let mut info = BTreeMap::new();
        info.insert(b"name".to_vec(), Bencode::BString(b"test1".to_vec()));
        info.insert(b"piece length".to_vec(), Bencode::BNumber(2));
        info.insert(b"pieces".to_vec(), Bencode::BString(b"test2".to_vec()));
        info.insert(b"files".to_vec(), Bencode::BList(vec![]));
        let bencode = Bencode::BDict(info);

        let response = Info::from(&bencode).unwrap_err();
        assert_eq!(response, FromInfoError::MultipleFilesNotSupported);
    }
}
