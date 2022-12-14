use url_encoder::url_encoder::encode;

/// `QueryParams` struct containing the query parameters information.
///
/// To create a new `TrackerResponse` use the method builder `new()`.
///
/// To build the Query params string use the method 'build()'.
#[derive(Debug)]
pub struct QueryParams {
    info_hash: String,
    client_port: u32,
    info_length: i64,
    client_peer_id: String,
}

impl QueryParams {
    /// Creates a new `QueryParams` from an **info_hash**, **client_port** and **info_lenght** passed by parameters.
    pub fn new(
        info_hash: String,
        client_port: u32,
        info_length: i64,
        client_peer_id: String,
    ) -> QueryParams {
        QueryParams {
            info_hash,
            client_port,
            info_length,
            client_peer_id,
        }
    }

    /// Builds the QueryParams string and returns it.
    pub fn build(&self) -> String {
        format!(
            "?info_hash={}&peer_id={}&port={}&uploaded=0&downloaded=0&left={}&event=started",
            encode(self.info_hash.as_str()),
            self.client_peer_id,
            self.client_port,
            self.info_length
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_params_build() {
        let info_hash = "2c6b6858d61da9543d4231a71db4b1c9264b0685".to_string();
        let client_port = 6969;
        let length = 100;
        let peer_id = "test_peer_id".to_string();
        let query_params =
            QueryParams::new(info_hash.clone(), client_port, length, peer_id.clone());

        assert_eq!(
            query_params.build(),
            format!(
                "?info_hash={}&peer_id={}&port={}&uploaded=0&downloaded=0&left={}&event=started",
                encode(info_hash.as_str()),
                peer_id,
                client_port,
                length
            )
        );
    }
}
