use std::env;
use tracing::error;
use dtracker::bt_tracker::tracker::BtTracker;

fn main() {
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
