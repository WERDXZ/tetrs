//! Core game state and logic

use crate::bag::Bag;
use crate::board::{Board, Cell, BOARD_HEIGHT};
use crate::mode::{GameMode, ModeState};
use crate::piece::Piece;
use crate::score::{ClearType, Score};
use crate::tetromino::{RotationDirection, TetrominoType};
use std::time::{Duration, Instant};

/// Info about the last line clear (for garbage calculation in multiplayer)
#[derive(Debug, Clone)]
pub struct ClearInfo {
    pub lines: u8,
    pub is_tspin: bool,
    pub combo: i32,
    pub back_to_back: bool,
}

/// Lock delay settings
const LOCK_DELAY: Duration = Duration::from_millis(500);
const MAX_LOCK_RESETS: u8 = 15;

/// Game state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Countdown(u8), // 3, 2, 1
    Playing,
    Paused,
    GameOver,
    Victory, // For Sprint mode when 40 lines cleared
}

/// Input actions the game can process
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    MoveLeft,
    MoveRight,
    SoftDrop,
    HardDrop,
    RotateCW,
    RotateCCW,
    Hold,
    Pause,
    Quit,
}

/// The main game struct
pub struct Game {
    /// The game board
    pub board: Board,
    /// Current falling piece
    pub current_piece: Option<Piece>,
    /// Held piece (can swap once per piece)
    pub hold_piece: Option<TetrominoType>,
    /// Whether hold has been used this piece
    hold_used: bool,
    /// Piece bag randomizer
    bag: Bag,
    /// Score tracking
    pub score: Score,
    /// Current game state
    pub state: GameState,
    /// Game mode state
    pub mode_state: ModeState,
    /// Last gravity tick
    last_fall: Instant,
    /// Lock delay timer (Some when piece is touching ground)
    lock_timer: Option<Instant>,
    /// Number of lock resets used
    lock_resets: u8,
    /// Lowest row reached (for lock reset tracking)
    lowest_row: i32,
    /// Last action text to display
    pub last_action: Option<String>,
    /// Soft drop distance this piece (for scoring)
    soft_drop_distance: u32,
    /// Countdown timer
    countdown_start: Option<Instant>,
    /// Flag set when a piece is locked (for multiplayer sync)
    pub piece_just_locked: bool,
    /// Last line clear info for garbage calculation
    pub last_clear_info: Option<ClearInfo>,
}

impl Game {
    /// Create a new game with specified mode
    pub fn new(mode: GameMode) -> Self {
        Self::with_seed(mode, rand::random())
    }

    /// Create a new game with specified mode and seed (for multiplayer)
    pub fn with_seed(mode: GameMode, seed: u64) -> Self {
        let mut bag = Bag::with_seed(seed);
        let first_piece = bag.next();
        let mut score = Score::new();
        score.level = mode.starting_level();

        Self {
            board: Board::new(),
            current_piece: Some(Piece::new(first_piece)),
            hold_piece: None,
            hold_used: false,
            bag,
            score,
            state: GameState::Countdown(3),
            mode_state: ModeState::new(mode),
            last_fall: Instant::now(),
            lock_timer: None,
            lock_resets: 0,
            lowest_row: i32::MAX,
            last_action: None,
            soft_drop_distance: 0,
            countdown_start: Some(Instant::now()),
            piece_just_locked: false,
            last_clear_info: None,
        }
    }

    /// Get the current game mode
    pub fn mode(&self) -> GameMode {
        self.mode_state.mode
    }

    /// Get preview of next pieces
    pub fn preview(&self) -> &[TetrominoType] {
        self.bag.preview(5)
    }

    /// Process an action
    pub fn process_action(&mut self, action: Action) {
        match self.state {
            GameState::Countdown(_) => {
                // No actions during countdown
            }
            GameState::Paused => {
                if action == Action::Pause {
                    self.state = GameState::Playing;
                    self.last_fall = Instant::now();
                }
            }
            GameState::Playing => match action {
                Action::MoveLeft => self.move_left(),
                Action::MoveRight => self.move_right(),
                Action::SoftDrop => self.soft_drop(),
                Action::HardDrop => self.hard_drop(),
                Action::RotateCW => self.rotate(RotationDirection::Clockwise),
                Action::RotateCCW => self.rotate(RotationDirection::CounterClockwise),
                Action::Hold => self.hold(),
                Action::Pause => {
                    self.state = GameState::Paused;
                }
                Action::Quit => {
                    self.state = GameState::GameOver;
                }
            },
            GameState::GameOver | GameState::Victory => {
                // No actions, handled by main loop
            }
        }
    }

    /// Update game state (call every frame)
    pub fn update(&mut self) {
        // Handle countdown
        if let GameState::Countdown(count) = self.state {
            if let Some(start) = self.countdown_start {
                let elapsed = start.elapsed().as_secs();
                let new_count = 3u8.saturating_sub(elapsed as u8);
                if new_count == 0 {
                    self.state = GameState::Playing;
                    self.mode_state.start();
                    self.last_fall = Instant::now();
                    self.countdown_start = None;
                } else if new_count != count {
                    self.state = GameState::Countdown(new_count);
                }
            }
            return;
        }

        if self.state != GameState::Playing {
            return;
        }

        // Update mode timer
        self.mode_state.update();

        // Check for mode completion
        if self.mode_state.is_complete(self.score.lines) {
            self.state = match self.mode_state.mode {
                GameMode::Sprint => GameState::Victory,
                GameMode::Ultra => GameState::GameOver, // Time's up
                GameMode::Marathon => GameState::Playing, // Never ends
                GameMode::Versus => GameState::Playing, // Ends when opponent disconnects/loses
                _ => GameState::Playing,
            };
            if self.state != GameState::Playing {
                return;
            }
        }

        let Some(piece) = &self.current_piece else {
            return;
        };

        // Check if piece is on ground
        let on_ground = !self.board.are_positions_valid(
            &piece
                .block_positions()
                .map(|(r, c)| (r - 1, c))
                .to_vec(),
        );

        if on_ground {
            // Start or check lock timer
            if let Some(lock_start) = self.lock_timer {
                if lock_start.elapsed() >= LOCK_DELAY {
                    self.lock_piece();
                }
            } else {
                self.lock_timer = Some(Instant::now());
            }
        } else {
            // Not on ground, apply gravity
            self.lock_timer = None;
            let fall_speed = Duration::from_secs_f64(self.score.fall_speed());
            if self.last_fall.elapsed() >= fall_speed {
                if let Some(piece) = &mut self.current_piece {
                    piece.move_down(&self.board);
                }
                self.last_fall = Instant::now();
            }
        }
    }

    fn move_left(&mut self) {
        if let Some(piece) = &mut self.current_piece {
            if piece.move_left(&self.board) {
                self.try_reset_lock();
            }
        }
    }

    fn move_right(&mut self) {
        if let Some(piece) = &mut self.current_piece {
            if piece.move_right(&self.board) {
                self.try_reset_lock();
            }
        }
    }

    fn soft_drop(&mut self) {
        if let Some(piece) = &mut self.current_piece {
            if piece.move_down(&self.board) {
                self.soft_drop_distance += 1;
                self.last_fall = Instant::now();
                // Reset lock timer if we moved down
                self.lock_timer = None;
            }
        }
    }

    fn hard_drop(&mut self) {
        if let Some(piece) = &mut self.current_piece {
            let distance = piece.hard_drop(&self.board);
            self.score.add_hard_drop(distance as u32);
            self.lock_piece();
        }
    }

    fn rotate(&mut self, direction: RotationDirection) {
        if let Some(piece) = &mut self.current_piece {
            if piece.rotate(direction, &self.board) {
                self.try_reset_lock();
            }
        }
    }

    fn hold(&mut self) {
        if self.hold_used {
            return;
        }

        let Some(current) = self.current_piece.take() else {
            return;
        };

        let next_piece = if let Some(held) = self.hold_piece.take() {
            self.hold_piece = Some(current.piece_type);
            Piece::new(held)
        } else {
            self.hold_piece = Some(current.piece_type);
            Piece::new(self.bag.next())
        };

        // Check if new piece can spawn
        if !self.board.are_positions_valid(&next_piece.block_positions()) {
            self.state = GameState::GameOver;
            return;
        }

        self.current_piece = Some(next_piece);
        self.hold_used = true;
        self.reset_piece_state();
    }

    /// Try to reset lock delay (limited resets per piece)
    fn try_reset_lock(&mut self) {
        if let Some(piece) = &self.current_piece {
            // Only reset if we're lower than before or haven't used all resets
            if piece.row < self.lowest_row {
                self.lowest_row = piece.row;
                self.lock_resets = 0;
            }

            if self.lock_resets < MAX_LOCK_RESETS && self.lock_timer.is_some() {
                self.lock_timer = Some(Instant::now());
                self.lock_resets += 1;
            }
        }
    }

    /// Lock the current piece and spawn next
    fn lock_piece(&mut self) {
        let Some(piece) = self.current_piece.take() else {
            return;
        };

        // Add soft drop score
        self.score.add_soft_drop(self.soft_drop_distance);

        // Lock piece onto board
        let positions = piece.block_positions();
        self.board.lock_piece(&positions, piece.piece_type);

        // Detect T-spin before clearing lines
        let is_t_spin = self.detect_t_spin(&piece);

        // Clear lines
        let lines_cleared = self.board.clear_lines();
        let all_clear = self.board.is_empty();

        // Track back-to-back before scoring updates it
        let was_back_to_back = self.score.back_to_back;

        // Calculate score
        if lines_cleared > 0 || is_t_spin.is_some() {
            let clear_type = match is_t_spin {
                Some(true) => ClearType::TSpin(lines_cleared as u8),
                Some(false) => ClearType::MiniTSpin(lines_cleared as u8),
                None => ClearType::Regular(lines_cleared as u8),
            };
            self.last_action = Some(self.score.add_clear(clear_type, all_clear));

            // Store clear info for garbage calculation
            self.last_clear_info = Some(ClearInfo {
                lines: lines_cleared as u8,
                is_tspin: is_t_spin.is_some(),
                combo: self.score.combo,
                back_to_back: was_back_to_back && (lines_cleared == 4 || is_t_spin.is_some()),
            });
        } else {
            self.score.reset_combo();
            self.last_action = None;
            self.last_clear_info = None;
        }

        // Flag that piece was locked (for multiplayer sync)
        self.piece_just_locked = true;

        // Spawn next piece
        let next_type = self.bag.next();
        let next_piece = Piece::new(next_type);

        // Check for top out
        if !self.board.are_positions_valid(&next_piece.block_positions()) {
            self.state = GameState::GameOver;
            return;
        }

        // Check for block out (any locked blocks above visible area)
        for (row, _) in &positions {
            if *row >= BOARD_HEIGHT as i32 {
                self.state = GameState::GameOver;
                return;
            }
        }

        self.current_piece = Some(next_piece);
        self.reset_piece_state();
    }

    /// Reset per-piece state
    fn reset_piece_state(&mut self) {
        self.hold_used = false;
        self.lock_timer = None;
        self.lock_resets = 0;
        self.lowest_row = i32::MAX;
        self.last_fall = Instant::now();
        self.soft_drop_distance = 0;
    }

    /// Detect T-spin (returns Some(true) for T-spin, Some(false) for mini T-spin, None for no T-spin)
    fn detect_t_spin(&self, piece: &Piece) -> Option<bool> {
        if !piece.is_t_piece() || piece.last_kick == 0 {
            return None;
        }

        // Check corners around T piece center
        // Note: row increases downward, col increases rightward
        let corners = [
            (piece.row + 1, piece.col - 1), // 0: Down-left
            (piece.row + 1, piece.col + 1), // 1: Down-right
            (piece.row - 1, piece.col - 1), // 2: Up-left
            (piece.row - 1, piece.col + 1), // 3: Up-right
        ];

        let filled_corners: Vec<bool> = corners
            .iter()
            .map(|&(r, c)| {
                self.board
                    .get(r, c)
                    .map(|cell| matches!(cell, Cell::Filled(_)))
                    .unwrap_or(true) // Out of bounds counts as filled
            })
            .collect();

        let filled_count = filled_corners.iter().filter(|&&f| f).count();

        if filled_count >= 3 {
            // Determine front corners based on rotation (where the T points)
            let (front_a, front_b) = match piece.rotation {
                crate::tetromino::Rotation::North => (2, 3), // T points up: up-left, up-right
                crate::tetromino::Rotation::East => (1, 3),  // T points right: down-right, up-right
                crate::tetromino::Rotation::South => (0, 1), // T points down: down-left, down-right
                crate::tetromino::Rotation::West => (0, 2),  // T points left: down-left, up-left
            };

            // T-Spin if front corners filled, Mini T-Spin otherwise
            if filled_corners[front_a] && filled_corners[front_b] {
                Some(true)
            } else {
                // Special case: kick 5 always counts as full T-spin
                if piece.last_kick == 5 {
                    Some(true)
                } else {
                    Some(false)
                }
            }
        } else {
            None
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new(GameMode::Marathon)
    }
}
