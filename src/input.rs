//! Input handling with DAS (Delayed Auto Shift) and ARR (Auto Repeat Rate)
//!
//! Uses a polling-based approach that doesn't rely on key release events,
//! which are unreliable on Linux terminals.

use crate::game::Action;
use crate::settings::Settings;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};

/// Time after which we consider a key "released" if no repeat received
const KEY_TIMEOUT: Duration = Duration::from_millis(100);

/// Input handler with DAS/ARR support
pub struct InputHandler {
    /// Last press time for movement keys (for DAS)
    left_state: Option<KeyPressState>,
    right_state: Option<KeyPressState>,
    down_state: Option<KeyPressState>,
    /// Key bindings
    bindings: KeyBindings,
    /// DAS duration
    das: Duration,
    /// ARR duration
    arr: Duration,
}

#[derive(Debug, Clone)]
struct KeyPressState {
    first_press: Instant,
    last_seen: Instant,
    das_triggered: bool,
    last_arr: Option<Instant>,
}

/// Key bindings configuration - supports multiple keys per action
#[derive(Debug, Clone)]
pub struct KeyBindings {
    pub move_left: Vec<KeyCode>,
    pub move_right: Vec<KeyCode>,
    pub soft_drop: Vec<KeyCode>,
    pub hard_drop: Vec<KeyCode>,
    pub rotate_cw: Vec<KeyCode>,
    pub rotate_ccw: Vec<KeyCode>,
    pub hold: Vec<KeyCode>,
    pub pause: Vec<KeyCode>,
    pub quit: Vec<KeyCode>,
}

impl KeyBindings {
    /// Parse a key string into KeyCode
    fn parse_key(s: &str) -> KeyCode {
        match s.to_lowercase().as_str() {
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "space" => KeyCode::Char(' '),
            "enter" => KeyCode::Enter,
            "tab" => KeyCode::Tab,
            "esc" | "escape" => KeyCode::Esc,
            "shift" => KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftShift),
            "ctrl" | "control" => KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftControl),
            "alt" => KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftAlt),
            s if s.len() == 1 => KeyCode::Char(s.chars().next().unwrap()),
            _ => KeyCode::Char(' '), // fallback
        }
    }

    /// Parse a list of key strings into KeyCodes
    fn parse_keys(keys: &[String]) -> Vec<KeyCode> {
        keys.iter().map(|s| Self::parse_key(s)).collect()
    }

    /// Create keybindings from settings
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            move_left: Self::parse_keys(&settings.keys.move_left),
            move_right: Self::parse_keys(&settings.keys.move_right),
            soft_drop: Self::parse_keys(&settings.keys.soft_drop),
            hard_drop: Self::parse_keys(&settings.keys.hard_drop),
            rotate_cw: Self::parse_keys(&settings.keys.rotate_cw),
            rotate_ccw: Self::parse_keys(&settings.keys.rotate_ccw),
            hold: Self::parse_keys(&settings.keys.hold),
            pause: Self::parse_keys(&settings.keys.pause),
            quit: Self::parse_keys(&settings.keys.quit),
        }
    }

}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            move_left: vec![KeyCode::Left],
            move_right: vec![KeyCode::Right],
            soft_drop: vec![KeyCode::Down],
            hard_drop: vec![KeyCode::Char(' ')],
            rotate_cw: vec![KeyCode::Up, KeyCode::Char('x')],
            rotate_ccw: vec![KeyCode::Char('z')],
            hold: vec![KeyCode::Char('c')],
            pause: vec![KeyCode::Char('p'), KeyCode::Esc],
            quit: vec![KeyCode::Char('q')],
        }
    }
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            left_state: None,
            right_state: None,
            down_state: None,
            bindings: KeyBindings::default(),
            das: Duration::from_millis(170),
            arr: Duration::from_millis(50),
        }
    }

    /// Create input handler from settings
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            left_state: None,
            right_state: None,
            down_state: None,
            bindings: KeyBindings::from_settings(settings),
            das: Duration::from_millis(settings.gameplay.das_ms),
            arr: Duration::from_millis(settings.gameplay.arr_ms),
        }
    }

    /// Handle a key press event - returns immediate actions
    pub fn key_down(&mut self, key: KeyEvent) -> Vec<Action> {
        let mut actions = Vec::new();
        let now = Instant::now();

        // Handle Ctrl+C for quit
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            actions.push(Action::Quit);
            return actions;
        }

        let code = normalize_key(key.code);

        // Handle movement keys with DAS/ARR tracking
        if self.bindings.move_left.contains(&code) {
            if self.left_state.is_none() {
                // New press - immediate action
                actions.push(Action::MoveLeft);
                self.left_state = Some(KeyPressState {
                    first_press: now,
                    last_seen: now,
                    das_triggered: false,
                    last_arr: None,
                });
            } else if let Some(state) = &mut self.left_state {
                state.last_seen = now;
            }
            // Cancel opposite direction
            self.right_state = None;
        } else if self.bindings.move_right.contains(&code) {
            if self.right_state.is_none() {
                actions.push(Action::MoveRight);
                self.right_state = Some(KeyPressState {
                    first_press: now,
                    last_seen: now,
                    das_triggered: false,
                    last_arr: None,
                });
            } else if let Some(state) = &mut self.right_state {
                state.last_seen = now;
            }
            // Cancel opposite direction
            self.left_state = None;
        } else if self.bindings.soft_drop.contains(&code) {
            if self.down_state.is_none() {
                actions.push(Action::SoftDrop);
                self.down_state = Some(KeyPressState {
                    first_press: now,
                    last_seen: now,
                    das_triggered: false,
                    last_arr: None,
                });
            } else if let Some(state) = &mut self.down_state {
                state.last_seen = now;
            }
        } else if self.bindings.hard_drop.contains(&code) {
            actions.push(Action::HardDrop);
        } else if self.bindings.rotate_cw.contains(&code) {
            actions.push(Action::RotateCW);
        } else if self.bindings.rotate_ccw.contains(&code) {
            actions.push(Action::RotateCCW);
        } else if self.bindings.hold.contains(&code) {
            actions.push(Action::Hold);
        } else if self.bindings.pause.contains(&code) {
            actions.push(Action::Pause);
        } else if self.bindings.quit.contains(&code) {
            actions.push(Action::Quit);
        }

        actions
    }

    /// Handle a key release event (may not be called on Linux)
    pub fn key_up(&mut self, key: KeyEvent) {
        let code = normalize_key(key.code);

        if self.bindings.move_left.contains(&code) {
            self.left_state = None;
        } else if self.bindings.move_right.contains(&code) {
            self.right_state = None;
        } else if self.bindings.soft_drop.contains(&code) {
            self.down_state = None;
        }
    }

    /// Update held keys and return repeat actions (call every frame)
    pub fn update(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();
        let now = Instant::now();

        // Check for timed-out keys (no recent key event = released)
        if let Some(state) = &self.left_state {
            if now.duration_since(state.last_seen) > KEY_TIMEOUT {
                self.left_state = None;
            }
        }
        if let Some(state) = &self.right_state {
            if now.duration_since(state.last_seen) > KEY_TIMEOUT {
                self.right_state = None;
            }
        }
        if let Some(state) = &self.down_state {
            if now.duration_since(state.last_seen) > KEY_TIMEOUT {
                self.down_state = None;
            }
        }

        // Copy DAS/ARR values to avoid borrow issues
        let das = self.das;
        let arr = self.arr;

        // Process DAS/ARR for left
        if let Some(state) = &mut self.left_state {
            if process_das_arr(state, now, das, arr) {
                actions.push(Action::MoveLeft);
            }
        }

        // Process DAS/ARR for right
        if let Some(state) = &mut self.right_state {
            if process_das_arr(state, now, das, arr) {
                actions.push(Action::MoveRight);
            }
        }

        // Process DAS/ARR for soft drop
        if let Some(state) = &mut self.down_state {
            if process_das_arr(state, now, das, arr) {
                actions.push(Action::SoftDrop);
            }
        }

        actions
    }

    /// Clear all held keys (useful for pause/resume)
    pub fn clear(&mut self) {
        self.left_state = None;
        self.right_state = None;
        self.down_state = None;
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Process DAS/ARR logic for a key state, returns true if should trigger action
fn process_das_arr(state: &mut KeyPressState, now: Instant, das: Duration, arr: Duration) -> bool {
    let held_duration = now.duration_since(state.first_press);

    if held_duration >= das {
        if !state.das_triggered {
            // First trigger after DAS
            state.das_triggered = true;
            state.last_arr = Some(now);
            return true;
        } else if let Some(last) = state.last_arr {
            // Subsequent ARR triggers
            if now.duration_since(last) >= arr {
                state.last_arr = Some(now);
                return true;
            }
        }
    }

    false
}

/// Normalize key codes for consistent handling
fn normalize_key(code: KeyCode) -> KeyCode {
    match code {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        other => other,
    }
}
