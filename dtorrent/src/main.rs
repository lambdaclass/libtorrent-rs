use dtorrent::ui::setup;
use gtk::gio::ApplicationFlags;
use gtk::prelude::*;
use gtk::Application;
use std::env;

fn main() {
    if env::args().count() != 2 {
        return eprintln!("Incorrect number of arguments. Only a directory path containing one or more torrents should be passed");
    };
    let torrents_directory = env::args().last().unwrap();

    let app = Application::builder()
        .application_id("bittorrent")
        .flags(ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_open(move |app, _file, _some_str| {
        let _ = setup::start_dtorrent_application(app, torrents_directory.clone());
    });

    app.run();
}
