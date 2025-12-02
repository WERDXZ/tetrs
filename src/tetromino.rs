//! Tetromino definitions and shapes
//!
//! All 7 standard tetrominoes with their rotations using SRS (Super Rotation System)

use ratatui::style::Color;

/// The 7 tetromino types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TetrominoType {
    I, // Cyan - long bar
    O, // Yellow - square
    T, // Purple - T-shape
    S, // Green - S-shape
    Z, // Red - Z-shape
    J, // Blue - J-shape
    L, // Orange - L-shape
}

impl TetrominoType {
    /// Get the color for this tetromino
    pub fn color(&self) -> Color {
        match self {
            TetrominoType::I => Color::Cyan,
            TetrominoType::O => Color::Yellow,
            TetrominoType::T => Color::Magenta,
            TetrominoType::S => Color::Green,
            TetrominoType::Z => Color::Red,
            TetrominoType::J => Color::Blue,
            TetrominoType::L => Color::Rgb(255, 165, 0), // Orange
        }
    }

    /// Get all tetromino types for bag randomization
    pub fn all() -> [TetrominoType; 7] {
        [
            TetrominoType::I,
            TetrominoType::O,
            TetrominoType::T,
            TetrominoType::S,
            TetrominoType::Z,
            TetrominoType::J,
            TetrominoType::L,
        ]
    }

    /// Get the shape offsets for this tetromino at a given rotation
    /// Returns 4 (row, col) offsets relative to the piece's position
    /// Row increases upward (matching display), col increases rightward
    pub fn shape(&self, rotation: Rotation) -> [(i32, i32); 4] {
        match self {
            TetrominoType::I => match rotation {
                Rotation::North => [(0, -1), (0, 0), (0, 1), (0, 2)],
                Rotation::East => [(1, 1), (0, 1), (-1, 1), (-2, 1)],
                Rotation::South => [(-1, -1), (-1, 0), (-1, 1), (-1, 2)],
                Rotation::West => [(1, 0), (0, 0), (-1, 0), (-2, 0)],
            },
            TetrominoType::O => {
                // O piece doesn't rotate
                [(0, 0), (0, 1), (-1, 0), (-1, 1)]
            }
            TetrominoType::T => match rotation {
                Rotation::North => [(0, -1), (0, 0), (0, 1), (1, 0)],
                Rotation::East => [(1, 0), (0, 0), (-1, 0), (0, 1)],
                Rotation::South => [(0, -1), (0, 0), (0, 1), (-1, 0)],
                Rotation::West => [(1, 0), (0, 0), (-1, 0), (0, -1)],
            },
            // S piece - row-up coordinate system
            // North: .SS    East: S.    South: SS.   West: .S
            //        SS.          SS           .SS        SS
            //                     .S                      S.
            TetrominoType::S => match rotation {
                Rotation::North => [(1, 0), (1, 1), (0, -1), (0, 0)],
                Rotation::East => [(1, 0), (0, 0), (0, 1), (-1, 1)],
                Rotation::South => [(0, 0), (0, 1), (-1, -1), (-1, 0)],
                Rotation::West => [(1, -1), (0, -1), (0, 0), (-1, 0)],
            },
            // Z piece - row-up coordinate system
            // North: ZZ.    East: .Z    South: .ZZ   West: Z.
            //        .ZZ          ZZ           ZZ.        ZZ
            //                     Z.                      .Z
            TetrominoType::Z => match rotation {
                Rotation::North => [(1, -1), (1, 0), (0, 0), (0, 1)],
                Rotation::East => [(1, 1), (0, 0), (0, 1), (-1, 0)],
                Rotation::South => [(0, -1), (0, 0), (-1, 0), (-1, 1)],
                Rotation::West => [(1, 0), (0, -1), (0, 0), (-1, -1)],
            },
            TetrominoType::J => match rotation {
                Rotation::North => [(1, -1), (0, -1), (0, 0), (0, 1)],
                Rotation::East => [(1, 0), (1, 1), (0, 0), (-1, 0)],
                Rotation::South => [(0, -1), (0, 0), (0, 1), (-1, 1)],
                Rotation::West => [(1, 0), (0, 0), (-1, 0), (-1, -1)],
            },
            TetrominoType::L => match rotation {
                Rotation::North => [(1, 1), (0, -1), (0, 0), (0, 1)],
                Rotation::East => [(1, 0), (0, 0), (-1, 0), (-1, 1)],
                Rotation::South => [(0, -1), (0, 0), (0, 1), (-1, -1)],
                Rotation::West => [(1, -1), (1, 0), (0, 0), (-1, 0)],
            },
        }
    }

    /// Get spawn position (row, col) - pieces spawn at top center
    pub fn spawn_position(&self) -> (i32, i32) {
        match self {
            TetrominoType::I => (0, 4),
            TetrominoType::O => (0, 4),
            _ => (1, 4),
        }
    }
}

/// Rotation states (using SRS naming convention)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rotation {
    #[default]
    North, // Spawn state
    East,  // Clockwise from North
    South, // 180 from North
    West,  // Counter-clockwise from North
}

impl Rotation {
    /// Rotate clockwise: North → East → South → West → North
    pub fn cw(&self) -> Rotation {
        match self {
            Rotation::North => Rotation::East,
            Rotation::East => Rotation::South,
            Rotation::South => Rotation::West,
            Rotation::West => Rotation::North,
        }
    }

    /// Rotate counter-clockwise: North → West → South → East → North
    pub fn ccw(&self) -> Rotation {
        match self {
            Rotation::North => Rotation::West,
            Rotation::West => Rotation::South,
            Rotation::South => Rotation::East,
            Rotation::East => Rotation::North,
        }
    }
}

/// Direction for rotation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationDirection {
    Clockwise,
    CounterClockwise,
}
