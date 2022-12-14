/// `TrackerUrl` struct containing a tracker url information.
///
/// To create a new `TrackerUrl` use the method builder `parse()`.
#[derive(Debug, PartialEq, Clone)]
pub struct TrackerUrl {
    pub protocol: ConnectionProtocol,
    pub host: String,
    pub port: u32,
    pub endpoint: String,
}

/// Posible `TrackerUrl` Connection Protocol values.
#[derive(Debug, PartialEq, Clone)]
pub enum ConnectionProtocol {
    Http,
    Https,
}

/// Posible `TrackerUrl` Errors.
#[derive(Debug, PartialEq)]
pub enum TrackerUrlError {
    InvalidTrackerURL,
    UnsupportedConnectionProtocol,
    InvalidPortNumber,
}

impl TrackerUrl {
    /// Builds a new `TrackerUrl` from a &str tracker url.
    ///
    /// On success it returns a `TrackerUrl` cointaining the tracker's url information.
    ///
    /// It returns an `TrackerUrlError` if:
    /// - the url format is invalid.
    /// - The url connection protocol is unsupported.
    /// - the url port number is not a number.
    pub fn parse(url: &str) -> Result<Self, TrackerUrlError> {
        let (url_without_protocol, protocol) = Self::identify_and_remove_protocol(url)?;

        let (url_without_endpoint, endpoint) =
            Self::identify_and_remove_endpoint(&url_without_protocol)?;

        let host = Self::identify_host(&url_without_endpoint)?;

        let port: u32 = if url_without_endpoint.contains(':') {
            Self::identify_port(&url_without_endpoint)?
        } else {
            match protocol {
                ConnectionProtocol::Https => 443,
                ConnectionProtocol::Http => 80,
            }
        };

        Ok(Self {
            protocol,
            host,
            port,
            endpoint,
        })
    }

    fn identify_and_remove_protocol(
        url: &str,
    ) -> Result<(String, ConnectionProtocol), TrackerUrlError> {
        let mut splitted_url = url.split("://");

        let protocol = match splitted_url.next() {
            Some(protocol_name) => {
                if protocol_name == "http" {
                    ConnectionProtocol::Http
                } else if protocol_name == "https" {
                    ConnectionProtocol::Https
                } else {
                    return Err(TrackerUrlError::UnsupportedConnectionProtocol);
                }
            }
            None => return Err(TrackerUrlError::InvalidTrackerURL),
        };

        match splitted_url.next() {
            Some(url_without_protocol) => Ok((url_without_protocol.to_string(), protocol)),
            None => Err(TrackerUrlError::InvalidTrackerURL),
        }
    }

    fn identify_and_remove_endpoint(url: &str) -> Result<(String, String), TrackerUrlError> {
        let mut splitted_url = url.split('/');

        let url_without_endpoint = match splitted_url.next() {
            Some(url_without_endpoint) => url_without_endpoint,
            None => return Err(TrackerUrlError::InvalidTrackerURL),
        };

        match splitted_url.next() {
            Some(endpoint) => Ok((url_without_endpoint.to_string(), endpoint.to_string())),
            None => Err(TrackerUrlError::InvalidTrackerURL),
        }
    }

    fn identify_host(url: &str) -> Result<String, TrackerUrlError> {
        match url.split(':').next() {
            Some(host) => Ok(host.to_string()),
            None => Err(TrackerUrlError::InvalidTrackerURL),
        }
    }

    fn identify_port(url: &str) -> Result<u32, TrackerUrlError> {
        match url.split(':').last() {
            Some(port) => match port.parse() {
                Ok(port_number) => Ok(port_number),
                Err(_) => Err(TrackerUrlError::InvalidPortNumber),
            },
            None => Err(TrackerUrlError::InvalidTrackerURL),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_https_no_port() {
        let url = String::from("https://www.example.org/ann");
        let parsed_tracker_url = TrackerUrl::parse(&url).unwrap();

        assert_eq!(ConnectionProtocol::Https, parsed_tracker_url.protocol);
        assert_eq!("www.example.org", parsed_tracker_url.host);
        assert_eq!(443, parsed_tracker_url.port);
        assert_eq!("ann", parsed_tracker_url.endpoint);
    }

    #[test]
    fn test_http_no_port() {
        let url = String::from("http://www.example.org/ann");
        let parsed_tracker_url = TrackerUrl::parse(&url).unwrap();

        assert_eq!(ConnectionProtocol::Http, parsed_tracker_url.protocol);
        assert_eq!("www.example.org", parsed_tracker_url.host);
        assert_eq!(80, parsed_tracker_url.port);
        assert_eq!("ann", parsed_tracker_url.endpoint);
    }

    #[test]
    fn test_http_with_port() {
        let url = String::from("http://www.example.org:1337/ann");
        let parsed_tracker_url = TrackerUrl::parse(&url).unwrap();

        assert_eq!(ConnectionProtocol::Http, parsed_tracker_url.protocol);
        assert_eq!("www.example.org", parsed_tracker_url.host);
        assert_eq!(1337, parsed_tracker_url.port);
        assert_eq!("ann", parsed_tracker_url.endpoint);
    }

    #[test]
    fn test_https_with_port() {
        let url = String::from("https://www.example.org:1337/ann");
        let parsed_tracker_url = TrackerUrl::parse(&url).unwrap();

        assert_eq!(ConnectionProtocol::Https, parsed_tracker_url.protocol);
        assert_eq!("www.example.org", parsed_tracker_url.host);
        assert_eq!(1337, parsed_tracker_url.port);
        assert_eq!("ann", parsed_tracker_url.endpoint);
    }

    #[test]
    fn test_invalid_protocol() {
        let url = String::from("udp://www.example.org:1337/ann");

        assert_eq!(
            TrackerUrl::parse(&url),
            Err(TrackerUrlError::UnsupportedConnectionProtocol)
        );
    }

    #[test]
    fn test_invalid_port() {
        let url = String::from("https://www.example.org:12a/ann");
        assert_eq!(
            TrackerUrl::parse(&url),
            Err(TrackerUrlError::InvalidPortNumber)
        );
    }

    #[test]
    fn test_missing_path() {
        let url = String::from("https://www.example.org:123");
        assert_eq!(
            TrackerUrl::parse(&url),
            Err(TrackerUrlError::InvalidTrackerURL)
        );
    }
}
