## Running

To run the program there needs to be a `config.cfg` file in the root of the project. We provide one with default values as an example.

Then run the program with `cargo` followed by the directory containing the .torrent files:

```bash
$ cargo run --bin dtorrent ./torrents/file1.torrent
```

On startup the client gets all the .torrent files on the specified directory and immediately starts the download & upload.

## Tests

Run tests with `cargo`:

```bash
$ cargo test --package dtorrent
```
