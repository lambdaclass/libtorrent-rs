use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::str::FromStr;

use super::constants;

/// `Cfg` struct containing the config file information, previusly created with `Cfg::new`.
///
/// - `tcp_port`: port to listen for incoming connections,
/// - `log_directory`: directory where the log files will be stored,
/// - `download_directory`: directory where the downloaded files will be stored,
/// - `pipelining_size`: number of request sent to a peer before waiting for the response,
/// - `read_write_seconds_timeout`: timeout in seconds for the read and write operations to a peer,
/// - `max_peers_per_torrent`: maximum number of simultaneous peers that a torrent can have,
/// - `max_log_file_kb_size`: max file size in kilobytes the log can have,
#[derive(Debug, Clone)]
pub struct Cfg {
    pub tcp_port: u16,
    pub log_directory: String,
    pub download_directory: String,
    pub pipelining_size: u32,
    pub read_write_seconds_timeout: u64,
    pub max_peers_per_torrent: u32,
    pub max_log_file_kb_size: u32,
}

impl Cfg {
    /// Builds a Cfg struct containing the config file information by the given path.
    /// The format of the config file must be: {config_name}={config_value} (without brackets).
    /// In case of success it returns a Cfg struct.
    ///
    /// It returns an io::Error if:
    /// - The path to the config file does not exist or could not be open/readed.
    /// - The confing file has wrong format.
    /// - A wrong config_name was in the config file.
    /// - tcp_port setting is not a valid number in the config file.
    /// - pipelining_size setting is not a valid number in the config file.
    /// - read_write_timeout setting is not a valid number in the config file.
    /// - max_peers_per_torrent  setting is not a valid number in the config file.
    /// - max_log_file_size setting is not a valid number in the config file.
    /// - Minimum number of correct settings were not reached.
    pub fn new(path: &str) -> io::Result<Self> {
        let mut cfg = Self {
            tcp_port: 0,
            log_directory: String::from(""),
            download_directory: String::from(""),
            pipelining_size: 0,
            read_write_seconds_timeout: 0,
            max_peers_per_torrent: 0,
            max_log_file_kb_size: 0,
        };

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut settings_loaded = 0;

        for line in reader.lines() {
            let current_line = line?;
            let setting: Vec<&str> = current_line.split('=').collect();

            if setting.len() != 2 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Invalid config input: {}", current_line),
                ));
            }
            cfg = Self::load_setting(cfg, setting[0], setting[1])?;
            settings_loaded += 1;
        }
        if settings_loaded < constants::MIN_SETTINGS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Minimum number of correct settings were not reached: {}",
                    settings_loaded
                ),
            ));
        }
        Ok(cfg)
    }

    fn load_setting(mut self, name: &str, value: &str) -> io::Result<Self> {
        match name {
            constants::TCP_PORT => {
                self.tcp_port = self.parse_value(value, constants::TCP_PORT)?;
            }
            constants::LOG_DIRECTORY => self.log_directory = String::from(value),

            constants::DOWNLOAD_DIRECTORY => self.download_directory = String::from(value),

            constants::PIPELINING_SIZE => {
                self.pipelining_size = self.parse_value(value, constants::PIPELINING_SIZE)?;
            }

            constants::READ_WRITE_SECONDS_TIMEOUT => {
                self.read_write_seconds_timeout =
                    self.parse_value(value, constants::READ_WRITE_SECONDS_TIMEOUT)?;
            }

            constants::MAX_PEERS_PER_TORRENT => {
                self.max_peers_per_torrent =
                    self.parse_value(value, constants::MAX_PEERS_PER_TORRENT)?;
            }

            constants::MAX_LOG_FILE_KB_SIZE => {
                self.max_log_file_kb_size =
                    self.parse_value(value, constants::MAX_LOG_FILE_KB_SIZE)?;
            }

            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Invalid config setting name: {}", name),
                ))
            }
        }
        Ok(self)
    }

    fn parse_value<F>(&self, value: &str, setting: &str) -> io::Result<F>
    where
        F: FromStr,
    {
        let parse = value.parse::<F>();
        match parse {
            Err(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Invalid setting: {}, is not a valid type: {}",
                        setting, value
                    ),
                ));
            }
            Ok(parse) => Ok(parse),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Write};

    // tests:
    //  1- test todo ok
    //  2- test archivo de config no existe
    //  3- test archivo vacio
    //  4- test setting que no existe
    //  5- test solo 2 settings
    //  6- test tcp_port no es numero
    //  7- test no importa el orden de los settings en el archivo
    //  8- test mal formato

    #[test]
    fn test_good_config() {
        let path = "./test_good_config.cfg";
        let contents = b"TCP_PORT=1000\nLOG_DIRECTORY=./log\nDOWNLOAD_DIRECTORY=./download\nPIPELINING_SIZE=5\nREAD_WRITE_SECONDS_TIMEOUT=120\nMAX_PEERS_PER_TORRENT=5\nMAX_LOG_FILE_KB_SIZE=100";
        create_and_write_file(path, contents);

        create_and_assert_config_is_ok(path, 1000, "./log", "./download", 5, 120, 5, 100);
    }

    #[test]
    fn test_bad_path() {
        let path = "bad path";
        let config = Cfg::new(path);
        assert!(config.is_err());
    }

    #[test]
    fn test_empty_file() {
        let path = "./test_empty_file.cfg";
        let contents = b"";
        create_and_write_file(path, contents);

        create_and_assert_config_is_not_ok(path);
    }

    #[test]
    fn test_setting_doesnt_exist() {
        let path = "./test_setting_doesnt_exist.cfg";
        let contents = b"WRONG_SETTING=1000";
        create_and_write_file(path, contents);

        create_and_assert_config_is_not_ok(path);
    }

    #[test]
    fn test_bad_number_of_settings() {
        let path = "./test_bad_number_of_settings.cfg";
        let contents = b"TCP_PORT=1000\nLOG_DIRECTORY=./log";
        create_and_write_file(path, contents);

        create_and_assert_config_is_not_ok(path);
    }

    #[test]
    fn test_tcp_port_not_a_number() {
        let path = "./test_tcp_port_not_a_number.cfg";
        let contents = b"TCP_PORT=abcd\nLOG_DIRECTORY=./log\nDOWNLOAD_DIRECTORY=./download\nPIPELINING_SIZE=5\nREAD_WRITE_SECONDS_TIMEOUT=120\nMAX_PEERS_PER_TORRENT=5\nMAX_LOG_FILE_KB_SIZE=100";
        create_and_write_file(path, contents);

        create_and_assert_config_is_not_ok(path);
    }

    #[test]
    fn test_read_write_timeout_not_a_number() {
        let path = "./test_read_write_timeout_not_a_number.cfg";
        let contents = b"TCP_PORT=1000\nLOG_DIRECTORY=./log\nDOWNLOAD_DIRECTORY=./download\nPIPELINING_SIZE=5\nREAD_WRITE_SECONDS_TIMEOUT=2segundos\nMAX_PEERS_PER_TORRENT=5\nMAX_LOG_FILE_KB_SIZE=100";
        create_and_write_file(path, contents);

        create_and_assert_config_is_not_ok(path);
    }

    #[test]
    fn test_pipelining_not_a_number() {
        let path = "./test_pipelining_not_a_number.cfg";
        let contents = b"TCP_PORT=1000\nLOG_DIRECTORY=./log\nDOWNLOAD_DIRECTORY=./download\nPIPELINING_SIZE=muy_grande\nREAD_WRITE_SECONDS_TIMEOUT=120\nMAX_PEERS_PER_TORRENT=5\nMAX_LOG_FILE_KB_SIZE=100";
        create_and_write_file(path, contents);

        create_and_assert_config_is_not_ok(path);
    }

    #[test]
    fn test_max_peers_not_a_number() {
        let path = "./test_max_peers_not_a_number.cfg";
        let contents = b"TCP_PORT=1000\nLOG_DIRECTORY=./log\nDOWNLOAD_DIRECTORY=./download\nPIPELINING_SIZE=5\nREAD_WRITE_SECODS_TIMEOUT=120\nMAX_PEERS_PER_TORRENT=un_millon\nMAX_LOG_FILE_KB_SIZE=100";
        create_and_write_file(path, contents);

        create_and_assert_config_is_not_ok(path);
    }

    #[test]
    fn test_max_log_file_size() {
        let path = "./test_max_log_file_size.cfg";
        let contents = b"TCP_PORT=1000\nLOG_DIRECTORY=./log\nDOWNLOAD_DIRECTORY=./download\nPIPELINING_SIZE=5\nREAD_WRITE_SECONDS_TIMEOUT=120\nMAX_PEERS_PER_TORRENT=100\nMAX_LOG_FILE_KB_SIZE=abc";
        create_and_write_file(path, contents);

        create_and_assert_config_is_not_ok(path);
    }

    #[test]
    fn test_order_doesnt_matter() {
        let path = "./test_order_doesnt_matter.cfg";
        let contents = b"LOG_DIRECTORY=./log2\nDOWNLOAD_DIRECTORY=./download2\nTCP_PORT=2500\nREAD_WRITE_SECONDS_TIMEOUT=10\nMAX_PEERS_PER_TORRENT=1\nPIPELINING_SIZE=10\nMAX_LOG_FILE_KB_SIZE=100";
        create_and_write_file(path, contents);

        create_and_assert_config_is_ok(path, 2500, "./log2", "./download2", 10, 10, 1, 100);
    }

    #[test]
    fn test_bad_format() {
        let path = "./test_bad_format.cfg";
        let contents = b"TCP_PORT=abcd=1234\nLOG_DIRECTORY=./log\nDOWNLOAD_DIRECTORY=./download\nPIPELINING_SIZE=5\nREAD_WRITE_SECONDS_TIMEOUT=120\nMAX_PEERS_PER_TORRENT=5";
        create_and_write_file(path, contents);

        create_and_assert_config_is_not_ok(path);
    }

    // Auxiliary functions

    fn create_and_write_file(path: &str, contents: &[u8]) -> () {
        let mut file =
            File::create(path).expect(&format!("Error creating file in path: {}", &path));
        file.write_all(contents)
            .expect(&format!("Error writing file in path: {}", &path));
    }

    fn create_and_assert_config_is_ok(
        path: &str,
        tcp_port: u16,
        log_directory: &str,
        download_directory: &str,
        pipelining_size: u32,
        read_write_timeout: u64,
        max_peers_per_torrent: u32,
        max_log_file_size: u32,
    ) {
        let config = Cfg::new(path);

        assert!(config.is_ok());

        let config = config.expect(&format!("Error creating config in path: {}", &path));

        assert_eq!(config.tcp_port, tcp_port);
        assert_eq!(config.log_directory, log_directory);
        assert_eq!(config.download_directory, download_directory);
        assert_eq!(config.pipelining_size, pipelining_size);
        assert_eq!(config.read_write_seconds_timeout, read_write_timeout);
        assert_eq!(config.max_peers_per_torrent, max_peers_per_torrent);
        assert_eq!(config.max_log_file_kb_size, max_log_file_size);

        fs::remove_file(path).expect(&format!("Error removing file in path: {}", &path));
    }

    fn create_and_assert_config_is_not_ok(path: &str) {
        let config = Cfg::new(path);
        assert!(config.is_err());
        fs::remove_file(path).expect(&format!("Error removing file in path: {}", &path));
    }
}
