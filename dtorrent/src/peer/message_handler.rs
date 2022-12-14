use logger::logger_sender::LoggerSender;
use std::{io::Write, net::TcpStream, sync::Arc};

use crate::{
    torrent_handler::status::{AtomicTorrentStatus, AtomicTorrentStatusError},
    torrent_parser::torrent::Torrent,
};

use super::{
    handshake::Handshake,
    peer_message::{Bitfield, Message, MessageId, Request},
};

#[derive(Debug)]
pub enum MessageHandlerError {
    ErrorGettingBitfield(AtomicTorrentStatusError),
    ErrorGettingPiece(AtomicTorrentStatusError),
    HandshakeError,
    MessageError(MessageId),
}

/// Message handler for a peer session.
///
/// It handles the handshake as well as the sending and receiving of messages from a peer.
pub struct MessageHandler {
    torrent: Torrent,
    torrent_status: Arc<AtomicTorrentStatus>,
    logger_sender: LoggerSender,
    client_peer_id: String,
}

impl MessageHandler {
    pub fn new(
        torrent: Torrent,
        torrent_status: Arc<AtomicTorrentStatus>,
        logger_sender: LoggerSender,
        client_peer_id: String,
    ) -> MessageHandler {
        Self {
            torrent,
            torrent_status,
            logger_sender,
            client_peer_id,
        }
    }

    /// ------------------------------------------------------------------------------------------------
    /// Receiving messages

    /// Handles a bitfield message received from the peer.
    pub fn handle_bitfield(&mut self, message: Message) -> Bitfield {
        Bitfield::new(message.payload)
    }

    /// Handles a piece message received from the peer.
    pub fn handle_piece(&mut self, message: Message) -> Vec<u8> {
        let block = &message.payload[8..];
        block.to_vec()
    }

    // Returns the received piece index
    pub fn handle_have(&mut self, message: Message) -> u32 {
        let mut index: [u8; 4] = [0; 4];
        index.copy_from_slice(&message.payload[0..4]);
        u32::from_be_bytes(index)
    }

    /// ------------------------------------------------------------------------------------------------
    /// Sending messages

    /// Sends a piece message to the peer.
    pub fn send_piece(
        &mut self,
        index: u32,
        begin: u32,
        block: &[u8],
        stream: &mut TcpStream,
    ) -> Result<(), MessageHandlerError> {
        let mut payload = vec![];
        payload.extend(index.to_be_bytes());
        payload.extend(begin.to_be_bytes());
        payload.extend(block);

        let piece_msg = Message::new(MessageId::Piece, payload);
        self.send(stream, piece_msg)?;

        self.logger_sender
            .info(&format!("Sent piece: {} / Offset: {}", index, begin));

        Ok(())
    }

    /// Sends a unchoked message to the peer.
    pub fn send_unchoked(&mut self, stream: &mut TcpStream) -> Result<(), MessageHandlerError> {
        let unchoked_msg = Message::new(MessageId::Unchoke, vec![]);
        self.send(stream, unchoked_msg)?;
        Ok(())
    }

    /// Sends a bitfield message to the peer.
    pub fn send_bitfield(&mut self, stream: &mut TcpStream) -> Result<(), MessageHandlerError> {
        let bitfield = self
            .torrent_status
            .get_bitfield()
            .map_err(MessageHandlerError::ErrorGettingBitfield)?;

        let bitfield_msg = Message::new(MessageId::Bitfield, bitfield.get_vec());
        self.send(stream, bitfield_msg)?;
        Ok(())
    }

    /// Sends a request message to the peer.
    pub fn send_request(
        &self,
        index: u32,
        begin: u32,
        length: u32,
        stream: &mut TcpStream,
    ) -> Result<(), MessageHandlerError> {
        let payload = Request::new(index, begin, length).as_bytes();

        let request_msg = Message::new(MessageId::Request, payload);
        self.send(stream, request_msg)?;
        Ok(())
    }

    /// Sends an interested message to the peer.
    pub fn send_interested(&mut self, stream: &mut TcpStream) -> Result<(), MessageHandlerError> {
        let interested_msg = Message::new(MessageId::Interested, vec![]);
        self.send(stream, interested_msg)?;
        Ok(())
    }

    /// Sends a cancel message to the peer.
    pub fn send_cancel(
        &mut self,
        index: u32,
        begin: u32,
        length: u32,
        stream: &mut TcpStream,
    ) -> Result<(), MessageHandlerError> {
        let mut payload = vec![];
        payload.extend(index.to_be_bytes());
        payload.extend(begin.to_be_bytes());
        payload.extend(length.to_be_bytes());

        let cancel_msg = Message::new(MessageId::Cancel, payload);
        self.send(stream, cancel_msg)?;

        self.logger_sender
            .info(&format!("Cancel piece: {} / Offset: {}", index, begin));
        Ok(())
    }

    pub fn send_have(
        &mut self,
        index: u32,
        stream: &mut TcpStream,
    ) -> Result<(), MessageHandlerError> {
        let mut payload = vec![];
        payload.extend(index.to_be_bytes());

        let have_msg = Message::new(MessageId::Have, payload);
        self.send(stream, have_msg)?;

        Ok(())
    }

    /// Generic sending function.
    fn send(&self, stream: &mut TcpStream, message: Message) -> Result<(), MessageHandlerError> {
        stream
            .write_all(&message.as_bytes())
            .map_err(|_| MessageHandlerError::MessageError(message.id))?;
        Ok(())
    }

    /// ------------------------------------------------------------------------------------------------
    /// Handshake

    /// Sends a handshake to the peer.
    ///
    /// It returns an error if the handshake could not be sent or the handshake was not successful.
    pub fn send_handshake(&mut self, stream: &mut TcpStream) -> Result<(), MessageHandlerError> {
        let info_hash = self
            .torrent
            .get_info_hash_as_bytes()
            .map_err(|_| MessageHandlerError::HandshakeError)?;

        let handshake = Handshake::new(info_hash, self.client_peer_id.as_bytes().to_vec());
        stream
            .write_all(&handshake.as_bytes())
            .map_err(|_| MessageHandlerError::HandshakeError)?;
        Ok(())
    }
}
