use std::{
    fmt::Write,
    io::{self, Read, Write as IOWrite},
    net::TcpStream,
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, Local};
use tracing::{info, warn};
use sha1::{Digest, Sha1};

use crate::{
    config::cfg::Cfg,
    torrent_handler::status::{AtomicTorrentStatus, AtomicTorrentStatusError},
    torrent_parser::torrent::Torrent,
};

use super::{
    bt_peer::{BtPeer, BtPeerError},
    message_handler::{MessageHandler, MessageHandlerError},
    peer_message::{Bitfield, Message, MessageError, MessageId},
    session_status::SessionStatus,
};

const BLOCK_SIZE: u32 = 16384;

#[derive(Debug)]
pub enum PeerSessionError {
    ErrorReadingMessage(io::Error),
    MessageDoesNotExist(MessageError),
    CouldNotConnectToPeer,
    ErrorDisconnectingFromPeer(AtomicTorrentStatusError),
    ErrorAbortingPiece(AtomicTorrentStatusError),
    ErrorSelectingPiece(AtomicTorrentStatusError),
    ErrorNotifyingPieceDownloaded(AtomicTorrentStatusError),
    ErrorConnectingToPeer(AtomicTorrentStatusError),
    PieceHashDoesNotMatch,
    NoPiecesLeftToDownloadInThisPeer,
    ErrorGettingBitfield(AtomicTorrentStatusError),
    ErrorGettingPiece(AtomicTorrentStatusError),
    ErrorGettingSessionsStatus(AtomicTorrentStatusError),
    PeerNotInterested,
    MessageHandlerError(MessageHandlerError),
    MessageError(MessageId),
    MessageLengthTooLong,
    ErrorSettingStreamTimeout,
    BtPeerError(BtPeerError),
    PeerIsOurself,
}

/// A PeerSession represents a connection to a peer.
///
/// It is used to send and receive messages from a peer.
pub struct PeerSession {
    torrent: Torrent,
    peer: BtPeer,
    bitfield: Bitfield,
    status: SessionStatus,
    piece: Vec<u8>,
    torrent_status: Arc<AtomicTorrentStatus>,
    current_piece: u32,
    config: Cfg,
    message_handler: MessageHandler,
    client_peer_id: String,
}

impl PeerSession {
    pub fn new(
        peer: BtPeer,
        torrent: Torrent,
        torrent_status: Arc<AtomicTorrentStatus>,
        config: Cfg,
        client_peer_id: String,
    ) -> Result<Self, PeerSessionError> {
        let our_bitfield = Bitfield::new(
            torrent_status
                .get_bitfield()
                .map_err(PeerSessionError::ErrorGettingBitfield)?
                .get_vec(),
        );

        let message_handler = MessageHandler::new(
            torrent.clone(),
            torrent_status.clone(),
            client_peer_id.clone(),
        );

        let pieces_count = torrent.total_pieces();

        Ok(PeerSession {
            torrent,
            peer,
            bitfield: Bitfield::new(vec![0; (pieces_count / 8) as usize]),
            status: SessionStatus::new(our_bitfield),
            piece: vec![],
            torrent_status,
            current_piece: 0,
            config,
            message_handler,
            client_peer_id,
        })
    }

    // ------------------------------------------------------------------------------------------------
    // Uploading

    /// Handshakes with an incoming leecher.
    pub fn handshake_incoming_leecher(
        &mut self,
        stream: &mut TcpStream,
    ) -> Result<(), PeerSessionError> {
        self.message_handler
            .send_handshake(stream)
            .map_err(PeerSessionError::MessageHandlerError)?;

        info!("IP: {}:{} Handshake successful", self.peer.ip, self.peer.port);

        self.message_handler
            .send_bitfield(stream)
            .map_err(PeerSessionError::MessageHandlerError)?;

        info!("IP: {}:{} Bitfield sent", self.peer.ip, self.peer.port);

        Ok(())
    }

    pub fn unchoke_incoming_leecher(
        &mut self,
        stream: &mut TcpStream,
    ) -> Result<(), PeerSessionError> {
        self.torrent_status
            .peer_connected(&self.peer)
            .map_err(PeerSessionError::ErrorConnectingToPeer)?;
        match self.unchoke_incoming_leecher_wrap(stream) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.torrent_status
                    .peer_disconnected(&self.peer)
                    .map_err(PeerSessionError::ErrorDisconnectingFromPeer)?;
                Err(e)
            }
        }
    }

    /// Sends an unchoke message to the peer to start sending pieces.
    pub fn unchoke_incoming_leecher_wrap(
        &mut self,
        stream: &mut TcpStream,
    ) -> Result<(), PeerSessionError> {
        let mut id = self.read_message_from_stream(stream)?;
        while id != MessageId::Interested {
            // if we receive a `not interested` message, we close the connection.
            if id == MessageId::NotInterested {
                self.status.peer_interested = false;
                // peer disconnected
                return Err(PeerSessionError::PeerNotInterested);
            }
            // wait for the peer to send an interested message
            id = self.read_message_from_stream(stream)?;
        }

        // Peer is interested
        self.status.peer_interested = true;

        self.message_handler
            .send_unchoked(stream)
            .map_err(PeerSessionError::MessageHandlerError)?;

        self.status.peer_choked = false;

        loop {
            self.update_bitfield(stream)?;

            // TODO: Handle max connections.
            self.read_message_from_stream(stream)?;
        }
    }

    /// ------------------------------------------------------------------------------------------------
    /// Downloading

    /// Starts a connection to an outgoing seeder to start downloading pieces.
    ///
    /// It returns an error if:
    /// - The connection could not be established
    /// - The handshake was not successful
    pub fn start_outgoing_seeder(&mut self) -> Result<(), PeerSessionError> {
        let mut stream = match self.set_up_peer_session() {
            Ok(stream) => stream,
            Err(e) => {
                self.torrent_status.peer_connecting_failed();
                return Err(e);
            }
        };

        self.torrent_status
            .peer_connected(&self.peer)
            .map_err(PeerSessionError::ErrorConnectingToPeer)?;

        match self.start_outgoing_seeder_wrap(&mut stream) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.torrent_status
                    .peer_disconnected(&self.peer)
                    .map_err(PeerSessionError::ErrorDisconnectingFromPeer)?;
                Err(e)
            }
        }
    }

    fn set_up_peer_session(&mut self) -> Result<TcpStream, PeerSessionError> {
        let peer_socket = format!("{}:{}", self.peer.ip, self.peer.port);

        let mut stream = TcpStream::connect(&peer_socket)
            .map_err(|_| PeerSessionError::CouldNotConnectToPeer)?;

        self.set_stream_timeouts(&mut stream)?;

        self.message_handler
            .send_handshake(&mut stream)
            .map_err(PeerSessionError::MessageHandlerError)?;

        self.peer
            .receive_handshake(&mut stream)
            .map_err(PeerSessionError::BtPeerError)?;

        info!("Handshake successful");

        // Avoid connecting to ourself.
        match &self.peer.peer_id {
            Some(id) => {
                if id == self.client_peer_id.to_string().as_bytes() {
                    return Err(PeerSessionError::PeerIsOurself);
                }
            }
            None => (),
        }
        Ok(stream)
    }

    fn start_outgoing_seeder_wrap(
        &mut self,
        stream: &mut TcpStream,
    ) -> Result<(), PeerSessionError> {
        loop {
            self.read_message_from_stream(stream)?;

            if self.status.choked && !self.status.interested {
                self.message_handler
                    .send_interested(stream)
                    .map_err(PeerSessionError::MessageHandlerError)?;

                self.status.interested = true;
            }

            if !self.status.choked && self.status.interested {
                self.request_pieces(stream)?;
            }
        }
    }

    fn request_pieces(&mut self, stream: &mut TcpStream) -> Result<(), PeerSessionError> {
        loop {
            let piece_index = self
                .torrent_status
                .select_piece(&self.bitfield)
                .map_err(PeerSessionError::ErrorSelectingPiece)?;

            match piece_index {
                Some(piece_index) => {
                    self.current_piece = piece_index;
                    match self.download_piece(stream, piece_index) {
                        Ok(_) => {
                            self.torrent_status
                                .piece_downloaded(piece_index, &self.piece)
                                .map_err(PeerSessionError::ErrorNotifyingPieceDownloaded)?;
                        }
                        Err(e) => {
                            self.torrent_status
                                .piece_aborted(piece_index)
                                .map_err(PeerSessionError::ErrorAbortingPiece)?;

                            return Err(e);
                        }
                    }
                    if self.status.choked {
                        // If we are choked, we need to wait for the peer to unchoke us.
                        return Ok(());
                    }
                }
                None => {
                    return Err(PeerSessionError::NoPiecesLeftToDownloadInThisPeer);
                }
            };
        }
    }

    /// Downloads a piece from the peer given the piece index.
    fn download_piece(
        &mut self,
        stream: &mut TcpStream,
        piece_index: u32,
    ) -> Result<(), PeerSessionError> {
        self.piece = vec![]; // reset piece

        let entire_blocks_in_piece = self.download_with_pipeline(piece_index, stream)?;

        self.check_last_piece_block(piece_index, entire_blocks_in_piece, stream)?;

        self.validate_piece(&self.piece, piece_index)?;

        info!("Piece {} downloaded!", piece_index);

        let remaining_pieces = self.torrent_status.downloaded_pieces();
        info!("*** Torrent: {} - Pieces downloaded: {} / {}",
            self.torrent.name(),
            remaining_pieces,
            self.torrent.total_pieces());

        Ok(())
    }

    /// Downloads a piece in 'chunks' of blocks.
    ///
    /// If the pipelinening size in the config is 5, then it will request 5 blocks and wait for those 5 blocks to be received.
    ///
    /// If there are less than 5 blocks left in the piece, it will request the remaining blocks and wait for those blocks to be received.
    fn download_with_pipeline(
        &mut self,
        piece_index: u32,
        stream: &mut TcpStream,
    ) -> Result<u32, PeerSessionError> {
        let entire_blocks_in_piece = self.complete_blocks_in_torrent_piece(piece_index);
        let mut blocks_downloaded = 0;
        while blocks_downloaded < entire_blocks_in_piece {
            let remaining_blocks = entire_blocks_in_piece - blocks_downloaded;
            let blocks_to_download = if remaining_blocks % self.config.pipelining_size == 0 {
                self.config.pipelining_size
            } else {
                remaining_blocks
            };

            let download_start_time = Local::now();

            // request blocks
            for block in 0..blocks_to_download {
                self.message_handler
                    .send_request(
                        piece_index,
                        (block + blocks_downloaded) * BLOCK_SIZE,
                        BLOCK_SIZE,
                        stream,
                    )
                    .map_err(PeerSessionError::MessageHandlerError)?;
            }

            // If we are in the endgame phase, and we already downloaded all the blocks, we send a cancel message.
            if self.torrent_status.is_finished() {
                for block in 0..blocks_to_download {
                    self.message_handler
                        .send_cancel(
                            piece_index,
                            (block + blocks_downloaded) * BLOCK_SIZE,
                            BLOCK_SIZE,
                            stream,
                        )
                        .map_err(PeerSessionError::MessageHandlerError)?;
                }
            }

            // Check that we receive a piece message.
            // If we receive another message we handle it accordingly.
            let mut current_blocks_downloaded = 0;
            while current_blocks_downloaded < blocks_to_download {
                if self.read_message_from_stream(stream)? == MessageId::Piece {
                    current_blocks_downloaded += 1;
                    blocks_downloaded += 1;
                }
            }
            // Calculate download speed
            let download_speed = self.calculate_kilobits_per_second(
                download_start_time,
                (blocks_to_download * BLOCK_SIZE).into(),
            );
            self.status.download_speed = download_speed;
            self.update_peer_status()?;
        }
        Ok(entire_blocks_in_piece)
    }

    fn check_last_piece_block(
        &mut self,
        piece_index: u32,
        entire_blocks_in_piece: u32,
        stream: &mut TcpStream,
    ) -> Result<(), PeerSessionError> {
        let last_block_size = self.torrent.last_piece_size() % BLOCK_SIZE;

        let last_piece_index = self.torrent.total_pieces() - 1;

        if last_block_size != 0 && piece_index == last_piece_index {
            self.message_handler
                .send_request(
                    piece_index,
                    entire_blocks_in_piece * BLOCK_SIZE,
                    last_block_size,
                    stream,
                )
                .map_err(PeerSessionError::MessageHandlerError)?;

            while self.read_message_from_stream(stream)? != MessageId::Piece {
                continue;
            }
        }
        Ok(())
    }

    fn complete_blocks_in_torrent_piece(&self, piece_index: u32) -> u32 {
        let last_piece_index = self.torrent.total_pieces() - 1;

        if piece_index != last_piece_index {
            self.torrent.piece_length() / BLOCK_SIZE
        } else {
            let last_piece_size = self.torrent.last_piece_size();

            // If the last piece is multiple of the piece length, then is the same as the other pieces.
            if last_piece_size == 0 {
                self.torrent.piece_length() / BLOCK_SIZE
            } else {
                (last_piece_size as f64 / BLOCK_SIZE as f64).floor() as u32
            }
        }
    }

    /// ------------------------------------------------------------------------------------------------
    /// Commons for download and upload

    fn update_bitfield(&mut self, stream: &mut TcpStream) -> Result<(), PeerSessionError> {
        let updated_bitfield = self
            .torrent_status
            .get_bitfield()
            .map_err(PeerSessionError::ErrorGettingBitfield)?;

        let indices = updated_bitfield.diff(&self.status.bitfield);

        for index in indices {
            self.message_handler
                .send_have(index as u32, stream)
                .map_err(PeerSessionError::MessageHandlerError)?;
        }

        self.status.bitfield = updated_bitfield;

        let bitfield_msg = Message::new(MessageId::Bitfield, self.status.bitfield.get_vec());
        stream
            .write_all(&bitfield_msg.as_bytes())
            .map_err(|_| PeerSessionError::MessageError(MessageId::Bitfield))?;
        Ok(())
    }

    fn calculate_kilobits_per_second(&self, start_time: DateTime<Local>, size: u64) -> f64 {
        let elapsed_time = Local::now().signed_duration_since(start_time);
        let elapsed_time_in_seconds = match elapsed_time.num_microseconds() {
            Some(x) => x as f64 / 1_000_000.0,
            None => return 0.0,
        };
        (size as f64 / elapsed_time_in_seconds) * 8.0 / 1024.0
    }

    fn update_peer_status(&mut self) -> Result<(), PeerSessionError> {
        self.torrent_status
            .update_peer_session_status(&self.peer, &self.status)
            .map_err(PeerSessionError::ErrorGettingSessionsStatus)?;
        Ok(())
    }

    /// Reads & handles a message from the stream.
    ///
    /// It returns an error if:
    /// - The message could not be read
    fn read_message_from_stream(
        &mut self,
        stream: &mut TcpStream,
    ) -> Result<MessageId, PeerSessionError> {
        let mut length = [0; 4];

        stream
            .read_exact(&mut length)
            .map_err(PeerSessionError::ErrorReadingMessage)?;
        let len = u32::from_be_bytes(length);

        // TODO: solucionar el problema de que el peer puede mandar un mensaje de mas de 16393 bytes. Cuando esta mandando cualquiera.
        // Ahora que en el server la iniciacion esta dentro del Ok() esta fallando en el handshake, mirar ahi tambien.
        if len > BLOCK_SIZE * 10 {
            return Err(PeerSessionError::MessageLengthTooLong);
        }

        if len == 0 {
            return Ok(MessageId::KeepAlive);
        }

        let mut payload = vec![0; (len) as usize];

        stream
            .read_exact(&mut payload)
            .map_err(PeerSessionError::ErrorReadingMessage)?;

        let message =
            Message::from_bytes(&payload).map_err(PeerSessionError::MessageDoesNotExist)?;
        let id = message.id.clone();

        self.handle_message(message, stream)?;
        Ok(id)
    }

    /// Handles a message received from the peer.
    fn handle_message(
        &mut self,
        message: Message,
        stream: &mut TcpStream,
    ) -> Result<(), PeerSessionError> {
        match message.id {
            MessageId::Unchoke => {
                self.status.choked = false;
            }
            MessageId::Choke => {
                self.status.choked = true;
            }
            MessageId::Bitfield => {
                self.bitfield = self.message_handler.handle_bitfield(message);
            }
            MessageId::Piece => {
                let mut block = self.message_handler.handle_piece(message);
                self.piece.append(&mut block);
            }
            MessageId::Request => self.handle_request(message, stream)?,
            MessageId::Have => {
                let index = self.message_handler.handle_have(message);
                self.bitfield.set_bit(index as u32, true);
            }
            _ => {} // TODO: handle other messages,
        }
        Ok(())
    }

    /// Sets read and write timeouts for the stream.
    fn set_stream_timeouts(&self, stream: &mut TcpStream) -> Result<(), PeerSessionError> {
        stream
            .set_read_timeout(Some(Duration::from_secs(
                self.config.read_write_seconds_timeout,
            )))
            .map_err(|_| PeerSessionError::ErrorSettingStreamTimeout)?;

        stream
            .set_write_timeout(Some(Duration::from_secs(
                self.config.read_write_seconds_timeout,
            )))
            .map_err(|_| PeerSessionError::ErrorSettingStreamTimeout)?;
        Ok(())
    }

    /// Handles a piece message received from the peer.
    fn handle_request(
        &mut self,
        message: Message,
        stream: &mut TcpStream,
    ) -> Result<(), PeerSessionError> {
        let mut index: [u8; 4] = [0; 4];
        let mut begin: [u8; 4] = [0; 4];
        let mut length: [u8; 4] = [0; 4];
        index.copy_from_slice(&message.payload[0..4]);
        begin.copy_from_slice(&message.payload[4..8]);
        length.copy_from_slice(&message.payload[8..12]);

        let index = u32::from_be_bytes(index);
        let begin = u32::from_be_bytes(begin);
        let length = u32::from_be_bytes(length);

        let offset = index * self.torrent.piece_length() + begin;

        let upload_start_time = Local::now();

        let block = self
            .torrent_status
            .get_piece(index, offset as u64, length as usize)
            .map_err(PeerSessionError::ErrorGettingPiece)?;

        self.message_handler
            .send_piece(index, begin, &block, stream)
            .map_err(PeerSessionError::MessageHandlerError)?;

        // Calculate upload speed
        let upload_speed = self.calculate_kilobits_per_second(upload_start_time, (length).into());
        self.status.upload_speed = upload_speed;
        self.update_peer_status()?;
        Ok(())
    }

    /// Validates the downloaded piece.
    ///
    /// Checks the piece hash and compares it to the hash in the torrent file.
    fn validate_piece(&self, piece: &[u8], piece_index: u32) -> Result<(), PeerSessionError> {
        let start = (piece_index * 20) as usize;
        let end = start + 20;

        let real_hash = &self.torrent.info.pieces[start..end];
        let real_piece_hash = self.convert_to_hex_string(real_hash);

        let hash = Sha1::digest(piece);
        let res_piece_hash = self.convert_to_hex_string(hash.as_slice());

        if real_piece_hash == res_piece_hash {
            Ok(())
        } else {
            Err(PeerSessionError::PieceHashDoesNotMatch)
        }
    }

    /// Converts a byte array to a hex string.
    fn convert_to_hex_string(&self, bytes: &[u8]) -> String {
        let mut res = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            match write!(&mut res, "{:02x}", b) {
                Ok(()) => (),
                Err(_) => warn!("Error converting bytes to hex string!"),
            }
        }
        res
    }
}
