use native_tls::Error;
use native_tls::HandshakeError;
use native_tls::TlsConnector;
use std::io::Error as IOError;
use std::io::{Read, Write};
use std::net::TcpStream;

use super::query_params::QueryParams;
use super::url_parser::TrackerUrl;

/// `HttpHandler` struct to make **HTTP** requests.
///
/// To create a new `HttpHandler` use the method builder `new()`.
///
/// To make a **HTTPS** request use the method `https_request()`.
///
/// To make a **HTTP** request use the method `http_request()`.
#[derive(Debug)]
pub struct HttpHandler {
    tracker_url: TrackerUrl,
    query_params: QueryParams,
}

/// Posible `HttpHandler` errors
#[derive(Debug)]
pub enum HttpHandlerError {
    CreateTlsConnectorError(Error),
    TcpStreamConnectError(IOError),
    TlsStreamConnectError(TlsStreamConnectError),
    ErrorWritingStream(IOError),
    ErrorReadingStream(IOError),
}

/// Posible `TlsStreamConnect` errors.
///
/// `FatalError` is an error that should not continue the program.
///
/// `BlockError` is an error that can be caused because the stream is performing I/O,
/// it should be safe to call `handshake` at a later time.
#[derive(Debug)]
pub enum TlsStreamConnectError {
    FatalError,
    BlockError,
}

impl HttpHandler {
    /// Builds a new `HttpHandler` from a **TrackerUrl** and a **QueryParams** passed by paramaters.
    pub fn new(tracker_url: TrackerUrl, query_params: QueryParams) -> Self {
        Self {
            tracker_url,
            query_params,
        }
    }

    /// Makes a **HTTPS** request to the tracker url.
    ///
    /// On success it returns a `Vec<u8>` cointaining the tracker's response.
    ///
    /// It returns an `HttpHandlerError` if:
    /// - There was a problem creating a TlsConnector.
    /// - There was a problem connecting to the tracker_url.
    /// - There was a problem writing to the tracker stream.
    /// - There was a problem reading the tracker stream.
    pub fn https_request(&self) -> Result<Vec<u8>, HttpHandlerError> {
        let connector = match TlsConnector::new() {
            Ok(connector) => connector,
            Err(err) => return Err(HttpHandlerError::CreateTlsConnectorError(err)),
        };
        let stream = self.connect_tcp_stream()?;
        let mut stream = match connector.connect(self.tracker_url.host.as_str(), stream) {
            Ok(stream) => stream,
            Err(err) => match err {
                HandshakeError::Failure(_) => {
                    return Err(HttpHandlerError::TlsStreamConnectError(
                        TlsStreamConnectError::FatalError,
                    ))
                }
                HandshakeError::WouldBlock(_) => {
                    return Err(HttpHandlerError::TlsStreamConnectError(
                        TlsStreamConnectError::BlockError,
                    ))
                }
            },
        };
        self.request_and_decode(&mut stream)
    }

    /// Makes a **HTTP** request to the tracker url.
    ///
    /// On success it returns a `Vec<u8>` cointaining the tracker's response.
    ///
    /// It returns an `HttpHandlerError` if:
    /// - There was a problem connecting to the tracker_url.
    /// - There was a problem writing to the tracker stream.
    /// - There was a problem reading the tracker stream.
    pub fn http_request(&self) -> Result<Vec<u8>, HttpHandlerError> {
        self.request_and_decode(&self.connect_tcp_stream()?)
    }

    fn connect_tcp_stream(&self) -> Result<TcpStream, HttpHandlerError> {
        let connect_url = format!("{}:{}", self.tracker_url.host, self.tracker_url.port);
        match TcpStream::connect(connect_url) {
            Ok(stream) => Ok(stream),
            Err(err) => Err(HttpHandlerError::TcpStreamConnectError(err)),
        }
    }

    fn request_and_decode<A>(&self, mut stream: A) -> Result<Vec<u8>, HttpHandlerError>
    where
        A: Write + Read,
    {
        let query_params = self.query_params.build();
        let mut request = format!(
            "GET /{}{} HTTP/1.1",
            self.tracker_url.endpoint, query_params
        );
        request.push_str("\r\n");
        request.push_str("Host: ");
        request.push_str(self.tracker_url.host.as_str());
        request.push_str("\r\n");
        request.push_str("User-Agent: LDTorrent/0.1");
        request.push_str("\r\n");
        request.push_str("\r\n");

        match stream.write_all(request.as_bytes()) {
            Ok(_) => (),
            Err(err) => return Err(HttpHandlerError::ErrorWritingStream(err)),
        }
        let mut res = vec![];
        match stream.read_to_end(&mut res) {
            Ok(_) => (),
            Err(err) => return Err(HttpHandlerError::ErrorReadingStream(err)),
        };

        Ok(Self::parse_http_response(&res).to_vec())
    }

    fn parse_http_response(res: &[u8]) -> &[u8] {
        for (i, b) in res.iter().enumerate() {
            if i + 3 > res.len() {
                break;
            }

            if *b == b"\r"[0]
                && res[i + 1] == b"\n"[0]
                && res[i + 2] == b"\r"[0]
                && res[i + 3] == b"\n"[0]
            {
                return &res[(i + 4)..];
            }
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use crate::tracker::http::url_parser;

    use super::*;

    #[test]
    fn test_http_handler_https_request() {
        let http_handler = HttpHandler::new(
            url_parser::TrackerUrl::parse("https://torrent.ubuntu.com/announce").unwrap(),
            QueryParams::new(
                "e82753b6692c4f3f3646b055f70ee390309020e6".to_string(),
                6969,
                100,
                "-qB4500-k51bMCWVA(~!".to_string(),
            ),
        );
        let response = http_handler.https_request().unwrap();

        // d8:complete
        assert!(response.starts_with(&[100, 56, 58, 99, 111, 109, 112, 108, 101, 116, 101]));
    }

    #[test]
    fn test_bad_http_handler_https_request() {
        let http_handler = HttpHandler::new(
            url_parser::TrackerUrl::parse("https://torrent.ubuntu.com:443/announce").unwrap(),
            QueryParams::new(
                "info_hash_test_info_hash_test_info_hash_test".to_string(),
                6969,
                100,
                "test_peer_id".to_string(),
            ),
        );
        let response = http_handler.https_request().unwrap();

        // d14:failure
        assert!(response.starts_with(&[100, 49, 52, 58, 102, 97, 105, 108, 117, 114, 101]));
    }

    #[test]
    fn test_http_handler_http_request() {
        let http_handler = HttpHandler::new(
            url_parser::TrackerUrl::parse("http://vps02.net.orel.ru/announce").unwrap(),
            QueryParams::new(
                "f834824904be1854c89ba007c01678ff797f8dc7".to_string(),
                6969,
                100,
                "-qB4500-k51bMCWVA(~!".to_string(),
            ),
        );
        let response = http_handler.http_request().unwrap();

        // d8:complete
        assert!(response.starts_with(&[100, 56, 58, 99, 111, 109, 112, 108, 101, 116, 101]));
    }

    #[test]
    fn test_bad_http_handler_http_request() {
        let http_handler = HttpHandler::new(
            url_parser::TrackerUrl::parse("http://vps02.net.orel.ru/announce").unwrap(),
            QueryParams::new(
                "info_hash_test_info_hash_test_info_hash_test".to_string(),
                6969,
                100,
                "test_peer_id".to_string(),
            ),
        );
        let response = http_handler.http_request().unwrap();

        // <title>Invalid Request</title>
        assert!(response.starts_with(&[
            60, 116, 105, 116, 108, 101, 62, 73, 110, 118, 97, 108, 105, 100, 32, 82, 101, 113,
            117, 101, 115, 116, 60, 47, 116, 105, 116, 108, 101, 62, 10
        ]));
    }
}
