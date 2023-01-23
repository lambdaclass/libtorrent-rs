use clap::Parser;
use dtorrent::{
    bt_server::server::BtServer, config::cfg::Cfg, torrent_handler::status::AtomicTorrentStatus,
    torrent_parser::parser::TorrentParser,
};
use logger::logger_receiver::Logger;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    file: String,
    #[arg(short, long)]
    config: String,
}

fn main() {
    // Reads the filepath from the command line argument (Check README)
    let args = Args::parse();
    let file_path = PathBuf::from(args.file.trim());
    let config_path = args.config.trim();

    // Initializes the server
    let parsed = TorrentParser::parse(&file_path).expect("parser could not find the file");
    let config = Cfg::new(&config_path).expect("Config file not found or incomplete");
    let (status, _status_reciever) = AtomicTorrentStatus::new(&parsed, config.clone());
    let mut torrent_with_status = HashMap::new();
    torrent_with_status.insert(parsed, Arc::new(status));
    let logger = Logger::new(&config.log_directory, config.max_log_file_kb_size)
        .expect("logger could not be created");
    let client_peer_id = "client_peer_id".to_string();
    let mut server = BtServer::new(
        torrent_with_status,
        config,
        logger.new_sender(),
        client_peer_id,
    );
    println!("Initializing server ...");
    server.init().expect("Failed to initialize server");
}
