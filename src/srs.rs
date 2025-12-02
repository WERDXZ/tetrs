//! Super Rotation System (SRS) wall kick data
//!
//! SRS defines the wall kicks attempted when rotating a piece.
//! If a rotation would cause collision, these offsets are tried in order.

use crate::tetromino::{Rotation, RotationDirection, TetrominoType};

/// Get wall kick offsets for a rotation attempt
/// Returns up to 5 (row_offset, col_offset) pairs to try
pub fn get_wall_kicks(
    piece_type: TetrominoType,
    from: Rotation,
    direction: RotationDirection,
) -> [(i32, i32); 5] {
    match piece_type {
        TetrominoType::O => {
            // O piece doesn't rotate, but we return identity kicks
            [(0, 0); 5]
        }
        TetrominoType::I => i_piece_kicks(from, direction),
        _ => jlstz_kicks(from, direction),
    }
}

/// Wall kicks for J, L, S, T, Z pieces
/// Standard SRS with row-up coordinate system (row offsets negated from spec)
fn jlstz_kicks(from: Rotation, direction: RotationDirection) -> [(i32, i32); 5] {
    use Rotation::*;
    use RotationDirection::*;

    // Kicks are (row_offset, col_offset) where row+ is up, col+ is right
    match (from, direction) {
        // 0→R (North → East CW)
        (North, Clockwise) => [(0, 0), (0, -1), (1, -1), (-2, 0), (-2, -1)],
        // R→0 (East → North CCW)
        (East, CounterClockwise) => [(0, 0), (0, 1), (-1, 1), (2, 0), (2, 1)],
        // R→2 (East → South CW)
        (East, Clockwise) => [(0, 0), (0, 1), (-1, 1), (2, 0), (2, 1)],
        // 2→R (South → East CCW)
        (South, CounterClockwise) => [(0, 0), (0, -1), (1, -1), (-2, 0), (-2, -1)],
        // 2→L (South → West CW)
        (South, Clockwise) => [(0, 0), (0, 1), (1, 1), (-2, 0), (-2, 1)],
        // L→2 (West → South CCW)
        (West, CounterClockwise) => [(0, 0), (0, -1), (-1, -1), (2, 0), (2, -1)],
        // L→0 (West → North CW)
        (West, Clockwise) => [(0, 0), (0, -1), (-1, -1), (2, 0), (2, -1)],
        // 0→L (North → West CCW)
        (North, CounterClockwise) => [(0, 0), (0, 1), (1, 1), (-2, 0), (-2, 1)],
    }
}

/// Wall kicks for I piece (different from other pieces)
/// Standard SRS with row-up coordinate system (row offsets negated from spec)
fn i_piece_kicks(from: Rotation, direction: RotationDirection) -> [(i32, i32); 5] {
    use Rotation::*;
    use RotationDirection::*;

    // Kicks are (row_offset, col_offset) where row+ is up, col+ is right
    match (from, direction) {
        // 0→R (North → East CW)
        (North, Clockwise) => [(0, 0), (0, -2), (0, 1), (1, -2), (-2, 1)],
        // R→0 (East → North CCW)
        (East, CounterClockwise) => [(0, 0), (0, 2), (0, -1), (-1, 2), (2, -1)],
        // R→2 (East → South CW)
        (East, Clockwise) => [(0, 0), (0, -1), (0, 2), (-1, -1), (2, 2)],
        // 2→R (South → East CCW)
        (South, CounterClockwise) => [(0, 0), (0, 1), (0, -2), (1, 1), (-2, -2)],
        // 2→L (South → West CW)
        (South, Clockwise) => [(0, 0), (0, 2), (0, -1), (1, 2), (-2, -1)],
        // L→2 (West → South CCW)
        (West, CounterClockwise) => [(0, 0), (0, -2), (0, 1), (-1, -2), (2, 1)],
        // L→0 (West → North CW)
        (West, Clockwise) => [(0, 0), (0, 1), (0, -2), (1, 1), (-2, -2)],
        // 0→L (North → West CCW)
        (North, CounterClockwise) => [(0, 0), (0, -1), (0, 2), (-1, -1), (2, 2)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kick_count() {
        // All rotations should return exactly 5 kicks
        for piece in TetrominoType::all() {
            for from in [
                Rotation::North,
                Rotation::East,
                Rotation::South,
                Rotation::West,
            ] {
                for dir in [RotationDirection::Clockwise, RotationDirection::CounterClockwise] {
                    let kicks = get_wall_kicks(piece, from, dir);
                    assert_eq!(kicks.len(), 5);
                }
            }
        }
    }

    #[test]
    fn test_first_kick_is_identity() {
        // First kick attempt should always be (0, 0) - no offset
        for piece in TetrominoType::all() {
            for from in [
                Rotation::North,
                Rotation::East,
                Rotation::South,
                Rotation::West,
            ] {
                for dir in [RotationDirection::Clockwise, RotationDirection::CounterClockwise] {
                    let kicks = get_wall_kicks(piece, from, dir);
                    assert_eq!(kicks[0], (0, 0));
                }
            }
        }
    }
}
