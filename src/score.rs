//! Scoring system following modern Tetris guidelines

/// Type of line clear for scoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearType {
    /// Regular line clear (1-4 lines)
    Regular(u8),
    /// T-Spin with lines cleared (0-3)
    TSpin(u8),
    /// Mini T-Spin with lines cleared (0-1)
    MiniTSpin(u8),
}

/// Scoring calculation
#[derive(Debug, Clone, Default)]
pub struct Score {
    /// Current score
    pub points: u64,
    /// Current level
    pub level: u32,
    /// Total lines cleared
    pub lines: u32,
    /// Current combo count (-1 = no combo)
    pub combo: i32,
    /// Whether last clear was a "difficult" clear (quad or t-spin)
    pub back_to_back: bool,
}

impl Score {
    pub fn new() -> Self {
        Self {
            points: 0,
            level: 1,
            lines: 0,
            combo: -1,
            back_to_back: false,
        }
    }

    /// Calculate and add score for a line clear
    /// Returns the action name for display
    pub fn add_clear(&mut self, clear_type: ClearType, all_clear: bool) -> String {
        let (base_score, lines, is_difficult, action_name) = match clear_type {
            ClearType::Regular(1) => (100, 1, false, "Single"),
            ClearType::Regular(2) => (300, 2, false, "Double"),
            ClearType::Regular(3) => (500, 3, false, "Triple"),
            ClearType::Regular(4) => (800, 4, true, "Tetris"),
            ClearType::TSpin(0) => (400, 0, true, "T-Spin"),
            ClearType::TSpin(1) => (800, 1, true, "T-Spin Single"),
            ClearType::TSpin(2) => (1200, 2, true, "T-Spin Double"),
            ClearType::TSpin(3) => (1600, 3, true, "T-Spin Triple"),
            ClearType::MiniTSpin(0) => (100, 0, false, "Mini T-Spin"),
            ClearType::MiniTSpin(1) => (200, 1, false, "Mini T-Spin Single"),
            _ => (0, 0, false, ""),
        };

        // Update lines cleared
        self.lines += lines as u32;

        // Update level (every 10 lines)
        self.level = (self.lines / 10) + 1;

        // Calculate score with multipliers
        let mut score = base_score * self.level as u64;

        // Back-to-back bonus (1.5x for consecutive difficult clears)
        if is_difficult {
            if self.back_to_back {
                score = score * 3 / 2;
            }
            self.back_to_back = true;
        } else if lines > 0 {
            self.back_to_back = false;
        }

        // Combo bonus
        if lines > 0 {
            self.combo += 1;
            if self.combo > 0 {
                score += 50 * self.combo as u64 * self.level as u64;
            }
        }

        // All-clear bonus
        if all_clear {
            let all_clear_bonus = match lines {
                1 => 800,
                2 => 1200,
                3 => 1800,
                4 if self.back_to_back => 3200,
                4 => 2000,
                _ => 0,
            };
            score += all_clear_bonus * self.level as u64;
        }

        self.points += score;

        // Build action string
        let mut action = String::from(action_name);
        if self.combo > 0 {
            action.push_str(&format!(" Combo x{}", self.combo));
        }
        if self.back_to_back && is_difficult && lines > 0 {
            action = format!("B2B {}", action);
        }
        if all_clear {
            action.push_str(" ALL CLEAR!");
        }
        action
    }

    /// Add score for soft drop (1 point per cell)
    pub fn add_soft_drop(&mut self, cells: u32) {
        self.points += cells as u64;
    }

    /// Add score for hard drop (2 points per cell)
    pub fn add_hard_drop(&mut self, cells: u32) {
        self.points += cells as u64 * 2;
    }

    /// Reset combo (called when piece locks without clearing lines)
    pub fn reset_combo(&mut self) {
        self.combo = -1;
    }

    /// Get the fall speed in seconds for the current level
    pub fn fall_speed(&self) -> f64 {
        // Tetris Guideline gravity formula
        // Level 1: 1 second per row, Level 20: ~0.01 seconds per row
        let level = self.level.min(20) as f64;
        (0.8 - ((level - 1.0) * 0.007)).powf(level - 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_clear() {
        let mut score = Score::new();
        score.add_clear(ClearType::Regular(1), false);
        assert_eq!(score.points, 100);
        assert_eq!(score.lines, 1);
    }

    #[test]
    fn test_tetris() {
        let mut score = Score::new();
        score.add_clear(ClearType::Regular(4), false);
        assert_eq!(score.points, 800);
        assert_eq!(score.lines, 4);
    }

    #[test]
    fn test_back_to_back() {
        let mut score = Score::new();
        // First tetris
        score.add_clear(ClearType::Regular(4), false);
        assert_eq!(score.points, 800);
        // Second tetris (back-to-back = 1.5x, plus combo bonus of 50)
        score.add_clear(ClearType::Regular(4), false);
        // 800 base + 1200 (800*1.5 b2b) + 50 (combo 1) = 2050
        assert_eq!(score.points, 800 + 1200 + 50);
    }

    #[test]
    fn test_combo() {
        let mut score = Score::new();
        score.add_clear(ClearType::Regular(1), false);
        score.add_clear(ClearType::Regular(1), false);
        // Second clear should have combo bonus
        assert!(score.points > 200);
    }

    #[test]
    fn test_level_up() {
        let mut score = Score::new();
        for _ in 0..10 {
            score.add_clear(ClearType::Regular(1), false);
        }
        assert_eq!(score.level, 2);
    }
}
