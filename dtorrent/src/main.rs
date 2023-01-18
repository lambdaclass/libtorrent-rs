use std::{env};
use std::path::{PathBuf};
use std::{sync::Arc};
use std::collections::HashMap;
use dtorrent::{bt_server::server::BtServer, config::cfg::Cfg, torrent_parser::{parser::TorrentParser}, torrent_handler::status::AtomicTorrentStatus};
use logger::logger_receiver::Logger;

fn main() {  
    // Reads the filepath from the command line argument (Check README)
    let mut arg = env::args();
    let _ = arg.next().expect("Failed to read command line arguments");
    let path = arg.next().expect("Failed to retrieve file path");
    let path = PathBuf::from(path.trim());
  
    // Initializes the server
    let parsed = TorrentParser::parse(&path).expect("parser could not find the file");
    let config = Cfg::new("./dtorrent/config.cfg").expect("config file not found");
    let (status,status_reciever) = AtomicTorrentStatus::new(&parsed,config.clone());
    let mut torrent_with_status = HashMap::new();
    torrent_with_status.insert(parsed,Arc::new(status)); 
    let logger = Logger::new(&config.log_directory, config.max_log_file_kb_size).expect("logger could not be created");
    let client_peer_id = "client_peer_id".to_string(); 
    let mut server = BtServer::new(torrent_with_status, config, logger.new_sender(), client_peer_id); 
    println!("Initializing server...");
    server.init().expect("Failed to initialize server");

}

// -- Another posibility to get the file path is to let the user type it: --
    //let mut user_input = String::new();
    //let stdin = io::stdin(); 
    //println!("Please enter the path of the file:");
    //stdin.read_line(&mut user_input).expect("Invalid input");
    //let path = PathBuf::from(user_input.trim()); // Path must look like this: "./torrents/file_name"