use std::{collections::HashMap, str::FromStr};

use super::http_method::HttpMethod;

/// A struct that represents a HTTP request.
///
/// # Fields
/// * `method`: The HTTP method of the request.
/// * `endpoint`: The endpoint of the request.
/// * `params`: The parameters of the request.
pub struct Http {
    pub method: HttpMethod,
    pub endpoint: String,
    pub params: HashMap<String, String>,
}

#[derive(Debug)]
pub enum HttpError {
    ParseError,
    HttpMethodNotSupported,
}

impl Http {
    /// Parses a HTTP request. If the request is invalid, returns an error.
    pub fn parse(buffer: &[u8]) -> Result<Http, HttpError> {
        let mut lines = buffer.split(|&b| b == b'\r');
        let line = lines.next().ok_or(HttpError::ParseError)?;

        let mut line_split = line.split(|&b| b == b' ');
        let method = HttpMethod::from_str(
            &String::from_utf8_lossy(line_split.next().ok_or(HttpError::ParseError)?).to_string(),
        )
        .map_err(|_| HttpError::HttpMethodNotSupported)?;

        let mut endpoint_split = line_split
            .next()
            .ok_or(HttpError::ParseError)?
            .split(|&b| b == b'?');
        let endpoint = String::from_utf8_lossy(endpoint_split.next().ok_or(HttpError::ParseError)?)
            .to_string();

        let query_params = endpoint_split.next().ok_or(HttpError::ParseError)?;
        let params = parse_params(query_params).map_err(|_| HttpError::ParseError)?;

        Ok(Http {
            method,
            endpoint,
            params,
        })
    }
}

fn parse_params(query_params: &[u8]) -> Result<HashMap<String, String>, HttpError> {
    let mut params = HashMap::new();
    let query_params = query_params.split(|&b| b == b'&');

    for param in query_params {
        let mut param_split = param.split(|&b| b == b'=');
        let key =
            String::from_utf8_lossy(param_split.next().ok_or(HttpError::ParseError)?).to_string();
        let value =
            String::from_utf8_lossy(param_split.next().ok_or(HttpError::ParseError)?).to_string();
        params.insert(key, value);
    }

    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_request() {
        let buffer = "GET /announce?info_hash=%b1%11%81%3c%e6%0f%42%91%97%34%82%3d%f5%ec%20%bd%1e%04%e7%f7&peer_id=DTorrent:02284204893&port=6969&uploaded=0&downloaded=0&left=396361728&event=started HTTP/1.1\r\nHost: bttracker.debian.org\r\nUser-Agent: LDTorrent/0.1\r\n\r\n".as_bytes();
        let http = Http::parse(buffer).unwrap();
        let mut params = HashMap::new();
        params.insert(
            "info_hash".to_string(),
            "%b1%11%81%3c%e6%0f%42%91%97%34%82%3d%f5%ec%20%bd%1e%04%e7%f7".to_string(),
        );
        params.insert("peer_id".to_string(), "DTorrent:02284204893".to_string());
        params.insert("port".to_string(), "6969".to_string());
        params.insert("uploaded".to_string(), "0".to_string());
        params.insert("downloaded".to_string(), "0".to_string());
        params.insert("left".to_string(), "396361728".to_string());
        params.insert("event".to_string(), "started".to_string());

        assert_eq!(http.method, HttpMethod::from_str("GET").unwrap());
        assert_eq!(http.endpoint, "/announce");
        assert_eq!(http.params, params);
    }

    #[test]
    fn test_parse_request_without_record_cannot_be_parsed() {
        let buffer = "/announce?info_hash=%b1%11%81%3c%e6%0f%42%91%97%34%82%3d%f5%ec%20%bd%1e%04%e7%f7&peer_id=DTorrent:02284204893&port=6969&uploaded=0&downloaded=0&left=396361728&event=started HTTP/1.1\r\nHost: bttracker.debian.org\r\nUser-Agent: LDTorrent/0.1\r\n\r\n".as_bytes();
        assert!(Http::parse(buffer).is_err());
    }

    #[test]
    fn test_parse_request_without_query_cannot_be_parsed() {
        let buffer =
            "GET\r\nHost: bttracker.debian.org\r\nUser-Agent: LDTorrent/0.1\r\n\r\n".as_bytes();
        assert!(Http::parse(buffer).is_err());
    }
}
