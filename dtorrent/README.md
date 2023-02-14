## Running
To run the program there needs to be a `config.cfg` file in the `configs` directory of the project. We provide two with default values as an example.
Then run the program with `cargo` followed by the directory containing the `.torrent` files, and the directory and name of the `.cfg` file as shown below:
```bash
$ cargo run --bin dtorrent -- --file ./torrents/file_name --config ./configs/config_file
```
On startup the client gets all the .torrent files on the specified directory and immediately starts the download & upload.

## Tests
Run tests with `cargo`:
```bash
$ cargo test --package dtorrent
```