//! Game board representation and collision detection

use crate::tetromino::TetrominoType;
use ratatui::style::Color;

/// Standard Tetris board dimensions
pub const BOARD_WIDTH: usize = 10;
pub const BOARD_HEIGHT: usize = 20;
/// Hidden rows above the visible board for spawning
pub const BUFFER_HEIGHT: usize = 4;
pub const TOTAL_HEIGHT: usize = BOARD_HEIGHT + BUFFER_HEIGHT;

/// A cell on the board - either empty or filled with a color
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cell {
    #[default]
    Empty,
    Filled(Color),
}

impl Cell {
    pub fn is_empty(&self) -> bool {
        matches!(self, Cell::Empty)
    }

    pub fn is_filled(&self) -> bool {
        matches!(self, Cell::Filled(_))
    }
}

/// The game board
#[derive(Debug, Clone)]
pub struct Board {
    /// Grid stored as [row][col], row 0 is bottom, row increases upward
    cells: [[Cell; BOARD_WIDTH]; TOTAL_HEIGHT],
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Board {
    /// Create a new empty board
    pub fn new() -> Self {
        Self {
            cells: [[Cell::Empty; BOARD_WIDTH]; TOTAL_HEIGHT],
        }
    }

    /// Get the cell at a position (row, col)
    /// Returns None if out of bounds
    pub fn get(&self, row: i32, col: i32) -> Option<Cell> {
        if row < 0 || col < 0 {
            return None;
        }
        let row = row as usize;
        let col = col as usize;
        if row >= TOTAL_HEIGHT || col >= BOARD_WIDTH {
            return None;
        }
        Some(self.cells[row][col])
    }

    /// Set a cell at a position
    /// Returns false if out of bounds
    pub fn set(&mut self, row: i32, col: i32, cell: Cell) -> bool {
        if row < 0 || col < 0 {
            return false;
        }
        let row = row as usize;
        let col = col as usize;
        if row >= TOTAL_HEIGHT || col >= BOARD_WIDTH {
            return false;
        }
        self.cells[row][col] = cell;
        true
    }

    /// Check if a position is valid (within bounds and empty)
    pub fn is_valid_position(&self, row: i32, col: i32) -> bool {
        if col < 0 || col >= BOARD_WIDTH as i32 {
            return false;
        }
        if row < 0 {
            return false;
        }
        if row >= TOTAL_HEIGHT as i32 {
            // Above the board is valid (for spawning)
            return true;
        }
        self.cells[row as usize][col as usize].is_empty()
    }

    /// Check if a set of block positions are all valid
    pub fn are_positions_valid(&self, positions: &[(i32, i32)]) -> bool {
        positions
            .iter()
            .all(|&(row, col)| self.is_valid_position(row, col))
    }

    /// Lock a piece onto the board
    pub fn lock_piece(&mut self, positions: &[(i32, i32)], piece_type: TetrominoType) {
        let color = piece_type.color();
        for &(row, col) in positions {
            self.set(row, col, Cell::Filled(color));
        }
    }

    /// Clear completed lines and return the number cleared
    pub fn clear_lines(&mut self) -> usize {
        let mut lines_cleared = 0;
        let mut write_row = 0;

        for read_row in 0..TOTAL_HEIGHT {
            if !self.is_line_full(read_row) {
                // Keep this line
                if write_row != read_row {
                    self.cells[write_row] = self.cells[read_row];
                }
                write_row += 1;
            } else {
                lines_cleared += 1;
            }
        }

        // Fill the top with empty rows
        for row in write_row..TOTAL_HEIGHT {
            self.cells[row] = [Cell::Empty; BOARD_WIDTH];
        }

        lines_cleared
    }

    /// Check if a line is completely filled
    fn is_line_full(&self, row: usize) -> bool {
        self.cells[row].iter().all(|cell| cell.is_filled())
    }

    /// Check if the board is completely empty (for all-clear detection)
    pub fn is_empty(&self) -> bool {
        self.cells
            .iter()
            .all(|row| row.iter().all(|cell| cell.is_empty()))
    }

    /// Get an iterator over visible rows (bottom to top)
    #[allow(dead_code)]
    pub fn visible_rows(&self) -> impl Iterator<Item = (usize, &[Cell; BOARD_WIDTH])> {
        self.cells[..BOARD_HEIGHT].iter().enumerate()
    }

    /// Check if game is over (blocks in the buffer zone that are locked)
    #[allow(dead_code)]
    pub fn is_topped_out(&self) -> bool {
        // Check if any cells in the visible top rows are filled
        // Game over when pieces stack above row 20
        for row in BOARD_HEIGHT..TOTAL_HEIGHT {
            if self.cells[row].iter().any(|cell| cell.is_filled()) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_board_is_empty() {
        let board = Board::new();
        assert!(board.is_empty());
    }

    #[test]
    fn test_set_and_get() {
        let mut board = Board::new();
        assert!(board.set(5, 5, Cell::Filled(Color::Red)));
        assert_eq!(board.get(5, 5), Some(Cell::Filled(Color::Red)));
    }

    #[test]
    fn test_out_of_bounds() {
        let board = Board::new();
        assert_eq!(board.get(-1, 0), None);
        assert_eq!(board.get(0, -1), None);
        assert_eq!(board.get(TOTAL_HEIGHT as i32, 0), None);
        assert_eq!(board.get(0, BOARD_WIDTH as i32), None);
    }

    #[test]
    fn test_clear_single_line() {
        let mut board = Board::new();
        // Fill the bottom row
        for col in 0..BOARD_WIDTH {
            board.set(0, col as i32, Cell::Filled(Color::Cyan));
        }
        // Add a block on row 1
        board.set(1, 0, Cell::Filled(Color::Red));

        let cleared = board.clear_lines();
        assert_eq!(cleared, 1);
        // The block from row 1 should now be on row 0
        assert_eq!(board.get(0, 0), Some(Cell::Filled(Color::Red)));
        assert!(board.get(1, 0).unwrap().is_empty());
    }
}
