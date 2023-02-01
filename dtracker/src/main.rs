use dtracker::bt_tracker::tracker::BtTracker;
use std::env;
use tracing::error;

fn main() {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    if env::args().count() != 2 {
        return error!("Incorrect number of arguments. Only a port number should be passed");
    };
    let port = match env::args().last().unwrap() {
        s if s.parse::<u16>().is_ok() => s.parse::<u16>().unwrap(),
        _ => return error!("Invalid port number"),
    };

    match BtTracker::init(port) {
        Ok(tracker) => match tracker.run() {
            Ok(_) => (),
            Err(e) => error!("Error: {:?}", e),
        },
        Err(error) => {
            error!("Error: {:?}", error);
        }
    }
}
