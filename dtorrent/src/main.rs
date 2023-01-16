use std::{io, sync::Arc};
use std::collections::HashMap;
use bencoder::bencode::Bencode;
use dtorrent::{bt_server::server::BtServer, config::cfg::Cfg, torrent_parser::{torrent::Torrent, parser::TorrentParser}, torrent_handler::status::AtomicTorrentStatus};
use logger::logger_receiver::Logger;

fn main() {  
    
    let parsed = TorrentParser::parse("/Users/marpinpar/Desktop/mpp/libtorrent-rs/torrents/testFile.torrent".to_string()).unwrap();
    let config = Cfg::new("/Users/marpinpar/Desktop/mpp/libtorrent-rs/dtorrent/config.cfg").unwrap(); //Test path
    let (status,status_reciever) = AtomicTorrentStatus::new(&parsed,config.clone());

    let mut torrent_with_status = HashMap::new();
    torrent_with_status.insert(parsed,Arc::new(status)); 

    let logger = Logger::new(&config.log_directory, config.max_log_file_kb_size).unwrap();
    let client_peer_id = "client_peer_id".to_string(); 
    let mut server = BtServer::new(torrent_with_status, config, logger.new_sender(), client_peer_id);
    server.init();
}