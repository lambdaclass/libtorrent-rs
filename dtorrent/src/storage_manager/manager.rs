use crate::config::cfg::Cfg;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

trait WriteWithOffset {
    fn write_all_at(&mut self, buf: &[u8], offset: u64) -> Result<(), std::io::Error>;
}

impl WriteWithOffset for File {
    fn write_all_at(&mut self, buf: &[u8], offset: u64) -> Result<(), std::io::Error> {
        self.seek(SeekFrom::Start(offset))?;
        self.write_all(buf)
    }
}

trait ReadWithOffset {
    fn read_exact_at(&mut self, buf: &mut [u8], offset: u64) -> Result<(), std::io::Error>;
}

impl ReadWithOffset for File {
    fn read_exact_at(&mut self, buf: &mut [u8], offset: u64) -> Result<(), std::io::Error> {
        self.seek(SeekFrom::Start(offset))?;
        self.read_exact(buf)
    }
}

pub fn save_piece(
    name: String,
    piece: &[u8],
    piece_offset: u64,
    config: Cfg,
) -> Result<(), std::io::Error> {
    let save_directory = config.download_directory;
    if !Path::new(&save_directory).exists() {
        fs::create_dir_all(save_directory.clone())?;
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(save_directory + "/" + &name)?;

    file.write_all_at(piece, piece_offset)?;

    Ok(())
}

/// Retrieves a block of data from a file at a given offset.
///
/// # Arguments
/// * `filename` - the name of the file to retrieve the data from.
/// * `offset` - integer specifying the offset in bytes from the start of the file
/// * `length` - integer specifying the requested length
/// * `config` - the configuration of the application
pub fn retrieve_block(
    filename: String,
    offset: u64,
    length: usize,
    config: Cfg,
) -> Result<Vec<u8>, std::io::Error> {
    let file_directory = config.download_directory;

    let mut file = OpenOptions::new()
        .read(true)
        .open(file_directory + "/" + &filename)?;

    let mut buffer = vec![0; length];
    file.read_exact_at(&mut buffer, offset)?;

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    use super::*;

    const CONFIG_PATH: &str = "config.cfg";

    #[test]
    fn retrieve_block_with_offset_zero() {
        let config = Cfg::new(CONFIG_PATH).unwrap();

        let filename = "test_retrieve_block_01.txt";
        let filepath = format!("{}/{}", config.download_directory, filename);
        let contents = "Hello, world!".as_bytes();
        create_and_write_file(&config, filepath.as_str(), contents);

        let offset = 0;
        let length = 5;

        let block = retrieve_block(String::from(filename), offset, length, config)
            .map_err(|err| {
                fs::remove_file(&filepath).unwrap();
                err
            })
            .unwrap();

        fs::remove_file(filepath).unwrap();

        assert_eq!(5, block.len());
        assert_eq!("Hello".as_bytes(), &block[..]);
    }

    #[test]
    fn retrieve_block_with_offset_in_the_middle() {
        let config = Cfg::new(CONFIG_PATH).unwrap();

        let filename = "test_retrieve_block_02.txt";
        let filepath = format!("{}/{}", config.download_directory, filename);
        let contents = "Hello, world!".as_bytes();
        create_and_write_file(&config, filepath.as_str(), contents);

        let offset = 4;
        let length = 7;

        let block = retrieve_block(String::from(filename), offset, length, config)
            .map_err(|err| {
                fs::remove_file(&filepath).unwrap();
                err
            })
            .unwrap();

        fs::remove_file(filepath).unwrap();

        assert_eq!(7, block.len());
        assert_eq!("o, worl".as_bytes(), &block[..]);
    }

    #[test]
    fn retrieve_block_with_offset_zero_and_length_equal_to_length_of_file() {
        let config = Cfg::new(CONFIG_PATH).unwrap();

        let filename = "test_retrieve_block_03.txt";
        let filepath = format!("{}/{}", config.download_directory, filename);
        let contents = "Hello, world!".as_bytes();
        create_and_write_file(&config, filepath.as_str(), contents);

        let offset = 0;
        let length = contents.len();

        let block = retrieve_block(String::from(filename), offset, length, config)
            .map_err(|err| {
                fs::remove_file(&filepath).unwrap();
                err
            })
            .unwrap();

        fs::remove_file(filepath).unwrap();

        assert_eq!(length, block.len());
        assert_eq!("Hello, world!".as_bytes(), &block[..]);
    }

    #[test]
    fn retrieve_block_with_offset_zero_and_length_more_than_file_length() {
        let config = Cfg::new(CONFIG_PATH).unwrap();

        let filename = "test_retrieve_block_04.txt";
        let filepath = format!("{}/{}", config.download_directory, filename);
        let contents = "Hello, world!".as_bytes();
        create_and_write_file(&config, filepath.as_str(), contents);

        let offset = 0;
        let length = contents.len() + 1;

        let io_error = retrieve_block(String::from(filename), offset, length, config).unwrap_err();

        fs::remove_file(filepath).unwrap();

        assert_eq!(io_error.kind(), std::io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn retrieve_block_with_offset_in_middle_and_length_more_than_file_length() {
        let config = Cfg::new(CONFIG_PATH).unwrap();

        let filename = "test_retrieve_block_05.txt";
        let filepath = format!("{}/{}", config.download_directory, filename);
        let contents = "Hello, world!".as_bytes();
        create_and_write_file(&config, filepath.as_str(), contents);

        let offset = 0;
        let length = contents.len() + 1;

        let io_error = retrieve_block(String::from(filename), offset, length, config).unwrap_err();

        fs::remove_file(filepath).unwrap();

        assert_eq!(io_error.kind(), std::io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn retrieve_block_with_offset_zero_and_length_zero() {
        let config = Cfg::new(CONFIG_PATH).unwrap();

        let filename = "test_retrieve_block_06.txt";
        let filepath = format!("{}/{}", config.download_directory, filename);
        let contents = "Hello, world!".as_bytes();
        create_and_write_file(&config, filepath.as_str(), contents);

        let offset = 0;
        let length = 0;

        let block = retrieve_block(String::from(filename), offset, length, config)
            .map_err(|err| {
                fs::remove_file(&filepath).unwrap();
                err
            })
            .unwrap();

        fs::remove_file(filepath).unwrap();

        assert_eq!(length, block.len());
        assert_eq!("".as_bytes(), &block[..]);
    }

    #[test]
    fn retrieve_block_and_directory_does_not_exist() {
        let config = Cfg::new(CONFIG_PATH).unwrap();

        let filename = "test_retrieve_block_07.txt";

        let offset = 0;
        let length = 6;

        let io_error = retrieve_block(String::from(filename), offset, length, config).unwrap_err();

        assert_eq!(io_error.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn retrieve_block_and_file_does_not_exist() {
        let config = Cfg::new(CONFIG_PATH).unwrap();

        let filename = "test_retrieve_block_08.txt";
        create_downloads_dir_if_necessary(config.download_directory.as_str());

        let offset = 0;
        let length = 5;

        let io_error = retrieve_block(String::from(filename), offset, length, config).unwrap_err();

        assert_eq!(io_error.kind(), std::io::ErrorKind::NotFound);
    }

    fn create_and_write_file(config: &Cfg, path: &str, contents: &[u8]) {
        create_downloads_dir_if_necessary(config.download_directory.as_str());

        let mut file = File::create(path).unwrap();
        file.write_all(contents).unwrap();
    }

    // -------------------------------------------------------------------------------------

    #[test]
    fn save_file_creates_file_if_it_does_not_exist() {
        let file_name = "test_file_01.txt".to_string();
        let config = Cfg::new(CONFIG_PATH).unwrap();
        let path = format!("{}/{}", config.download_directory, &file_name);

        assert!(!Path::new(&path).exists());
        assert!(save_piece(
            file_name,
            &[0x50u8, 0x65u8, 0x72u8, 0xF3u8, 0x6Eu8],
            0,
            config
        )
        .is_ok());
        assert!(Path::new(&path).exists());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn write_in_nonexistent_file() {
        let file_name = "test_file_02.txt".to_string();
        let config = Cfg::new(CONFIG_PATH).unwrap();
        let path = format!("{}/{}", config.download_directory, &file_name);

        create_downloads_dir_if_necessary(config.download_directory.as_str());

        assert!(!Path::new(&path).exists());

        let content_to_write = vec![0x50u8, 0x65u8, 0x72u8, 0xF3u8, 0x6Eu8];
        assert!(save_piece(file_name, &content_to_write, 0, config).is_ok());
        assert!(Path::new(&path).exists());

        read_file_and_assert_its_content_equals_expected_content(content_to_write, &path);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn write_in_existing_file() {
        let file_name = "test_file_03.txt".to_string();
        let config = Cfg::new(CONFIG_PATH).unwrap();
        let path = format!("{}/{}", config.download_directory, &file_name);

        create_downloads_dir_if_necessary(config.download_directory.as_str());

        File::create(&path).unwrap();

        let content_to_write = vec![0x50u8, 0x65u8, 0x72u8, 0xF3u8, 0x6Eu8];
        assert!(save_piece(file_name, &content_to_write, 0, config).is_ok());

        read_file_and_assert_its_content_equals_expected_content(content_to_write, &path);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn write_at_the_end_of_existing_file_that_already_has_contents() {
        let file_name = "test_file_04.txt".to_string();
        let config = Cfg::new(CONFIG_PATH).unwrap();
        let path = format!("{}/{}", config.download_directory, &file_name);

        create_downloads_dir_if_necessary(config.download_directory.as_str());

        let mut file = File::create(&path).unwrap();
        let previous_content = vec![0x56u8, 0x69u8, 0x76u8, 0x61u8, 0x20u8];
        file.write_all(&previous_content).unwrap();

        let content_to_write = vec![0x50u8, 0x65u8, 0x72u8, 0xF3u8, 0x6Eu8];
        assert!(save_piece(file_name, &content_to_write, 5, config).is_ok());

        read_file_and_assert_its_content_equals_expected_content(
            vec![
                0x56u8, 0x69u8, 0x76u8, 0x61u8, 0x20u8, 0x50u8, 0x65u8, 0x72u8, 0xF3u8, 0x6Eu8,
            ],
            &path,
        );

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn write_between_pieces_of_existing_file_that_already_has_contents() {
        let file_name = "test_file_05.txt".to_string();
        let config = Cfg::new(CONFIG_PATH).unwrap();
        let path = format!("{}/{}", config.download_directory, &file_name);

        create_downloads_dir_if_necessary(config.download_directory.as_str());

        let mut file = File::create(&path).unwrap();
        let first_piece = vec![0x56u8, 0x69u8, 0x76u8, 0x61u8];
        let second_piece = vec![0x20, 0x50u8, 0x65u8];
        let third_piece = vec![0x72u8, 0xF3u8, 0x6Eu8];

        file.write_all(&first_piece).unwrap();
        file.write_all_at(&third_piece, 7).unwrap();

        assert!(save_piece(file_name, &second_piece, 4, config).is_ok());

        read_file_and_assert_its_content_equals_expected_content(
            vec![
                0x56u8, 0x69u8, 0x76u8, 0x61u8, 0x20u8, 0x50u8, 0x65u8, 0x72u8, 0xF3u8, 0x6Eu8,
            ],
            &path,
        );

        fs::remove_file(path).unwrap();
    }

    fn read_file_and_assert_its_content_equals_expected_content(
        expected_content: Vec<u8>,
        file_name: &str,
    ) {
        let content = fs::read(file_name).unwrap();
        assert_eq!(content, expected_content);
    }

    fn create_downloads_dir_if_necessary(path: &str) {
        if !Path::new(path).exists() {
            fs::create_dir_all(path).unwrap();
        }
    }
}
