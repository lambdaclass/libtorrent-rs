# libtorrent-rs

A Rust implementation of the [BitTorrent V2] (http://bittorrent.org/beps/bep_0052.html) protocol. For now only V1 is implemented but we're working on V2.

## Dependencies
- Rust
- Cargo

## Running
To run the program there needs to be a `config.cfg` file in the `configs` directory of the project. We provide two with default values as an example.
Then run the program with `cargo` followed by the directory and name containing the .torrent file, and the directory and name of the config file as shown below:
```bash
$ cargo run --bin dtorrent -- --file ./torrents/file_name --config ./configs/config_file
```

## Testing
Run the test suite:
```bash
make test
```

## BitTorrent Specification

- [Index of BitTorrent Enhancement Proposals](http://bittorrent.org/beps/bep_0000.html)
- [The BitTorrent Protocol Specification v2](http://bittorrent.org/beps/bep_0052.html)
- [DHT Protocol](http://bittorrent.org/beps/bep_0005.html)
