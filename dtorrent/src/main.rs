use dtorrent::{
    bt_server::server::BtServer, config::cfg::Cfg, torrent_handler::status::AtomicTorrentStatus,
    torrent_parser::parser::TorrentParser,
};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() {
    // Reads the filepath from the command line argument (Check README)
    let mut arg = env::args();
    let path = PathBuf::from((arg.nth(1).expect("Failed to retrieve file path")).trim());

    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    // Initializes the server
    let parsed = TorrentParser::parse(&path).expect("parser could not find the file");
    let config = Cfg::new("./dtorrent/config.cfg").expect("config file not found");
    let (status, _status_reciever) = AtomicTorrentStatus::new(&parsed, config.clone());
    let mut torrent_with_status = HashMap::new();
    torrent_with_status.insert(parsed, Arc::new(status));
    let client_peer_id = "client_peer_id".to_string();
    let mut server = BtServer::new(torrent_with_status, config, client_peer_id);
    info!("Initializing server ...");
    server.init().expect("Failed to initialize server");
}
