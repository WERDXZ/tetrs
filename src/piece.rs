//! Active falling piece logic

use crate::board::Board;
use crate::srs::get_wall_kicks;
use crate::tetromino::{Rotation, RotationDirection, TetrominoType};

/// An active falling piece
#[derive(Debug, Clone)]
pub struct Piece {
    /// The type of tetromino
    pub piece_type: TetrominoType,
    /// Current rotation state
    pub rotation: Rotation,
    /// Position (row, col) of the piece's pivot point
    /// Row 0 is bottom, increases upward
    pub row: i32,
    pub col: i32,
    /// Which wall kick was used for the last rotation (for T-spin detection)
    /// 0 = no kick, 1-5 = kick index
    pub last_kick: u8,
}

impl Piece {
    /// Create a new piece at spawn position
    pub fn new(piece_type: TetrominoType) -> Self {
        let (row, col) = piece_type.spawn_position();
        // Spawn at top of visible area (row 20-21 in a 24-row board)
        Self {
            piece_type,
            rotation: Rotation::North,
            row: 20 + row, // Spawn above visible area
            col,
            last_kick: 0,
        }
    }

    /// Get the absolute positions of all 4 blocks
    pub fn block_positions(&self) -> [(i32, i32); 4] {
        let offsets = self.piece_type.shape(self.rotation);
        offsets.map(|(dr, dc)| (self.row + dr, self.col + dc))
    }

    /// Try to move left, returns true if successful
    pub fn move_left(&mut self, board: &Board) -> bool {
        self.col -= 1;
        if board.are_positions_valid(&self.block_positions()) {
            self.last_kick = 0; // Reset kick tracking on successful move
            true
        } else {
            self.col += 1;
            false
        }
    }

    /// Try to move right, returns true if successful
    pub fn move_right(&mut self, board: &Board) -> bool {
        self.col += 1;
        if board.are_positions_valid(&self.block_positions()) {
            self.last_kick = 0; // Reset kick tracking on successful move
            true
        } else {
            self.col -= 1;
            false
        }
    }

    /// Try to move down, returns true if successful
    pub fn move_down(&mut self, board: &Board) -> bool {
        self.row -= 1;
        if board.are_positions_valid(&self.block_positions()) {
            self.last_kick = 0; // Reset kick tracking on successful move
            true
        } else {
            self.row += 1;
            false
        }
    }

    /// Try to rotate, using SRS wall kicks
    pub fn rotate(&mut self, direction: RotationDirection, board: &Board) -> bool {
        let new_rotation = match direction {
            RotationDirection::Clockwise => self.rotation.cw(),
            RotationDirection::CounterClockwise => self.rotation.ccw(),
        };

        let kicks = get_wall_kicks(self.piece_type, self.rotation, direction);

        // Store original position
        let original_row = self.row;
        let original_col = self.col;
        let original_rotation = self.rotation;

        // Try each kick
        for (kick_idx, (kick_row, kick_col)) in kicks.iter().enumerate() {
            self.row = original_row + kick_row;
            self.col = original_col + kick_col;
            self.rotation = new_rotation;

            if board.are_positions_valid(&self.block_positions()) {
                self.last_kick = (kick_idx + 1) as u8;
                return true;
            }
        }

        // Restore original state
        self.row = original_row;
        self.col = original_col;
        self.rotation = original_rotation;
        false
    }

    /// Hard drop - move down as far as possible and return distance dropped
    pub fn hard_drop(&mut self, board: &Board) -> i32 {
        let mut distance = 0;
        while self.move_down(board) {
            distance += 1;
        }
        distance
    }

    /// Get the ghost piece position (where the piece would land)
    pub fn ghost_row(&self, board: &Board) -> i32 {
        let mut ghost_row = self.row;
        loop {
            ghost_row -= 1;
            let offsets = self.piece_type.shape(self.rotation);
            let positions: Vec<_> = offsets
                .iter()
                .map(|(dr, dc)| (ghost_row + dr, self.col + dc))
                .collect();

            if !board.are_positions_valid(&positions) {
                return ghost_row + 1;
            }
        }
    }

    /// Check if this is a T piece (for T-spin detection)
    pub fn is_t_piece(&self) -> bool {
        matches!(self.piece_type, TetrominoType::T)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_position() {
        let piece = Piece::new(TetrominoType::T);
        // T piece spawns at row 21 (20 + 1), col 4
        assert_eq!(piece.row, 21);
        assert_eq!(piece.col, 4);
    }

    #[test]
    fn test_block_positions() {
        let piece = Piece::new(TetrominoType::O);
        let positions = piece.block_positions();
        // O piece at spawn should have 4 blocks forming a 2x2 square
        assert_eq!(positions.len(), 4);
    }

    #[test]
    fn test_move_down() {
        let board = Board::new();
        let mut piece = Piece::new(TetrominoType::T);
        let original_row = piece.row;
        assert!(piece.move_down(&board));
        assert_eq!(piece.row, original_row - 1);
    }

    #[test]
    fn test_hard_drop() {
        let board = Board::new();
        let mut piece = Piece::new(TetrominoType::I);
        let distance = piece.hard_drop(&board);
        // I piece should drop to the bottom
        assert!(distance > 0);
    }
}
