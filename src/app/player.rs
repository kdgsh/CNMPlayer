use crate::app::streaming::StreamingReader;
use crate::data::assets;
use crate::data::config::{CacheCleanStrategy, Config};
use crate::tmplayer::app::state::{EQ_BANDS, EQ_FREQS_HZ, EqSettings};
use anyhow::{Context, Result, anyhow};
use directories::BaseDirs;
use rodio::{Decoder, MixerDeviceSink, Player, Source};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::num::NonZero;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::watch::Receiver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioPlayerState {
    Playing,
    Paused,
    Stopped,
}

pub struct AudioPlayer {
    device_sink: Option<MixerDeviceSink>,
    sink: Option<Player>,
    cache_dir: PathBuf,
    current_song_id: Option<String>,
    current_file_path: Option<PathBuf>,
    total_duration: Option<Duration>,
    paused_position: Duration,
    started_at: Option<Instant>,
    eq: EqSettings,
    eq_params: Arc<EqParams>,
    /// Receives buffer progress updates pushed from StreamingReader.
    progress_rx: Option<Receiver<(u64, u64)>>,
}

impl AudioPlayer {
    pub fn new(config: &Config) -> Self {
        let cache_root = resolve_cache_root(config);
        let cache_dir = cache_root.join("audio");
        let eq = EqSettings {
            bands_db: config.eq_bands_db,
        }
        .clamp();
        let eq_params = Arc::new(EqParams::new());
        eq_params.set_from(eq);

        if config.cache.clean_on_startup {
            let _ = cleanup_cache_dir(&cache_dir, &config.cache);
        }
        let _ = fs::create_dir_all(&cache_dir);

        let mut player = Self {
            device_sink: None,
            sink: None,
            cache_dir,
            current_song_id: None,
            current_file_path: None,
            total_duration: None,
            paused_position: Duration::from_secs(0),
            started_at: None,
            eq,
            eq_params,
            progress_rx: None,
        };

        if let Err(err) = player.ensure_output_device_sink() {
            log::warn!("audio output unavailable at startup: {}", err);
        }

        player
    }

    pub fn play_cached_song(&mut self, song_id: &str, quality_level: &str) -> Result<bool> {
        self.ensure_output_device_sink()?;
        let file_path = self.cached_song_path(song_id, quality_level);
        if !is_nonempty_file(&file_path) {
            return Ok(false);
        }
        self.play_from_file(song_id, file_path)?;
        Ok(true)
    }

    pub fn cached_song_path(&self, song_id: &str, quality_level: &str) -> PathBuf {
        let quality = sanitize_cache_key(quality_level);
        self.cache_dir
            .join(format!("{}__{}.audio", song_id, quality))
    }

    fn play_from_file(&mut self, song_id: &str, file_path: PathBuf) -> Result<()> {
        let file = File::open(&file_path)
            .with_context(|| format!("open cached audio failed: {}", file_path.display()))?;
        let decoder = Decoder::new(BufReader::new(file))
            .with_context(|| format!("decode cached audio failed: {}", file_path.display()))?;
        let total_duration = decoder.total_duration();
        let source = EqSource::new(decoder, self.eq_params.clone());

        let device_sink = self
            .device_sink
            .as_ref()
            .ok_or_else(|| anyhow!("audio output device_sink not initialized"))?;
        let player = Player::connect_new(device_sink.mixer());
        player.append(source);
        player.play();

        if let Some(old) = self.sink.take() {
            old.stop();
        }

        self.sink = Some(player);
        self.current_song_id = Some(song_id.to_string());
        self.current_file_path = Some(file_path);
        self.total_duration = total_duration;
        self.paused_position = Duration::from_secs(0);
        self.started_at = Some(Instant::now());
        self.progress_rx = None;
        Ok(())
    }

    /// Play audio from a streaming reader that downloads while playing.
    /// The streaming reader handles the background download.
    /// Progress updates are pushed through `progress_rx`.
    pub fn play_streaming(
        &mut self,
        reader: StreamingReader,
        song_id: &str,
        cache_path: PathBuf,
        progress_rx: Receiver<(u64, u64)>,
    ) -> Result<()> {
        self.ensure_output_device_sink()?;

        let decoder = Decoder::new(reader)
            .with_context(|| format!("decode streaming audio failed for {}", song_id))?;
        let total_duration = decoder.total_duration();
        let source = EqSource::new(decoder, self.eq_params.clone());

        let device_sink = self
            .device_sink
            .as_ref()
            .context("audio output device_sink not initialized")?;
        let player = Player::connect_new(device_sink.mixer());
        player.append(source);
        player.play();

        if let Some(old) = self.sink.take() {
            old.stop();
        }

        self.sink = Some(player);
        self.current_song_id = Some(song_id.to_string());
        self.current_file_path = Some(cache_path);
        self.total_duration = total_duration;
        self.paused_position = Duration::from_secs(0);
        self.started_at = Some(Instant::now());
        self.progress_rx = Some(progress_rx);
        Ok(())
    }

    pub fn set_eq(&mut self, eq: EqSettings) -> Result<()> {
        self.eq = eq.clamp();
        self.eq_params.set_from(self.eq);
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
        self.progress_rx = None;
        self.current_song_id = None;
        self.current_file_path = None;
        self.total_duration = None;
        self.paused_position = Duration::from_secs(0);
        self.started_at = None;
    }

    pub fn duration(&self) -> Option<Duration> {
        self.total_duration
    }

    /// Returns the latest buffered progress (downloaded, total).
    /// Uses watch channel which caches the latest value.
    pub fn recv_progress(&mut self) -> Option<(u64, u64)> {
        self.progress_rx.as_mut().map(|rx| *rx.borrow())
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
        let source = EqSource::new(decoder.skip_duration(target), self.eq_params.clone());

        let device_sink = self
            .device_sink
            .as_ref()
            .ok_or_else(|| anyhow!("audio output device_sink not initialized"))?;
        let player = Player::connect_new(device_sink.mixer());
        player.append(source);
        if was_paused {
            player.pause();
        } else {
            player.play();
        }

        if let Some(old) = self.sink.take() {
            old.stop();
        }

        self.sink = Some(player);
        self.paused_position = target;
        self.started_at = if was_paused {
            None
        } else {
            Some(Instant::now())
        };
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

    fn ensure_output_device_sink(&mut self) -> Result<()> {
        if self.device_sink.is_some() {
            return Ok(());
        }

        let mut device_sink = rodio::DeviceSinkBuilder::open_default_sink()
            .context("open default audio output failed")?;
        device_sink.log_on_drop(false);
        self.device_sink = Some(device_sink);
        Ok(())
    }
}

struct EqParams {
    bands_db_x10: [AtomicI32; EQ_BANDS],
}

impl EqParams {
    fn new() -> Self {
        Self {
            bands_db_x10: std::array::from_fn(|_| AtomicI32::new(0)),
        }
    }

    fn set_from(&self, eq: EqSettings) {
        let eq = eq.clamp();
        for (idx, value) in eq.bands_db.iter().enumerate() {
            self.bands_db_x10[idx].store((value * 10.0).round() as i32, Ordering::Relaxed);
        }
    }

    fn load_db(&self) -> [f32; EQ_BANDS] {
        std::array::from_fn(|idx| self.bands_db_x10[idx].load(Ordering::Relaxed) as f32 / 10.0)
    }

    fn load_db_x10(&self) -> [i32; EQ_BANDS] {
        std::array::from_fn(|idx| self.bands_db_x10[idx].load(Ordering::Relaxed))
    }
}

struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

#[derive(Default, Clone, Copy)]
struct BiquadState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

fn biquad_peaking(fs: f32, f0: f32, q: f32, gain_db: f32) -> BiquadCoeffs {
    let fs = if fs > 0.0 { fs } else { 44100.0 };
    let f0 = f0.clamp(10.0, fs * 0.45);
    let q = q.max(0.001);

    let a = 10.0_f32.powf(gain_db / 40.0);
    let w0 = 2.0 * std::f32::consts::PI * (f0 / fs);
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);

    let b0 = 1.0 + alpha * a;
    let b1 = -2.0 * cos_w0;
    let b2 = 1.0 - alpha * a;
    let a0 = 1.0 + alpha / a;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha / a;

    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

fn biquad_process(coeffs: &BiquadCoeffs, state: &mut BiquadState, input: f32) -> f32 {
    let output = coeffs.b0 * input + coeffs.b1 * state.x1 + coeffs.b2 * state.x2
        - coeffs.a1 * state.y1
        - coeffs.a2 * state.y2;
    state.x2 = state.x1;
    state.x1 = input;
    state.y2 = state.y1;
    state.y1 = output;
    output
}

struct EqSource<S>
where
    S: Source<Item = f32>,
{
    inner: S,
    channels: NonZero<u16>,
    idx: usize,
    params: Arc<EqParams>,
    last_db_x10: [i32; EQ_BANDS],
    coeffs: [BiquadCoeffs; EQ_BANDS],
    states: Vec<BiquadState>,
}

impl<S> EqSource<S>
where
    S: Source<Item = f32>,
{
    fn new(inner: S, params: Arc<EqParams>) -> Self {
        let channels = inner.channels();
        let fs = inner.sample_rate().get() as f32;
        let eq_db = params.load_db();
        let last_db_x10 = params.load_db_x10();
        let coeffs =
            std::array::from_fn(|idx| biquad_peaking(fs, EQ_FREQS_HZ[idx], 1.0, eq_db[idx]));
        let states = vec![BiquadState::default(); (channels.get() as usize) * EQ_BANDS];

        Self {
            inner,
            channels,
            idx: 0,
            params,
            last_db_x10,
            coeffs,
            states,
        }
    }

    fn state_index(&self, channel: usize, band: usize) -> usize {
        channel * EQ_BANDS + band
    }
}

impl<S> Iterator for EqSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.params.load_db_x10();
        if current != self.last_db_x10 {
            let fs = self.inner.sample_rate().get() as f32;
            let eq_db = self.params.load_db();
            self.coeffs =
                std::array::from_fn(|idx| biquad_peaking(fs, EQ_FREQS_HZ[idx], 1.0, eq_db[idx]));
            self.last_db_x10 = current;
        }

        let input = self.inner.next()?;
        let channel =
            (self.idx % (self.channels.get() as usize)).min(self.channels.get() as usize - 1);
        self.idx = self.idx.wrapping_add(1);

        let mut output = input;
        for band in 0..EQ_BANDS {
            let state_idx = self.state_index(channel, band);
            output = biquad_process(&self.coeffs[band], &mut self.states[state_idx], output);
        }
        Some(output)
    }
}

impl<S> Source for EqSource<S>
where
    S: Source<Item = f32>,
{
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> NonZero<u16> {
        self.inner.channels()
    }

    fn sample_rate(&self) -> NonZero<u32> {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
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

pub(crate) fn cleanup_cache_dir(
    cache_dir: &Path,
    policy: &crate::data::config::CacheConfig,
) -> Result<()> {
    let mut entries = list_cache_entries(cache_dir)?;

    if matches!(
        policy.clean_strategy,
        CacheCleanStrategy::Age | CacheCleanStrategy::Both
    ) && policy.max_age_days > 0
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

    if matches!(
        policy.clean_strategy,
        CacheCleanStrategy::Size | CacheCleanStrategy::Both
    ) && policy.max_size_mb > 0
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
