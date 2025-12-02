//! 7-bag randomizer for piece generation
//!
//! Tetris uses a "7-bag" system where all 7 pieces are shuffled,
//! then dealt out before reshuffling. This prevents long droughts.

use crate::tetromino::TetrominoType;
use rand::seq::SliceRandom;
use rand::thread_rng;

/// The 7-bag piece randomizer
#[derive(Debug, Clone)]
pub struct Bag {
    /// Preview queue for upcoming pieces
    queue: Vec<TetrominoType>,
}

impl Default for Bag {
    fn default() -> Self {
        Self::new()
    }
}

impl Bag {
    /// Create a new bag randomizer with initial queue
    pub fn new() -> Self {
        let mut bag = Self {
            queue: Vec::with_capacity(14),
        };
        // Fill the queue with at least 2 full bags
        bag.refill();
        bag.refill();
        bag
    }

    /// Get the next piece from the queue
    pub fn next(&mut self) -> TetrominoType {
        // Ensure we always have pieces in the queue
        if self.queue.len() <= 7 {
            self.refill();
        }
        self.queue.remove(0)
    }

    /// Preview the next N pieces without removing them
    pub fn preview(&self, count: usize) -> &[TetrominoType] {
        &self.queue[..count.min(self.queue.len())]
    }

    /// Refill the queue with a new shuffled bag
    fn refill(&mut self) {
        let mut new_bag = TetrominoType::all().to_vec();
        new_bag.shuffle(&mut thread_rng());
        self.queue.extend(new_bag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_bag_contains_all_pieces() {
        let mut bag = Bag::new();
        let mut pieces = Vec::new();

        // Get 7 pieces
        for _ in 0..7 {
            pieces.push(bag.next());
        }

        // Should contain all 7 unique pieces
        let unique: HashSet<_> = pieces.iter().collect();
        assert_eq!(unique.len(), 7);
    }

    #[test]
    fn test_preview() {
        let bag = Bag::new();
        let preview = bag.preview(5);
        assert_eq!(preview.len(), 5);
    }

    #[test]
    fn test_many_pieces() {
        let mut bag = Bag::new();
        // Should be able to get many pieces without panicking
        for _ in 0..100 {
            let _ = bag.next();
        }
    }
}
