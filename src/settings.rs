//! Settings persistence using TOML
//!
//! Stores settings in ~/.config/tetrs/settings.toml (or platform equivalent)

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Game settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Keybindings
    pub keys: KeyBindings,
    /// Visual settings
    pub visual: VisualSettings,
    /// Gameplay settings
    pub gameplay: GameplaySettings,
    /// Audio settings
    pub audio: AudioSettings,
    /// High scores
    pub high_scores: HighScores,
}

/// Key bindings (stored as strings for easy editing)
/// Each action can have one or more keys bound to it
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyBindings {
    #[serde(deserialize_with = "deserialize_keys", serialize_with = "serialize_keys")]
    pub move_left: Vec<String>,
    #[serde(deserialize_with = "deserialize_keys", serialize_with = "serialize_keys")]
    pub move_right: Vec<String>,
    #[serde(deserialize_with = "deserialize_keys", serialize_with = "serialize_keys")]
    pub soft_drop: Vec<String>,
    #[serde(deserialize_with = "deserialize_keys", serialize_with = "serialize_keys")]
    pub hard_drop: Vec<String>,
    #[serde(deserialize_with = "deserialize_keys", serialize_with = "serialize_keys")]
    pub rotate_cw: Vec<String>,
    #[serde(deserialize_with = "deserialize_keys", serialize_with = "serialize_keys")]
    pub rotate_ccw: Vec<String>,
    #[serde(deserialize_with = "deserialize_keys", serialize_with = "serialize_keys")]
    pub hold: Vec<String>,
    #[serde(deserialize_with = "deserialize_keys", serialize_with = "serialize_keys")]
    pub pause: Vec<String>,
    #[serde(deserialize_with = "deserialize_keys", serialize_with = "serialize_keys")]
    pub quit: Vec<String>,
}

/// Deserialize keys as either a single string or array of strings
fn deserialize_keys<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct KeysVisitor;

    impl<'de> Visitor<'de> for KeysVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string or array of strings")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![v.to_string()])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut keys = Vec::new();
            while let Some(key) = seq.next_element::<String>()? {
                keys.push(key);
            }
            Ok(keys)
        }
    }

    deserializer.deserialize_any(KeysVisitor)
}

/// Serialize keys: single key as string, multiple as array
fn serialize_keys<S>(keys: &Vec<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;

    if keys.len() == 1 {
        serializer.serialize_str(&keys[0])
    } else {
        let mut seq = serializer.serialize_seq(Some(keys.len()))?;
        for key in keys {
            seq.serialize_element(key)?;
        }
        seq.end()
    }
}

/// Visual settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VisualSettings {
    /// Ghost piece visibility
    pub show_ghost: bool,
    /// Block style: "solid", "bracket", "round"
    pub block_style: String,
}

/// Gameplay settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GameplaySettings {
    /// Delayed Auto Shift in milliseconds
    pub das_ms: u64,
    /// Auto Repeat Rate in milliseconds
    pub arr_ms: u64,
}

/// Audio settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    /// BGM volume (0-100)
    pub bgm_volume: u32,
    /// SFX volume (0-100)
    pub sfx_volume: u32,
    /// Selected BGM track name
    pub bgm_track: String,
}

/// High scores for each mode
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct HighScores {
    pub marathon: Vec<ScoreEntry>,
    pub sprint: Vec<ScoreEntry>,
    pub ultra: Vec<ScoreEntry>,
}

/// A single high score entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreEntry {
    pub score: u64,
    pub lines: u32,
    pub level: u32,
    /// For Sprint mode: time in milliseconds
    pub time_ms: Option<u64>,
    /// Date as ISO string
    pub date: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            keys: KeyBindings::default(),
            visual: VisualSettings::default(),
            gameplay: GameplaySettings::default(),
            audio: AudioSettings::default(),
            high_scores: HighScores::default(),
        }
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            move_left: vec!["Left".to_string()],
            move_right: vec!["Right".to_string()],
            soft_drop: vec!["Down".to_string()],
            hard_drop: vec!["Space".to_string()],
            rotate_cw: vec!["Up".to_string(), "x".to_string()],
            rotate_ccw: vec!["z".to_string()],
            hold: vec!["c".to_string(), "Shift".to_string()],
            pause: vec!["p".to_string(), "Esc".to_string()],
            quit: vec!["q".to_string()],
        }
    }
}

impl Default for VisualSettings {
    fn default() -> Self {
        Self {
            show_ghost: true,
            block_style: "solid".to_string(),
        }
    }
}

impl Default for GameplaySettings {
    fn default() -> Self {
        Self {
            das_ms: 170,
            arr_ms: 50,
        }
    }
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            bgm_volume: 25,
            sfx_volume: 50,
            bgm_track: "Korobeiniki".to_string(),
        }
    }
}

impl Settings {
    /// Get the config directory path
    fn config_dir() -> Option<PathBuf> {
        ProjectDirs::from("com", "tetrs", "tetrs").map(|dirs| dirs.config_dir().to_path_buf())
    }

    /// Get the settings file path
    fn settings_path() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("settings.toml"))
    }

    /// Load settings from file, or create default
    pub fn load() -> Self {
        let Some(path) = Self::settings_path() else {
            return Self::default();
        };

        match fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save settings to file
    pub fn save(&self) -> Result<(), String> {
        let Some(dir) = Self::config_dir() else {
            return Err("Could not determine config directory".to_string());
        };

        let Some(path) = Self::settings_path() else {
            return Err("Could not determine settings path".to_string());
        };

        // Create directory if needed
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;

        // Serialize and write
        let contents =
            toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize: {}", e))?;

        fs::write(&path, contents).map_err(|e| format!("Failed to write settings: {}", e))?;

        Ok(())
    }

    /// Add a high score for Marathon mode
    pub fn add_marathon_score(&mut self, score: u64, lines: u32, level: u32) {
        let entry = ScoreEntry {
            score,
            lines,
            level,
            time_ms: None,
            date: chrono_lite_now(),
        };
        self.high_scores.marathon.push(entry);
        self.high_scores.marathon.sort_by(|a, b| b.score.cmp(&a.score));
        self.high_scores.marathon.truncate(10);
    }

    /// Add a high score for Sprint mode (sorted by time, lower is better)
    pub fn add_sprint_score(&mut self, time_ms: u64, lines: u32, level: u32) {
        let entry = ScoreEntry {
            score: 0,
            lines,
            level,
            time_ms: Some(time_ms),
            date: chrono_lite_now(),
        };
        self.high_scores.sprint.push(entry);
        self.high_scores
            .sprint
            .sort_by(|a, b| a.time_ms.cmp(&b.time_ms));
        self.high_scores.sprint.truncate(10);
    }

    /// Add a high score for Ultra mode
    pub fn add_ultra_score(&mut self, score: u64, lines: u32, level: u32) {
        let entry = ScoreEntry {
            score,
            lines,
            level,
            time_ms: None,
            date: chrono_lite_now(),
        };
        self.high_scores.ultra.push(entry);
        self.high_scores.ultra.sort_by(|a, b| b.score.cmp(&a.score));
        self.high_scores.ultra.truncate(10);
    }

    /// Get the best score for Marathon mode
    pub fn best_marathon(&self) -> Option<u64> {
        self.high_scores.marathon.first().map(|e| e.score)
    }

    /// Get the best time for Sprint mode (in ms)
    pub fn best_sprint(&self) -> Option<u64> {
        self.high_scores.sprint.first().and_then(|e| e.time_ms)
    }

    /// Get the best score for Ultra mode
    pub fn best_ultra(&self) -> Option<u64> {
        self.high_scores.ultra.first().map(|e| e.score)
    }
}

/// Simple date string without external crate
fn chrono_lite_now() -> String {
    // Use system time to create a simple timestamp
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Convert to rough date (good enough for display)
    let days = secs / 86400;
    let years = 1970 + days / 365;
    let remaining_days = days % 365;
    let month = remaining_days / 30 + 1;
    let day = remaining_days % 30 + 1;

    format!("{:04}-{:02}-{:02}", years, month, day)
}

impl VisualSettings {
    /// Get the block characters based on style
    pub fn block_chars(&self) -> (&'static str, &'static str) {
        match self.block_style.as_str() {
            "bracket" => ("[]", ".."),
            "round" => ("()", ".."),
            _ => ("██", "░░"), // "solid" or default
        }
    }
}
