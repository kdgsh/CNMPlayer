use crate::app::api::ApiState;
use crate::data::assets;
use crate::data::config::{CacheCleanStrategy, Config};
use anyhow::{anyhow, Context, Result};
use directories::BaseDirs;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioPlayerState {
    Playing,
    Paused,
    Stopped,
}

pub struct AudioPlayer {
    stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    cache_dir: PathBuf,
    current_song_id: Option<String>,
    current_file_path: Option<PathBuf>,
    total_duration: Option<Duration>,
    paused_position: Duration,
    started_at: Option<Instant>,
}

impl AudioPlayer {
    pub fn new(config: &Config) -> Self {
        let cache_root = resolve_cache_root(config);
        let cache_dir = cache_root.join("audio");

        if config.cache.clean_on_startup {
            let _ = cleanup_cache_dir(&cache_dir, &config.cache);
        }
        let _ = fs::create_dir_all(&cache_dir);

        let mut player = Self {
            stream: None,
            handle: None,
            sink: None,
            cache_dir,
            current_song_id: None,
            current_file_path: None,
            total_duration: None,
            paused_position: Duration::from_secs(0),
            started_at: None,
        };

        if let Err(err) = player.ensure_output_stream() {
            log::warn!("audio output unavailable at startup: {}", err);
        }

        player
    }

    pub fn play_song(&mut self, api: &mut ApiState, song_id: &str, quality_level: &str) -> Result<()> {
        self.ensure_output_stream()?;

        let file_path = self.ensure_cached_song(api, song_id, quality_level)?;
        self.play_from_file(song_id, file_path)
    }

    pub fn play_cached_song(&mut self, song_id: &str, quality_level: &str) -> Result<bool> {
        self.ensure_output_stream()?;
        let file_path = self.cached_song_path(song_id, quality_level);
        if !is_nonempty_file(&file_path) {
            return Ok(false);
        }
        self.play_from_file(song_id, file_path)?;
        Ok(true)
    }

    pub fn cached_song_path(&self, song_id: &str, quality_level: &str) -> PathBuf {
        let quality = sanitize_cache_key(quality_level);
        self.cache_dir.join(format!("{}__{}.audio", song_id, quality))
    }

    fn play_from_file(&mut self, song_id: &str, file_path: PathBuf) -> Result<()> {
        let file = File::open(&file_path)
            .with_context(|| format!("open cached audio failed: {}", file_path.display()))?;
        let decoder = Decoder::new(BufReader::new(file))
            .with_context(|| format!("decode cached audio failed: {}", file_path.display()))?;
        let total_duration = decoder.total_duration();

        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| anyhow!("audio output handle not initialized"))?;
        let sink = Sink::try_new(handle).context("create audio sink failed")?;
        sink.append(decoder);
        sink.play();

        if let Some(old) = self.sink.take() {
            old.stop();
        }

        self.sink = Some(sink);
        self.current_song_id = Some(song_id.to_string());
        self.current_file_path = Some(file_path);
        self.total_duration = total_duration;
        self.paused_position = Duration::from_secs(0);
        self.started_at = Some(Instant::now());
        Ok(())
    }

    pub fn toggle_play_pause(&mut self) {
        let Some(sink) = self.sink.as_ref() else {
            return;
        };

        if sink.empty() {
            self.started_at = None;
            return;
        }

        if sink.is_paused() {
            sink.play();
            self.started_at = Some(Instant::now());
        } else {
            self.paused_position = self.position();
            sink.pause();
            self.started_at = None;
        }
    }

    pub fn state(&self) -> AudioPlayerState {
        let Some(sink) = self.sink.as_ref() else {
            return AudioPlayerState::Stopped;
        };

        if sink.empty() {
            return AudioPlayerState::Stopped;
        }

        if sink.is_paused() {
            AudioPlayerState::Paused
        } else {
            AudioPlayerState::Playing
        }
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.current_song_id = None;
        self.current_file_path = None;
        self.total_duration = None;
        self.paused_position = Duration::from_secs(0);
        self.started_at = None;
    }

    pub fn duration(&self) -> Option<Duration> {
        self.total_duration
    }

    pub fn seek_to_ratio(&mut self, ratio: f32, fallback_total: Option<Duration>) -> Result<()> {
        let Some(path) = self.current_file_path.clone() else {
            return Ok(());
        };
        let Some(total) = self.total_duration.or(fallback_total) else {
            return Ok(());
        };

        let target = Duration::from_secs_f32(total.as_secs_f32() * ratio.clamp(0.0, 1.0));
        let was_paused = matches!(self.state(), AudioPlayerState::Paused);

        let file = File::open(&path)
            .with_context(|| format!("open cached audio failed: {}", path.display()))?;
        let decoder = Decoder::new(BufReader::new(file))
            .with_context(|| format!("decode cached audio failed: {}", path.display()))?;

        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| anyhow!("audio output handle not initialized"))?;
        let sink = Sink::try_new(handle).context("create audio sink failed")?;
        sink.append(decoder.skip_duration(target));
        if was_paused {
            sink.pause();
        } else {
            sink.play();
        }

        if let Some(old) = self.sink.take() {
            old.stop();
        }

        self.sink = Some(sink);
        self.paused_position = target;
        self.started_at = if was_paused { None } else { Some(Instant::now()) };
        Ok(())
    }

    pub fn position(&self) -> Duration {
        match self.state() {
            AudioPlayerState::Playing => self
                .started_at
                .map(|started| self.paused_position.saturating_add(started.elapsed()))
                .unwrap_or(self.paused_position),
            AudioPlayerState::Paused | AudioPlayerState::Stopped => self.paused_position,
        }
    }

    fn ensure_output_stream(&mut self) -> Result<()> {
        if self.stream.is_some() && self.handle.is_some() {
            return Ok(());
        }

        let (stream, handle) = OutputStream::try_default().context("open default audio output failed")?;
        self.stream = Some(stream);
        self.handle = Some(handle);
        Ok(())
    }

    fn ensure_cached_song(&self, api: &mut ApiState, song_id: &str, quality_level: &str) -> Result<PathBuf> {
        let path = self.cached_song_path(song_id, quality_level);
        if is_nonempty_file(&path) {
            return Ok(path);
        }

        fs::create_dir_all(&self.cache_dir)
            .with_context(|| format!("create cache dir failed: {}", self.cache_dir.display()))?;

        let stream_url = api.song_stream_url_with_quality(song_id, quality_level)?;
        api.fetch_audio_to_path(&stream_url, &path)?;

        Ok(path)
    }
}

fn is_nonempty_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0) > 0
}

fn sanitize_cache_key(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            out.push(ch);
        }
    }
    if out.is_empty() {
        "exhigh".to_string()
    } else {
        out
    }
}

pub(crate) fn resolve_cache_root(config: &Config) -> PathBuf {
    if let Some(custom) = config
        .cache
        .path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return PathBuf::from(custom);
    }

    system_cache_root().unwrap_or_else(|| assets::resolve_asset_path(Path::new("cache")))
}

fn system_cache_root() -> Option<PathBuf> {
    BaseDirs::new().map(|dirs| dirs.cache_dir().join("cnmplayer"))
}

#[derive(Debug, Clone)]
struct CacheEntry {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

pub(crate) fn cleanup_cache_dir(cache_dir: &Path, policy: &crate::data::config::CacheConfig) -> Result<()> {
    let mut entries = list_cache_entries(cache_dir)?;

    if matches!(policy.clean_strategy, CacheCleanStrategy::Age | CacheCleanStrategy::Both)
        && policy.max_age_days > 0
    {
        let now = SystemTime::now();
        let ttl = Duration::from_secs(policy.max_age_days.saturating_mul(24 * 60 * 60));
        entries.retain(|entry| {
            let expired = now
                .duration_since(entry.modified)
                .map(|elapsed| elapsed > ttl)
                .unwrap_or(false);
            if expired {
                let _ = fs::remove_file(&entry.path);
                return false;
            }
            true
        });
    }

    if matches!(policy.clean_strategy, CacheCleanStrategy::Size | CacheCleanStrategy::Both)
        && policy.max_size_mb > 0
    {
        let limit_bytes = policy.max_size_mb.saturating_mul(1024 * 1024);
        let mut total_bytes = entries.iter().map(|entry| entry.size).sum::<u64>();

        if total_bytes > limit_bytes {
            entries.sort_by_key(|entry| entry.modified);
            for entry in entries {
                if total_bytes <= limit_bytes {
                    break;
                }
                if fs::remove_file(&entry.path).is_ok() {
                    total_bytes = total_bytes.saturating_sub(entry.size);
                }
            }
        }
    }

    Ok(())
}

fn list_cache_entries(cache_dir: &Path) -> Result<Vec<CacheEntry>> {
    let mut out = Vec::new();

    if !cache_dir.is_dir() {
        return Ok(out);
    }

    for entry in fs::read_dir(cache_dir)
        .with_context(|| format!("read cache dir failed: {}", cache_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let metadata = entry
            .metadata()
            .with_context(|| format!("read cache metadata failed: {}", path.display()))?;
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        out.push(CacheEntry {
            path,
            size: metadata.len(),
            modified,
        });
    }

    Ok(out)
}
