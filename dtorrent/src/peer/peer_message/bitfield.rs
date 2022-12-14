use std::collections::HashMap;

use crate::torrent_handler::status::PieceStatus;

/// Represents a Bitfield.
///
/// It contains information about the pieces that the peer has.
#[derive(Debug, Clone)]
pub struct Bitfield {
    pub bitfield: Vec<u8>,
}

impl Bitfield {
    pub fn new(bitfield: Vec<u8>) -> Bitfield {
        Bitfield { bitfield }
    }

    /// Returns whether the bitfield has the piece with the given index.
    pub fn has_piece(&self, index: u32) -> bool {
        let byte_index = (index / 8) as usize;
        let byte = self.bitfield[byte_index];

        let bit_index = 7 - (index % 8); // Gets the bit index in the byte (from the right)

        // Moves the corresponding bit to the rightmost side of the byte
        // and then checks if that last bit is 1 or 0
        let bit = (byte >> bit_index) & 1;
        bit != 0
    }

    // Returns whether the bitfield has all the pieces.
    pub fn is_complete(&self) -> bool {
        self.bitfield.iter().all(|byte| *byte == 0b1111_1111)
    }

    /// Creates a bitfield from pieces status
    pub fn from(pieces_status: &HashMap<u32, PieceStatus>) -> Bitfield {
        let bytes_count = (pieces_status.len() + 7) / 8;
        let mut bitfield = vec![0; bytes_count];

        for (piece_index, status) in pieces_status {
            if status == &PieceStatus::Finished {
                let byte_index = (piece_index / 8) as usize;
                let byte = bitfield[byte_index];

                let bit_index = 7 - (piece_index % 8); // Gets the bit index in the byte (from the right)
                let bit = 1 << bit_index; // Shifts 1 to the left bit_index times

                bitfield[byte_index] = byte | bit;
            }
        }

        Self::new(bitfield)
    }

    /// Returns the indices difference between two bitfields of the same size.
    pub fn diff(&self, other: &Bitfield) -> Vec<usize> {
        let mut diff = vec![];

        for (index, byte) in self.bitfield.iter().enumerate() {
            let other_byte = other.bitfield[index];

            for bit_index in 0..8 {
                let bit = 1 << (7 - bit_index);
                let our_bit = (byte & bit) != 0;
                let other_bit = (other_byte & bit) != 0;

                if our_bit != other_bit {
                    diff.push(index * 8 + bit_index as usize);
                }
            }
        }
        diff
    }

    /// Sets the indexth bit to the given value.
    pub fn set_bit(&mut self, index: u32, value: bool) {
        let byte_index = (index / 8) as usize;
        let byte = self.bitfield[byte_index];

        let bit_index = 7 - (index % 8); // Gets the bit index in the byte (from the right)
        let bit = 1 << bit_index; // Shifts 1 to the left bit_index times

        if value {
            self.bitfield[byte_index] = byte | bit;
        } else {
            self.bitfield[byte_index] = byte & !bit;
        }
    }

    pub fn get_vec(&self) -> Vec<u8> {
        self.bitfield.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitfield_has_all_pieces() {
        let bitfield = Bitfield::new(vec![0b11111111, 0b11111111, 0b11111111, 0b11111111]);

        assert!(bitfield.has_piece(4));
    }

    #[test]
    fn test_bitfield_has_one_piece() {
        let bitfield = Bitfield::new(vec![0b00000000, 0b00000010, 0b00000000, 0b00000000]);

        assert!(bitfield.has_piece(14));
    }

    #[test]
    fn test_bitfield_not_has_piece() {
        let bitfield = Bitfield::new(vec![0b11111111, 0b11111111, 0b11111101, 0b11111111]);

        assert!(!bitfield.has_piece(22));
    }

    #[test]
    fn test_bitfield_from_one_piece_finished() {
        let mut pieces_status = HashMap::new();
        for i in 0..8 {
            pieces_status.insert(i, PieceStatus::Free);
        }

        pieces_status.insert(0, PieceStatus::Finished);

        let bitfield = Bitfield::from(&pieces_status);

        assert_eq!(bitfield.get_vec(), vec![0b1000_0000]);
    }

    #[test]
    fn test_bitfield_from_one_piece_finished_in_the_middle() {
        let mut pieces_status = HashMap::new();
        for i in 0..8 {
            pieces_status.insert(i, PieceStatus::Free);
        }

        pieces_status.insert(3, PieceStatus::Finished);

        let bitfield = Bitfield::from(&pieces_status);

        assert_eq!(bitfield.get_vec(), vec![0b0001_0000]);
    }

    #[test]
    fn test_bitfield_from_all_pieces_finished() {
        let mut pieces_status = HashMap::new();
        for i in 0..8 {
            pieces_status.insert(i, PieceStatus::Finished);
        }

        let bitfield = Bitfield::from(&pieces_status);

        assert_eq!(bitfield.get_vec(), vec![0b1111_1111]);
    }

    #[test]
    fn test_from_two_bytes() {
        let mut pieces_status = HashMap::new();
        for i in 0..9 {
            pieces_status.insert(i, PieceStatus::Finished);
        }

        let bitfield = Bitfield::from(&pieces_status);

        assert_eq!(bitfield.get_vec(), vec![0b1111_1111, 0b1000_0000]);
    }

    #[test]
    fn test_from_two_bytes_complete() {
        let mut pieces_status = HashMap::new();
        for i in 0..16 {
            pieces_status.insert(i, PieceStatus::Finished);
        }

        let bitfield = Bitfield::from(&pieces_status);

        assert_eq!(bitfield.get_vec(), vec![0b1111_1111, 0b1111_1111]);
    }

    #[test]
    fn test_diff() {
        let bitfield1 = Bitfield::new(vec![0b11111100, 0b11111111]);
        let bitfield2 = Bitfield::new(vec![0b00011100, 0b00111111]);

        assert_eq!(bitfield2.diff(&bitfield1), vec![0, 1, 2, 8, 9]);
    }

    #[test]
    fn test_equal_diff() {
        let bitfield1 = Bitfield::new(vec![0b11111100, 0b11111111]);
        let bitfield2 = Bitfield::new(vec![0b11111100, 0b11111111]);

        assert_eq!(bitfield2.diff(&bitfield1), vec![]);
    }

    #[test]
    fn test_set_bit_true() {
        let mut bitfield = Bitfield::new(vec![0b00000000]);
        bitfield.set_bit(0, true);

        assert_eq!(bitfield.get_vec(), vec![0b10000000]);
    }

    #[test]
    fn test_set_bit_false() {
        let mut bitfield = Bitfield::new(vec![0b11000000]);
        bitfield.set_bit(1, false);

        assert_eq!(bitfield.get_vec(), vec![0b10000000]);
    }
}
