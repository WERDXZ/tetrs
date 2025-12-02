//! Game modes: Marathon, Sprint, Ultra

use std::time::{Duration, Instant};

/// Available game modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameMode {
    #[default]
    Marathon, // Endless, level increases every 10 lines
    Sprint,   // Clear 40 lines as fast as possible
    Ultra,    // Score as much as possible in 3 minutes
}

impl GameMode {
    pub fn name(&self) -> &'static str {
        match self {
            GameMode::Marathon => "Marathon",
            GameMode::Sprint => "Sprint",
            GameMode::Ultra => "Ultra",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            GameMode::Marathon => "Endless mode - level up every 10 lines",
            GameMode::Sprint => "Clear 40 lines as fast as possible",
            GameMode::Ultra => "Score as much as you can in 3 minutes",
        }
    }

    pub fn starting_level(&self) -> u32 {
        match self {
            GameMode::Marathon => 1,
            GameMode::Sprint => 5,
            GameMode::Ultra => 5,
        }
    }

    pub fn all() -> &'static [GameMode] {
        &[GameMode::Marathon, GameMode::Sprint, GameMode::Ultra]
    }
}

/// Mode-specific game state
#[derive(Debug, Clone)]
pub struct ModeState {
    pub mode: GameMode,
    pub start_time: Option<Instant>,
    pub elapsed: Duration,
    /// For Sprint: lines remaining
    pub target_lines: u32,
    /// For Ultra: time limit
    pub time_limit: Duration,
}

impl ModeState {
    pub fn new(mode: GameMode) -> Self {
        Self {
            mode,
            start_time: None,
            elapsed: Duration::ZERO,
            target_lines: 40,
            time_limit: Duration::from_secs(180), // 3 minutes
        }
    }

    /// Start the timer
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// Update elapsed time
    pub fn update(&mut self) {
        if let Some(start) = self.start_time {
            self.elapsed = start.elapsed();
        }
    }

    /// Check if game is complete based on mode
    pub fn is_complete(&self, lines_cleared: u32) -> bool {
        match self.mode {
            GameMode::Marathon => false, // Never ends automatically
            GameMode::Sprint => lines_cleared >= self.target_lines,
            GameMode::Ultra => self.elapsed >= self.time_limit,
        }
    }

    /// Get remaining time for Ultra mode (None for other modes)
    pub fn time_remaining(&self) -> Option<Duration> {
        match self.mode {
            GameMode::Ultra => Some(self.time_limit.saturating_sub(self.elapsed)),
            _ => None,
        }
    }

    /// Get lines remaining for Sprint mode (None for other modes)
    pub fn lines_remaining(&self, lines_cleared: u32) -> Option<u32> {
        match self.mode {
            GameMode::Sprint => Some(self.target_lines.saturating_sub(lines_cleared)),
            _ => None,
        }
    }

    /// Format elapsed time as MM:SS.mmm
    pub fn format_time(&self) -> String {
        let total_millis = self.elapsed.as_millis();
        let minutes = total_millis / 60000;
        let seconds = (total_millis % 60000) / 1000;
        let millis = total_millis % 1000;
        format!("{:02}:{:02}.{:03}", minutes, seconds, millis)
    }

    /// Format remaining time for Ultra mode
    pub fn format_remaining(&self) -> Option<String> {
        self.time_remaining().map(|remaining| {
            let total_secs = remaining.as_secs();
            let minutes = total_secs / 60;
            let seconds = total_secs % 60;
            format!("{:02}:{:02}", minutes, seconds)
        })
    }
}
