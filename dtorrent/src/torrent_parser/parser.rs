use std::{
    fs::File,
    io::{BufReader, Error, Read},
};

use super::torrent::{FromTorrentError, Torrent};
use bencoder::bencode::{Bencode, BencodeError};

#[derive(Debug)]
pub enum ParseError {
    IoError(Error),
    BencodeError(BencodeError),
    FromTorrentError(FromTorrentError),
}

pub struct TorrentParser;

impl TorrentParser {
    /// Given a path to a torrent file, it parses the file and returns a Torrent struct.
    ///
    /// # Errors
    ///
    /// * `ParseError::IoError` - An error occurred while reading the file
    /// * `ParseError::BencodeError` - An error occurred while parsing the bencode
    /// * `ParseError::FromTorrentError` - An error occurred while creating the Torrent struct
    pub fn parse(filepath: String) -> Result<Torrent, ParseError> {
        let buffer = match TorrentParser::read_file(filepath) {
            Ok(buffer) => buffer,
            Err(e) => return Err(ParseError::IoError(e)),
        };

        let bencode = match Bencode::decode(&buffer) {
            Ok(bencode) => bencode,
            Err(e) => return Err(ParseError::BencodeError(e)),
        };

        let torrent = match Torrent::from(bencode) {
            Ok(torrent) => torrent,
            Err(e) => return Err(ParseError::FromTorrentError(e)),
        };

        Ok(torrent)
    }

    fn read_file(filepath: String) -> Result<Vec<u8>, Error> {
        let file = File::open(filepath)?;
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();

        reader.read_to_end(&mut buffer)?;

        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Write};

    #[test]
    fn test_parse_torrent() {
        let filepath = "./test_parse_torrent.torrent";
        let contents =
            b"d8:announce35:https://torrent.ubuntu.com/announce4:infod6:lengthi3654957056e4:name30:ubuntu-22.04-desktop-amd64.iso12:piece lengthi262144e6:pieces64:<hex>BC 07 C0 6A 9D BC 07 C0 6A 9D BC 07 C0 6A 9D BC 07 C0 6A 9Dee";
        create_and_write_file(filepath, contents);

        let torrent = match TorrentParser::parse(filepath.to_string()) {
            Ok(torrent) => torrent,
            Err(e) => {
                remove_file(filepath);
                panic!("{:?}", e);
            }
        };

        assert_eq!(torrent.announce_url, "https://torrent.ubuntu.com/announce",);
        assert_eq!(torrent.info.length, 3654957056);
        assert_eq!(torrent.info.name, "ubuntu-22.04-desktop-amd64.iso");
        assert_eq!(torrent.info.piece_length, 262144);
        assert_eq!(
            torrent.info_hash,
            "48442ddee1900ed8c8101bb8b2bd955060f1eabc"
        );
        remove_file(filepath);
    }

    fn create_and_write_file(path: &str, contents: &[u8]) {
        let mut file = File::create(path).unwrap();
        file.write_all(contents).unwrap();
    }

    fn remove_file(path: &str) {
        fs::remove_file(path).unwrap();
    }
}
