//! Main menu system with settings configuration

use crate::mode::GameMode;
use crate::settings::Settings;

/// Menu screens
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuScreen {
    Main,
    ModeSelect,
    Settings,
    SettingsKeys,
    SettingsVisual,
    SettingsGameplay,
    SettingsAudio,
    Multiplayer,
    HostGame,
    JoinGame,
}

/// Menu state
#[derive(Debug, Clone)]
pub struct Menu {
    pub screen: MenuScreen,
    pub selected: usize,
    pub items: Vec<MenuItem>,
    /// For key rebinding: which action is waiting for input
    pub rebinding: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub item_type: MenuItemType,
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum MenuItemType {
    /// Simple button that triggers an action
    Button(MenuAction),
    /// Toggle boolean setting
    Toggle { key: SettingKey, value: bool },
    /// Cycle through options
    Cycle { key: SettingKey, options: Vec<String>, current: usize },
    /// Numeric value with increment/decrement
    Number { key: SettingKey, value: u64, min: u64, max: u64, step: u64 },
    /// Key binding (shows current keys, can rebind)
    KeyBind { action: String, keys: Vec<String> },
    /// Text input field
    TextInput { value: String, placeholder: String },
    /// Display-only label (not selectable)
    Label { text: String },
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    StartGame(GameMode),
    GoToScreen(MenuScreen),
    Back,
    Quit,
    SaveSettings,
    /// Host a multiplayer game
    HostGame,
    /// Join with the entered ticket
    JoinGame,
}

/// Setting keys for identifying which setting to modify
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingKey {
    ShowGhost,
    BlockStyle,
    DasMs,
    ArrMs,
    BgmVolume,
    SfxVolume,
    BgmTrack,
}

impl Menu {
    pub fn new() -> Self {
        Self::main_menu()
    }

    pub fn main_menu() -> Self {
        Self {
            screen: MenuScreen::Main,
            selected: 0,
            rebinding: None,
            items: vec![
                MenuItem {
                    label: "Play".to_string(),
                    item_type: MenuItemType::Button(MenuAction::GoToScreen(MenuScreen::ModeSelect)),
                },
                MenuItem {
                    label: "Settings".to_string(),
                    item_type: MenuItemType::Button(MenuAction::GoToScreen(MenuScreen::Settings)),
                },
                MenuItem {
                    label: "Quit".to_string(),
                    item_type: MenuItemType::Button(MenuAction::Quit),
                },
            ],
        }
    }

    pub fn mode_select() -> Self {
        Self {
            screen: MenuScreen::ModeSelect,
            selected: 0,
            rebinding: None,
            items: vec![
                MenuItem {
                    label: "Marathon".to_string(),
                    item_type: MenuItemType::Button(MenuAction::StartGame(GameMode::Marathon)),
                },
                MenuItem {
                    label: "Sprint (40 Lines)".to_string(),
                    item_type: MenuItemType::Button(MenuAction::StartGame(GameMode::Sprint)),
                },
                MenuItem {
                    label: "Ultra (3 Minutes)".to_string(),
                    item_type: MenuItemType::Button(MenuAction::StartGame(GameMode::Ultra)),
                },
                MenuItem {
                    label: "Versus (Online)".to_string(),
                    item_type: MenuItemType::Button(MenuAction::GoToScreen(MenuScreen::Multiplayer)),
                },
                MenuItem {
                    label: "Back".to_string(),
                    item_type: MenuItemType::Button(MenuAction::Back),
                },
            ],
        }
    }

    pub fn multiplayer_menu() -> Self {
        Self {
            screen: MenuScreen::Multiplayer,
            selected: 0,
            rebinding: None,
            items: vec![
                MenuItem {
                    label: "Host Game".to_string(),
                    item_type: MenuItemType::Button(MenuAction::GoToScreen(MenuScreen::HostGame)),
                },
                MenuItem {
                    label: "Join Game".to_string(),
                    item_type: MenuItemType::Button(MenuAction::GoToScreen(MenuScreen::JoinGame)),
                },
                MenuItem {
                    label: "Back".to_string(),
                    item_type: MenuItemType::Button(MenuAction::Back),
                },
            ],
        }
    }

    pub fn host_game_menu(ticket: Option<&str>) -> Self {
        let mut items = vec![];

        if let Some(t) = ticket {
            items.push(MenuItem {
                label: "Ticket".to_string(),
                item_type: MenuItemType::Label { text: t.to_string() },
            });
            items.push(MenuItem {
                label: "Waiting for opponent...".to_string(),
                item_type: MenuItemType::Label { text: String::new() },
            });
        } else {
            items.push(MenuItem {
                label: "Start Hosting".to_string(),
                item_type: MenuItemType::Button(MenuAction::HostGame),
            });
        }

        items.push(MenuItem {
            label: "Back".to_string(),
            item_type: MenuItemType::Button(MenuAction::Back),
        });

        Self {
            screen: MenuScreen::HostGame,
            selected: if ticket.is_some() { items.len() - 1 } else { 0 },
            rebinding: None,
            items,
        }
    }

    pub fn join_game_menu() -> Self {
        Self {
            screen: MenuScreen::JoinGame,
            selected: 0,
            rebinding: None,
            items: vec![
                MenuItem {
                    label: "Ticket".to_string(),
                    item_type: MenuItemType::TextInput {
                        value: String::new(),
                        placeholder: "Empty = use clipboard".to_string(),
                    },
                },
                MenuItem {
                    label: "Connect".to_string(),
                    item_type: MenuItemType::Button(MenuAction::JoinGame),
                },
                MenuItem {
                    label: "Back".to_string(),
                    item_type: MenuItemType::Button(MenuAction::Back),
                },
            ],
        }
    }

    /// Get ticket input value from JoinGame menu
    pub fn get_ticket_input(&self) -> Option<String> {
        for item in &self.items {
            if let MenuItemType::TextInput { value, .. } = &item.item_type {
                if !value.is_empty() {
                    return Some(value.clone());
                }
            }
        }
        None
    }

    /// Add character to text input
    pub fn text_input_char(&mut self, c: char) {
        if let Some(item) = self.items.get_mut(self.selected) {
            if let MenuItemType::TextInput { value, .. } = &mut item.item_type {
                value.push(c);
            }
        }
    }

    /// Backspace on text input
    pub fn text_input_backspace(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected) {
            if let MenuItemType::TextInput { value, .. } = &mut item.item_type {
                value.pop();
            }
        }
    }

    /// Paste text into input
    pub fn text_input_paste(&mut self, text: &str) {
        if let Some(item) = self.items.get_mut(self.selected) {
            if let MenuItemType::TextInput { value, .. } = &mut item.item_type {
                value.push_str(text);
            }
        }
    }

    pub fn settings_menu() -> Self {
        Self {
            screen: MenuScreen::Settings,
            selected: 0,
            rebinding: None,
            items: vec![
                MenuItem {
                    label: "Key Bindings".to_string(),
                    item_type: MenuItemType::Button(MenuAction::GoToScreen(MenuScreen::SettingsKeys)),
                },
                MenuItem {
                    label: "Visual".to_string(),
                    item_type: MenuItemType::Button(MenuAction::GoToScreen(MenuScreen::SettingsVisual)),
                },
                MenuItem {
                    label: "Gameplay".to_string(),
                    item_type: MenuItemType::Button(MenuAction::GoToScreen(MenuScreen::SettingsGameplay)),
                },
                MenuItem {
                    label: "Audio".to_string(),
                    item_type: MenuItemType::Button(MenuAction::GoToScreen(MenuScreen::SettingsAudio)),
                },
                MenuItem {
                    label: "Back".to_string(),
                    item_type: MenuItemType::Button(MenuAction::Back),
                },
            ],
        }
    }

    pub fn settings_keys(settings: &Settings) -> Self {
        Self {
            screen: MenuScreen::SettingsKeys,
            selected: 0,
            rebinding: None,
            items: vec![
                MenuItem {
                    label: "Move Left".to_string(),
                    item_type: MenuItemType::KeyBind {
                        action: "move_left".to_string(),
                        keys: settings.keys.move_left.clone(),
                    },
                },
                MenuItem {
                    label: "Move Right".to_string(),
                    item_type: MenuItemType::KeyBind {
                        action: "move_right".to_string(),
                        keys: settings.keys.move_right.clone(),
                    },
                },
                MenuItem {
                    label: "Soft Drop".to_string(),
                    item_type: MenuItemType::KeyBind {
                        action: "soft_drop".to_string(),
                        keys: settings.keys.soft_drop.clone(),
                    },
                },
                MenuItem {
                    label: "Hard Drop".to_string(),
                    item_type: MenuItemType::KeyBind {
                        action: "hard_drop".to_string(),
                        keys: settings.keys.hard_drop.clone(),
                    },
                },
                MenuItem {
                    label: "Rotate CW".to_string(),
                    item_type: MenuItemType::KeyBind {
                        action: "rotate_cw".to_string(),
                        keys: settings.keys.rotate_cw.clone(),
                    },
                },
                MenuItem {
                    label: "Rotate CCW".to_string(),
                    item_type: MenuItemType::KeyBind {
                        action: "rotate_ccw".to_string(),
                        keys: settings.keys.rotate_ccw.clone(),
                    },
                },
                MenuItem {
                    label: "Hold".to_string(),
                    item_type: MenuItemType::KeyBind {
                        action: "hold".to_string(),
                        keys: settings.keys.hold.clone(),
                    },
                },
                MenuItem {
                    label: "Pause".to_string(),
                    item_type: MenuItemType::KeyBind {
                        action: "pause".to_string(),
                        keys: settings.keys.pause.clone(),
                    },
                },
                MenuItem {
                    label: "Back".to_string(),
                    item_type: MenuItemType::Button(MenuAction::Back),
                },
            ],
        }
    }

    pub fn settings_visual(settings: &Settings) -> Self {
        let block_styles = vec!["solid".to_string(), "bracket".to_string(), "round".to_string()];
        let current_style = block_styles.iter()
            .position(|s| s == &settings.visual.block_style)
            .unwrap_or(0);

        Self {
            screen: MenuScreen::SettingsVisual,
            selected: 0,
            rebinding: None,
            items: vec![
                MenuItem {
                    label: "Ghost Piece".to_string(),
                    item_type: MenuItemType::Toggle {
                        key: SettingKey::ShowGhost,
                        value: settings.visual.show_ghost,
                    },
                },
                MenuItem {
                    label: "Block Style".to_string(),
                    item_type: MenuItemType::Cycle {
                        key: SettingKey::BlockStyle,
                        options: block_styles,
                        current: current_style,
                    },
                },
                MenuItem {
                    label: "Back".to_string(),
                    item_type: MenuItemType::Button(MenuAction::Back),
                },
            ],
        }
    }

    pub fn settings_gameplay(settings: &Settings) -> Self {
        Self {
            screen: MenuScreen::SettingsGameplay,
            selected: 0,
            rebinding: None,
            items: vec![
                MenuItem {
                    label: "DAS (ms)".to_string(),
                    item_type: MenuItemType::Number {
                        key: SettingKey::DasMs,
                        value: settings.gameplay.das_ms,
                        min: 0,
                        max: 500,
                        step: 10,
                    },
                },
                MenuItem {
                    label: "ARR (ms)".to_string(),
                    item_type: MenuItemType::Number {
                        key: SettingKey::ArrMs,
                        value: settings.gameplay.arr_ms,
                        min: 0,
                        max: 100,
                        step: 5,
                    },
                },
                MenuItem {
                    label: "Back".to_string(),
                    item_type: MenuItemType::Button(MenuAction::Back),
                },
            ],
        }
    }

    pub fn settings_audio(settings: &Settings) -> Self {
        let bgm_tracks = vec![
            "Korobeiniki".to_string(),
            "Korobeiniki (Fast)".to_string(),
            "Kalinka".to_string(),
            "Ievan Polkka".to_string(),
        ];
        let current_track = bgm_tracks.iter()
            .position(|s| s == &settings.audio.bgm_track)
            .unwrap_or(0);

        Self {
            screen: MenuScreen::SettingsAudio,
            selected: 0,
            rebinding: None,
            items: vec![
                MenuItem {
                    label: "BGM Volume".to_string(),
                    item_type: MenuItemType::Number {
                        key: SettingKey::BgmVolume,
                        value: settings.audio.bgm_volume as u64,
                        min: 0,
                        max: 100,
                        step: 5,
                    },
                },
                MenuItem {
                    label: "SFX Volume".to_string(),
                    item_type: MenuItemType::Number {
                        key: SettingKey::SfxVolume,
                        value: settings.audio.sfx_volume as u64,
                        min: 0,
                        max: 100,
                        step: 5,
                    },
                },
                MenuItem {
                    label: "BGM Track".to_string(),
                    item_type: MenuItemType::Cycle {
                        key: SettingKey::BgmTrack,
                        options: bgm_tracks,
                        current: current_track,
                    },
                },
                MenuItem {
                    label: "Back".to_string(),
                    item_type: MenuItemType::Button(MenuAction::Back),
                },
            ],
        }
    }

    pub fn move_up(&mut self) {
        if self.rebinding.is_some() {
            return; // Don't move while rebinding
        }
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.items.len().saturating_sub(1);
        }
    }

    pub fn move_down(&mut self) {
        if self.rebinding.is_some() {
            return;
        }
        if self.selected < self.items.len() - 1 {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
    }

    /// Handle left/right for cycling options and numbers
    pub fn adjust_left(&mut self, settings: &mut Settings) {
        if self.rebinding.is_some() {
            return;
        }
        if let Some(item) = self.items.get_mut(self.selected) {
            match &mut item.item_type {
                MenuItemType::Toggle { key, value } => {
                    *value = !*value;
                    apply_setting(settings, key, &SettingValue::Bool(*value));
                }
                MenuItemType::Cycle { key, options, current } => {
                    *current = if *current == 0 { options.len() - 1 } else { *current - 1 };
                    apply_setting(settings, key, &SettingValue::String(options[*current].clone()));
                }
                MenuItemType::Number { key, value, min, step, .. } => {
                    *value = value.saturating_sub(*step).max(*min);
                    apply_setting(settings, key, &SettingValue::Number(*value));
                }
                _ => {}
            }
        }
    }

    pub fn adjust_right(&mut self, settings: &mut Settings) {
        if self.rebinding.is_some() {
            return;
        }
        if let Some(item) = self.items.get_mut(self.selected) {
            match &mut item.item_type {
                MenuItemType::Toggle { key, value } => {
                    *value = !*value;
                    apply_setting(settings, key, &SettingValue::Bool(*value));
                }
                MenuItemType::Cycle { key, options, current } => {
                    *current = (*current + 1) % options.len();
                    apply_setting(settings, key, &SettingValue::String(options[*current].clone()));
                }
                MenuItemType::Number { key, value, max, step, .. } => {
                    *value = (*value + *step).min(*max);
                    apply_setting(settings, key, &SettingValue::Number(*value));
                }
                _ => {}
            }
        }
    }

    /// Get the action for the current selection (for Button types)
    pub fn select(&self) -> Option<&MenuAction> {
        if self.rebinding.is_some() {
            return None;
        }
        if let Some(item) = self.items.get(self.selected) {
            if let MenuItemType::Button(action) = &item.item_type {
                return Some(action);
            }
        }
        None
    }

    /// Start rebinding a key
    pub fn start_rebind(&mut self) {
        if let Some(item) = self.items.get(self.selected) {
            if matches!(item.item_type, MenuItemType::KeyBind { .. }) {
                self.rebinding = Some(self.selected);
            }
        }
    }

    /// Cancel rebinding
    pub fn cancel_rebind(&mut self) {
        self.rebinding = None;
    }

    /// Add a key to the current rebinding action (stays in rebind mode)
    pub fn add_key(&mut self, key_str: String, settings: &mut Settings) {
        if let Some(idx) = self.rebinding {
            if let Some(item) = self.items.get_mut(idx) {
                if let MenuItemType::KeyBind { action, keys } = &mut item.item_type {
                    // Add key if not already present
                    if !keys.contains(&key_str) {
                        keys.push(key_str);
                        // Update settings
                        update_key_binding(settings, action, keys.clone());
                    }
                }
            }
        }
        // Stay in rebinding mode to allow adding more keys
    }

    /// Clear keys for current rebinding action and set new key
    pub fn set_key(&mut self, key_str: String, settings: &mut Settings) {
        if let Some(idx) = self.rebinding {
            if let Some(item) = self.items.get_mut(idx) {
                if let MenuItemType::KeyBind { action, keys } = &mut item.item_type {
                    keys.clear();
                    keys.push(key_str);
                    update_key_binding(settings, action, keys.clone());
                }
            }
        }
        self.rebinding = None;
    }

    /// Finish adding keys and exit rebind mode
    pub fn finish_rebind(&mut self) {
        self.rebinding = None;
    }

    pub fn go_to(&mut self, screen: MenuScreen, settings: &Settings) {
        *self = match screen {
            MenuScreen::Main => Self::main_menu(),
            MenuScreen::ModeSelect => Self::mode_select(),
            MenuScreen::Settings => Self::settings_menu(),
            MenuScreen::SettingsKeys => Self::settings_keys(settings),
            MenuScreen::SettingsVisual => Self::settings_visual(settings),
            MenuScreen::SettingsGameplay => Self::settings_gameplay(settings),
            MenuScreen::SettingsAudio => Self::settings_audio(settings),
            MenuScreen::Multiplayer => Self::multiplayer_menu(),
            MenuScreen::HostGame => Self::host_game_menu(None),
            MenuScreen::JoinGame => Self::join_game_menu(),
            _ => Self::main_menu(),
        };
    }

    /// Go back to previous screen
    pub fn go_back(&mut self, settings: &Settings) {
        let prev = match self.screen {
            MenuScreen::Main => MenuScreen::Main,
            MenuScreen::ModeSelect => MenuScreen::Main,
            MenuScreen::Settings => MenuScreen::Main,
            MenuScreen::SettingsKeys => MenuScreen::Settings,
            MenuScreen::SettingsVisual => MenuScreen::Settings,
            MenuScreen::SettingsGameplay => MenuScreen::Settings,
            MenuScreen::SettingsAudio => MenuScreen::Settings,
            MenuScreen::Multiplayer => MenuScreen::ModeSelect,
            MenuScreen::HostGame => MenuScreen::Multiplayer,
            MenuScreen::JoinGame => MenuScreen::Multiplayer,
            _ => MenuScreen::Main,
        };
        self.go_to(prev, settings);
    }
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper enum for setting values
enum SettingValue {
    Bool(bool),
    String(String),
    Number(u64),
}

/// Apply a setting change to the Settings struct
fn apply_setting(settings: &mut Settings, key: &SettingKey, value: &SettingValue) {
    match (key, value) {
        (SettingKey::ShowGhost, SettingValue::Bool(v)) => {
            settings.visual.show_ghost = *v;
        }
        (SettingKey::BlockStyle, SettingValue::String(v)) => {
            settings.visual.block_style = v.clone();
        }
        (SettingKey::DasMs, SettingValue::Number(v)) => {
            settings.gameplay.das_ms = *v;
        }
        (SettingKey::ArrMs, SettingValue::Number(v)) => {
            settings.gameplay.arr_ms = *v;
        }
        (SettingKey::BgmVolume, SettingValue::Number(v)) => {
            settings.audio.bgm_volume = *v as u32;
        }
        (SettingKey::SfxVolume, SettingValue::Number(v)) => {
            settings.audio.sfx_volume = *v as u32;
        }
        (SettingKey::BgmTrack, SettingValue::String(v)) => {
            settings.audio.bgm_track = v.clone();
        }
        _ => {}
    }
}

/// Update a key binding in settings (internal)
fn update_key_binding(settings: &mut Settings, action: &str, keys: Vec<String>) {
    update_key_binding_pub(settings, action, keys);
}

/// Update a key binding in settings (public for main.rs)
pub fn update_key_binding_pub(settings: &mut Settings, action: &str, keys: Vec<String>) {
    match action {
        "move_left" => settings.keys.move_left = keys,
        "move_right" => settings.keys.move_right = keys,
        "soft_drop" => settings.keys.soft_drop = keys,
        "hard_drop" => settings.keys.hard_drop = keys,
        "rotate_cw" => settings.keys.rotate_cw = keys,
        "rotate_ccw" => settings.keys.rotate_ccw = keys,
        "hold" => settings.keys.hold = keys,
        "pause" => settings.keys.pause = keys,
        "quit" => settings.keys.quit = keys,
        _ => {}
    }
}
