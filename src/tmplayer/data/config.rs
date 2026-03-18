use crate::tmplayer::data::assets;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_EQ_BANDS_DB: [f32; crate::tmplayer::app::state::EQ_BANDS] = [0.0; crate::tmplayer::app::state::EQ_BANDS];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub theme: String,
    pub ui_fps: u32,
    pub spectrum_hz: u32,
    pub mpris_poll_ms: u64,

    #[serde(default = "default_visualize")]
    pub visualize: VisualizeMode,

    #[serde(default = "default_eq_bands_db")]
    pub eq_bands_db: [f32; crate::tmplayer::app::state::EQ_BANDS],

    #[serde(default)]
    pub transparent_background: bool,

    #[serde(default = "default_album_border")]
    pub album_border: bool,

    #[serde(default)]
    pub kitty_graphics: bool,

    #[serde(default = "default_kitty_cover_scale_percent")]
    pub kitty_cover_scale_percent: u8,

    #[serde(default)]
    pub super_smooth_bar: bool,

    #[serde(default)]
    pub bars_gap: bool,

    #[serde(default = "default_audio_quality")]
    pub audio_quality: AudioQuality,

    #[serde(default = "default_show_hints")]
    pub show_hints: bool,

    #[serde(default = "default_bar_number")]
    pub bar_number: BarNumber,

    #[serde(default = "default_bar_channels")]
    pub bar_channels: BarChannels,

    #[serde(default)]
    pub bar_channel_reverse: bool,

    #[serde(default)]
    pub lyrics_cover_fetch: bool,

    #[serde(default)]
    pub lyrics_cover_download: bool,

    #[serde(default)]
    pub audio_fingerprint: bool,

    #[serde(default)]
    pub acoustid_api_key: String,

    #[serde(default)]
    pub resume_last_position: bool,

    #[serde(default = "default_keybind_search_box")]
    pub keybind_search_box: String,

    #[serde(default = "default_keybind_fullscreen")]
    pub keybind_fullscreen: String,

    #[serde(default = "default_keybind_settings")]
    pub keybind_settings: String,

    #[serde(default = "default_keybind_sidebar")]
    pub keybind_sidebar: String,

    #[serde(default = "default_keybind_quit")]
    pub keybind_quit: String,

    #[serde(default = "default_keybind_prev")]
    pub keybind_prev: String,

    #[serde(default = "default_keybind_next")]
    pub keybind_next: String,

    #[serde(default = "default_keybind_toggle_play_pause")]
    pub keybind_toggle_play_pause: String,

    #[serde(default = "default_keybind_toggle_mode")]
    pub keybind_toggle_mode: String,

    #[serde(default, rename = "default-opening-folder")]
    pub default_opening_folder: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VisualizeMode {
    Bars,
    Oscilloscope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BarChannels {
    Stereo,
    Mono,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarNumber {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "16")]
    N16,
    #[serde(rename = "32")]
    N32,
    #[serde(rename = "48")]
    N48,
    #[serde(rename = "64")]
    N64,
    #[serde(rename = "80")]
    N80,
    #[serde(rename = "96")]
    N96,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioQuality {
    #[serde(rename = "standard")]
    Standard,
    #[serde(rename = "higher")]
    Higher,
    #[serde(rename = "exhigh")]
    Exhigh,
    #[serde(rename = "lossless")]
    Lossless,
    #[serde(rename = "hires")]
    Hires,
    #[serde(rename = "jyeffect")]
    Jyeffect,
    #[serde(rename = "sky")]
    Sky,
    #[serde(rename = "dolby")]
    Dolby,
    #[serde(rename = "jymaster")]
    Jymaster,
}

impl AudioQuality {
    pub const FREE_LEVELS: [Self; 3] = [Self::Standard, Self::Higher, Self::Exhigh];
    pub const ALL_LEVELS: [Self; 9] = [
        Self::Standard,
        Self::Higher,
        Self::Exhigh,
        Self::Lossless,
        Self::Hires,
        Self::Jyeffect,
        Self::Sky,
        Self::Dolby,
        Self::Jymaster,
    ];

    pub fn as_api_level(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Higher => "higher",
            Self::Exhigh => "exhigh",
            Self::Lossless => "lossless",
            Self::Hires => "hires",
            Self::Jyeffect => "jyeffect",
            Self::Sky => "sky",
            Self::Dolby => "dolby",
            Self::Jymaster => "jymaster",
        }
    }

    pub fn clamp_for_vip(self, vip_unlocked: bool) -> Self {
        if vip_unlocked {
            self
        } else {
            match self {
                Self::Standard | Self::Higher | Self::Exhigh => self,
                _ => Self::Exhigh,
            }
        }
    }

    pub fn cycle(self, delta: i32, vip_unlocked: bool) -> Self {
        let options: &[Self] = if vip_unlocked {
            &Self::ALL_LEVELS
        } else {
            &Self::FREE_LEVELS
        };

        let current = self.clamp_for_vip(vip_unlocked);
        let index = options
            .iter()
            .position(|item| *item == current)
            .unwrap_or(0) as i32;
        let next = (index + delta).rem_euclid(options.len() as i32) as usize;
        options[next]
    }
}

fn default_visualize() -> VisualizeMode {
    VisualizeMode::Bars
}

fn default_album_border() -> bool {
    true
}

fn default_eq_bands_db() -> [f32; crate::tmplayer::app::state::EQ_BANDS] {
    DEFAULT_EQ_BANDS_DB
}

fn default_kitty_cover_scale_percent() -> u8 {
    100
}

fn default_bar_number() -> BarNumber {
    BarNumber::Auto
}

fn default_bar_channels() -> BarChannels {
    BarChannels::Mono
}

fn default_audio_quality() -> AudioQuality {
    AudioQuality::Exhigh
}

fn default_show_hints() -> bool {
    true
}

fn default_keybind_search_box() -> String {
    "Ctrl+S".to_string()
}

fn default_keybind_fullscreen() -> String {
    "Ctrl+F".to_string()
}

fn default_keybind_settings() -> String {
    "T".to_string()
}

fn default_keybind_sidebar() -> String {
    "P".to_string()
}

fn is_legacy_sidebar_default(value: &str) -> bool {
    let normalized = value
        .trim()
        .to_ascii_lowercase()
        .replace(' ', "");
    normalized == "alt+b"
}

fn default_keybind_quit() -> String {
    "Q".to_string()
}

fn default_keybind_prev() -> String {
    "Alt+Left".to_string()
}

fn default_keybind_next() -> String {
    "Alt+Right".to_string()
}

fn default_keybind_toggle_play_pause() -> String {
    "Alt+Space".to_string()
}

fn default_keybind_toggle_mode() -> String {
    "Alt+M".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "mocha".to_string(),
            ui_fps: 60,
            spectrum_hz: 60,
            mpris_poll_ms: 100,
            visualize: default_visualize(),
            eq_bands_db: default_eq_bands_db(),
            transparent_background: false,
            album_border: default_album_border(),
            kitty_graphics: false,
            kitty_cover_scale_percent: default_kitty_cover_scale_percent(),
            super_smooth_bar: false,
            bars_gap: false,
            audio_quality: default_audio_quality(),
            show_hints: default_show_hints(),
            bar_number: default_bar_number(),
            bar_channels: default_bar_channels(),
            bar_channel_reverse: false,
            lyrics_cover_fetch: false,
            lyrics_cover_download: false,
            audio_fingerprint: false,
            acoustid_api_key: String::new(),
            resume_last_position: false,
            keybind_search_box: default_keybind_search_box(),
            keybind_fullscreen: default_keybind_fullscreen(),
            keybind_settings: default_keybind_settings(),
            keybind_sidebar: default_keybind_sidebar(),
            keybind_quit: default_keybind_quit(),
            keybind_prev: default_keybind_prev(),
            keybind_next: default_keybind_next(),
            keybind_toggle_play_pause: default_keybind_toggle_play_pause(),
            keybind_toggle_mode: default_keybind_toggle_mode(),
            default_opening_folder: String::new(),
        }
    }
}

impl Config {
    pub fn load_or_default() -> Result<Self> {
        // Ensure assets exist according to the required resolution rules.
        let _ = assets::ensure_assets_ready();
        let path = Self::default_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&raw).unwrap_or_default();

        // Ensure spectrum rate stays in sync with UI fps (one-time sync on load).
        let mut cfg = cfg;
        if cfg.spectrum_hz != cfg.ui_fps {
            let synced = cfg.spectrum_hz.max(cfg.ui_fps);
            cfg.spectrum_hz = synced;
            cfg.ui_fps = synced;
        }

        let mut migrated_legacy_sidebar = false;
        if is_legacy_sidebar_default(&cfg.keybind_sidebar) {
            cfg.keybind_sidebar = default_keybind_sidebar();
            migrated_legacy_sidebar = true;
        }

        // Auto-migrate missing fields into the config file.
        if !raw.contains("default-opening-folder")
            || !raw.contains("resume_last_position")
            || !raw.contains("bar_number")
            || !raw.contains("bar_channels")
            || !raw.contains("bar_channel_reverse")
            || !raw.contains("audio_quality")
            || !raw.contains("show_hints")
            || !raw.contains("keybind_search_box")
            || !raw.contains("keybind_fullscreen")
            || !raw.contains("keybind_settings")
            || !raw.contains("keybind_sidebar")
            || !raw.contains("keybind_quit")
            || !raw.contains("keybind_prev")
            || !raw.contains("keybind_next")
            || !raw.contains("keybind_toggle_play_pause")
            || !raw.contains("keybind_toggle_mode")
            || !raw.contains("spectrum_hz")
            || cfg.spectrum_hz != cfg.ui_fps
            || migrated_legacy_sidebar
        {
            let _ = cfg.save();
        }

        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let _ = assets::ensure_assets_ready();
        let path = Self::default_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let raw = toml::to_string_pretty(self).unwrap_or_default();
        fs::write(path, raw)?;
        Ok(())
    }

    fn default_path() -> PathBuf {
        assets::resolve_config_path()
    }
}
