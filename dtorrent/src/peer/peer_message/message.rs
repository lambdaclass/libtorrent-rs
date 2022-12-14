// IDs of the messages defined in the protocol.
#[derive(PartialEq, Debug, Clone)]
pub enum MessageId {
    KeepAlive = -1,
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
    Port = 9,
}

/// The message that is sent to the peer.
///
/// It contains the message ID and the payload.
#[derive(Debug)]
pub struct Message {
    pub id: MessageId,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum MessageError {
    InvalidMessage,
}

impl Message {
    /// Creates a new `Message` from a message ID and a payload.
    pub fn new(id: MessageId, payload: Vec<u8>) -> Self {
        Self { id, payload }
    }

    /// Parses a byte array into a `Message`.
    pub fn from_bytes(payload: &[u8]) -> Result<Self, MessageError> {
        let id = match payload[0] {
            0 => MessageId::Choke,
            1 => MessageId::Unchoke,
            2 => MessageId::Interested,
            3 => MessageId::NotInterested,
            4 => MessageId::Have,
            5 => MessageId::Bitfield,
            6 => MessageId::Request,
            7 => MessageId::Piece,
            8 => MessageId::Cancel,
            9 => MessageId::Port,
            _ => return Err(MessageError::InvalidMessage),
        };

        let msg_payload = if payload.len() > 1 {
            payload[1..].to_vec()
        } else {
            vec![]
        };

        Ok(Self {
            id,
            payload: msg_payload,
        })
    }

    /// Converts a `Message` to a byte array.
    pub fn as_bytes(&self) -> Vec<u8> {
        let len = self.payload.len() + 1;
        let len_bytes: [u8; 4] = (len as u32).to_be_bytes();
        let mut bytes = vec![0; 4 + len];
        bytes[0..4].copy_from_slice(&len_bytes);
        bytes[4] = self.id.clone() as u8;
        bytes[5..].copy_from_slice(&self.payload);
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_unchoke_from_bytes() {
        let payload = 1u8.to_be_bytes();
        let msg = Message::from_bytes(&payload).unwrap();

        assert_eq!(msg.id, MessageId::Unchoke);
        assert_eq!(msg.payload, vec![]);
    }

    #[test]
    fn test_message_interested_from_bytes() {
        let payload = 2u8.to_be_bytes();
        let msg = Message::from_bytes(&payload).unwrap();

        assert_eq!(msg.id, MessageId::Interested);
        assert_eq!(msg.payload, vec![]);
    }

    #[test]
    fn test_message_request_as_bytes() {
        let index = 0u32.to_be_bytes();
        let begin = 0u32.to_be_bytes();
        let length = 16384u32.to_be_bytes();
        let payload = [index, begin, length].concat();
        let msg = Message::new(MessageId::Request, payload.clone());

        let bytes = msg.as_bytes();

        let len = 13u32.to_be_bytes();
        let msg_type = 6u8.to_be_bytes();
        let mut expected = vec![];
        expected.extend(&len);
        expected.extend(&msg_type);
        expected.extend(&payload);

        assert_eq!(bytes, expected);
    }

    #[test]
    fn test_message_interested_as_bytes() {
        let msg = Message::new(MessageId::Interested, vec![]);

        let bytes = msg.as_bytes();

        let len = 1u32.to_be_bytes();
        let msg_type = 2u8.to_be_bytes();
        let mut expected = vec![];
        expected.extend(&len);
        expected.extend(&msg_type);

        assert_eq!(bytes, expected);
    }
}
