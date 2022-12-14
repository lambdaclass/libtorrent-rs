/// Represents the payload of a Request message.
#[derive(Debug)]
pub struct Request {
    index: u32,
    begin: u32,
    length: u32,
}

impl Request {
    /// Creates a new `Request` message.
    pub fn new(index: u32, begin: u32, length: u32) -> Self {
        Self {
            index,
            begin,
            length,
        }
    }

    /// Converts a `Request` message to a byte array.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0; 12];
        bytes[0..4].copy_from_slice(&self.index.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.begin.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.length.to_be_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_as_bytes() {
        let index = 0u32;
        let begin = 0u32;
        let length = 16384u32;
        let request = Request::new(index, begin, length);

        let bytes = request.as_bytes();

        let mut expected = vec![];
        expected.extend(&index.to_be_bytes());
        expected.extend(&begin.to_be_bytes());
        expected.extend(&length.to_be_bytes());

        assert_eq!(bytes, expected);
    }
}
