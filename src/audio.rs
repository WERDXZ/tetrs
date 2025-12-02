//! Audio system for background music and sound effects

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;

/// Sound effect types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sfx {
    SelectMove,
    SelectConfirm,
    SelectBack,
    Countdown,
    Go,
    Single,
    Double,
    Triple,
    Quad,
    TSpinSingle,
    TSpinDouble,
    TSpinTriple,
}

impl Sfx {
    fn filename(&self) -> &'static str {
        match self {
            Sfx::SelectMove => "select_move.wav",
            Sfx::SelectConfirm => "select_confirm.wav",
            Sfx::SelectBack => "select_back.wav",
            Sfx::Countdown => "countdown.wav",
            Sfx::Go => "Go.wav",
            Sfx::Single => "single.wav",
            Sfx::Double => "double.wav",
            Sfx::Triple => "single.wav", // Reuse single for triple
            Sfx::Quad => "quad.wav",
            Sfx::TSpinSingle => "t_spin_single.wav",
            Sfx::TSpinDouble => "t_spin_double.wav",
            Sfx::TSpinTriple => "t_spin_triple.wav",
        }
    }
}

/// Background music tracks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BgmTrack {
    Korobeiniki,
    KorobeinikiFast,
    Kalinka,
    IevanPolkka,
}

impl BgmTrack {
    fn filename(&self) -> &'static str {
        match self {
            BgmTrack::Korobeiniki => "Korobeiniki.wav",
            BgmTrack::KorobeinikiFast => "Korobeiniki_Fast.wav",
            BgmTrack::Kalinka => "Kalinka.wav",
            BgmTrack::IevanPolkka => "Ivean_Polkka.wav",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            BgmTrack::Korobeiniki => "Korobeiniki",
            BgmTrack::KorobeinikiFast => "Korobeiniki (Fast)",
            BgmTrack::Kalinka => "Kalinka",
            BgmTrack::IevanPolkka => "Ievan Polkka",
        }
    }

    pub fn all() -> &'static [BgmTrack] {
        &[
            BgmTrack::Korobeiniki,
            BgmTrack::KorobeinikiFast,
            BgmTrack::Kalinka,
            BgmTrack::IevanPolkka,
        ]
    }
}

/// Audio manager handles all sound playback
pub struct AudioManager {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    bgm_sink: Option<Sink>,
    assets_path: PathBuf,
    bgm_volume: f32,
    sfx_volume: f32,
    current_bgm: Option<BgmTrack>,
    sfx_cache: HashMap<Sfx, Arc<Vec<u8>>>,
}

impl AudioManager {
    /// Create a new audio manager
    pub fn new() -> Option<Self> {
        let (stream, stream_handle) = OutputStream::try_default().ok()?;
        let assets_path = Self::find_assets_path()?;

        Some(Self {
            _stream: stream,
            stream_handle,
            bgm_sink: None,
            assets_path,
            bgm_volume: 0.25,
            sfx_volume: 0.5,
            current_bgm: None,
            sfx_cache: HashMap::new(),
        })
    }

    fn find_assets_path() -> Option<PathBuf> {
        let paths = [
            PathBuf::from("assets"),
            PathBuf::from("./assets"),
            std::env::current_exe()
                .ok()?
                .parent()?
                .join("assets"),
        ];

        paths.iter()
            .find(|p| p.exists() && p.join("bgm").exists())
            .cloned()
    }

    /// Set BGM volume (0.0 to 1.0)
    pub fn set_bgm_volume(&mut self, volume: f32) {
        self.bgm_volume = volume.clamp(0.0, 1.0);
        if let Some(sink) = &self.bgm_sink {
            sink.set_volume(self.bgm_volume);
        }
    }

    /// Set SFX volume (0.0 to 1.0)
    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.sfx_volume = volume.clamp(0.0, 1.0);
    }

    /// Get current BGM volume
    pub fn bgm_volume(&self) -> f32 {
        self.bgm_volume
    }

    /// Get current SFX volume
    pub fn sfx_volume(&self) -> f32 {
        self.sfx_volume
    }

    /// Play background music (loops indefinitely)
    pub fn play_bgm(&mut self, track: BgmTrack) {
        // Don't restart if already playing this track
        if self.current_bgm == Some(track) {
            return;
        }

        self.stop_bgm();

        let path = self.assets_path.join("bgm").join(track.filename());
        let Ok(file) = File::open(&path) else { return };
        let Ok(sink) = Sink::try_new(&self.stream_handle) else { return };
        let Ok(decoder) = Decoder::new(BufReader::new(file)) else { return };

        sink.set_volume(self.bgm_volume);
        sink.append(decoder.repeat_infinite());
        self.bgm_sink = Some(sink);
        self.current_bgm = Some(track);
    }

    /// Stop background music
    pub fn stop_bgm(&mut self) {
        if let Some(sink) = self.bgm_sink.take() {
            sink.stop();
        }
        self.current_bgm = None;
    }

    /// Pause background music
    pub fn pause_bgm(&mut self) {
        if let Some(sink) = &self.bgm_sink {
            sink.pause();
        }
    }

    /// Resume background music
    pub fn resume_bgm(&mut self) {
        if let Some(sink) = &self.bgm_sink {
            sink.play();
        }
    }

    /// Play a sound effect
    pub fn play_sfx(&mut self, sfx: Sfx) {
        if self.sfx_volume <= 0.0 {
            return;
        }

        let path = self.assets_path.join("sfx").join(sfx.filename());

        // Try to play from cache or load new
        if let Ok(file) = File::open(&path) {
            if let Ok(decoder) = Decoder::new(BufReader::new(file)) {
                if let Ok(sink) = Sink::try_new(&self.stream_handle) {
                    sink.set_volume(self.sfx_volume);
                    sink.append(decoder);
                    sink.detach(); // Let it play and clean up automatically
                }
            }
        }
    }

    /// Get current BGM track
    pub fn current_bgm(&self) -> Option<BgmTrack> {
        self.current_bgm
    }
}

impl Default for AudioManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|| {
            // Create a dummy manager if audio init fails
            panic!("Failed to initialize audio system")
        })
    }
}
