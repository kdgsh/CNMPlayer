mod app;
mod data;
mod render;
mod tmplayer;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use data::config::Config;
use data::theme_loader::ThemeLoader;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Clear};
use rustls::crypto::ring;
use std::io::{self, Stdout};
use std::time::{Duration, Instant};
use tokio::time::sleep;

struct AppFullscreenBridge<'a> {
    app: &'a mut App,
}

impl tmplayer::HostPlaybackBridge for AppFullscreenBridge<'_> {
    async fn tick(&mut self) {
        self.app.fullscreen_tick_playback().await;
    }

    fn metadata_signature(&self) -> u64 {
        self.app.fullscreen_metadata_signature()
    }

    fn runtime_snapshot(&self) -> tmplayer::HostPlaybackRuntimeSnapshot {
        let runtime = self.app.fullscreen_runtime_snapshot();
        tmplayer::HostPlaybackRuntimeSnapshot {
            current_index: runtime.current_index,
            current_liked: runtime.now_playing_liked,
            state: match runtime.state {
                app::PlaybackRuntimeState::Playing => tmplayer::HostPlaybackState::Playing,
                app::PlaybackRuntimeState::Paused => tmplayer::HostPlaybackState::Paused,
                app::PlaybackRuntimeState::Stopped => tmplayer::HostPlaybackState::Stopped,
            },
            repeat_mode: match runtime.repeat_mode {
                app::PlaybackRepeatMode::Sequence => tmplayer::HostRepeatMode::Sequence,
                app::PlaybackRepeatMode::Shuffle => tmplayer::HostRepeatMode::Shuffle,
                app::PlaybackRepeatMode::LoopAll => tmplayer::HostRepeatMode::LoopAll,
                app::PlaybackRepeatMode::LoopOne => tmplayer::HostRepeatMode::LoopOne,
            },
            position: runtime.position,
        }
    }

    fn snapshot(&mut self) -> tmplayer::HostPlaybackSnapshot {
        let snapshot = self.app.fullscreen_playback_snapshot();

        if snapshot.now_playing.is_none() {
            return tmplayer::HostPlaybackSnapshot {
                playlist: Vec::new(),
                current_index: None,
                current_track: None,
                current_liked: snapshot.now_playing_liked,
                state: tmplayer::HostPlaybackState::Stopped,
                repeat_mode: match snapshot.repeat_mode {
                    app::PlaybackRepeatMode::Sequence => tmplayer::HostRepeatMode::Sequence,
                    app::PlaybackRepeatMode::Shuffle => tmplayer::HostRepeatMode::Shuffle,
                    app::PlaybackRepeatMode::LoopAll => tmplayer::HostRepeatMode::LoopAll,
                    app::PlaybackRepeatMode::LoopOne => tmplayer::HostRepeatMode::LoopOne,
                },
                position: Duration::from_secs(0),
            };
        }

        let playlist = snapshot
            .queue
            .iter()
            .map(|track| tmplayer::FullscreenPlaylistItemSeed {
                id: Some(track.song_id.clone()),
                title: track.title.clone(),
                artist: track.artist.clone(),
                album: track.album.clone(),
                duration: Duration::from_millis(track.duration_ms.max(0) as u64),
            })
            .collect::<Vec<_>>();

        let current_track =
            snapshot
                .now_playing
                .as_ref()
                .map(|track| tmplayer::FullscreenTrackSeed {
                    playlist_index: snapshot.current_index,
                    title: track.title.clone(),
                    artist: track.artist.clone(),
                    album: track.album.clone(),
                    duration: Duration::from_millis(track.duration_ms.max(0) as u64),
                    liked: snapshot.now_playing_liked,
                    cover: track.cover.clone(),
                    lyrics: track.lyrics.clone(),
                });

        tmplayer::HostPlaybackSnapshot {
            playlist,
            current_index: snapshot.current_index,
            current_track,
            current_liked: snapshot.now_playing_liked,
            state: match snapshot.state {
                app::PlaybackRuntimeState::Playing => tmplayer::HostPlaybackState::Playing,
                app::PlaybackRuntimeState::Paused => tmplayer::HostPlaybackState::Paused,
                app::PlaybackRuntimeState::Stopped => tmplayer::HostPlaybackState::Stopped,
            },
            repeat_mode: match snapshot.repeat_mode {
                app::PlaybackRepeatMode::Sequence => tmplayer::HostRepeatMode::Sequence,
                app::PlaybackRepeatMode::Shuffle => tmplayer::HostRepeatMode::Shuffle,
                app::PlaybackRepeatMode::LoopAll => tmplayer::HostRepeatMode::LoopAll,
                app::PlaybackRepeatMode::LoopOne => tmplayer::HostRepeatMode::LoopOne,
            },
            position: snapshot.position,
        }
    }

    fn config_snapshot(&self) -> tmplayer::HostConfigSync {
        self.app.fullscreen_config_snapshot()
    }

    async fn apply_config_sync(&mut self, config: tmplayer::HostConfigSync) {
        self.app.fullscreen_apply_config_sync(config).await;
    }

    async fn toggle_play_pause(&mut self) {
        self.app.fullscreen_toggle_play_pause().await;
    }

    async fn play_previous(&mut self) {
        self.app.fullscreen_play_previous().await;
    }

    async fn play_next(&mut self) {
        self.app.fullscreen_play_next().await;
    }

    async fn play_queue_index(&mut self, index: usize) {
        self.app.fullscreen_play_queue_index(index).await;
    }

    fn seek_to_ratio(&mut self, ratio: f32) {
        self.app.fullscreen_seek_to_ratio(ratio);
    }

    fn toggle_repeat_mode(&mut self) {
        self.app.fullscreen_toggle_repeat_mode();
    }

    async fn toggle_like_current(&mut self) {
        self.app.fullscreen_toggle_like().await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    ring::default_provider().install_default().unwrap();
    let config = Config::load_or_default()?;
    let theme = ThemeLoader::load(&config.theme).unwrap_or_default();
    let mut app = App::new(config, theme).await?;

    let mut terminal = init_terminal()?;
    let run_result = run_app(&mut terminal, &mut app).await;
    restore_terminal(&mut terminal)?;
    run_result
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    Ok(())
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    let mut needs_redraw = true;
    let mut active_graphics_protocol = app.config.graphics_protocol;
    let mut last_draw_at = Instant::now()
        .checked_sub(Duration::from_millis(250))
        .unwrap_or_else(Instant::now);

    loop {
        app.tick().await;

        if app.config.graphics_protocol != active_graphics_protocol {
            active_graphics_protocol = app.config.graphics_protocol;
            needs_redraw = true;
        }

        if app.consume_fullscreen_launch_request() {
            let bootstrap = app.build_fullscreen_bootstrap().await;
            launch_tmplayer_fullscreen(terminal, app, bootstrap).await?;
            needs_redraw = true;
            continue;
        }

        let animating = app.main_should_continuous_redraw();
        let target_fps = if animating {
            app.main_active_fps()
        } else {
            app.main_idle_fps()
        };
        let frame_dt = Duration::from_millis((1000_u64 / u64::from(target_fps.max(1))).max(8));

        if needs_redraw || last_draw_at.elapsed() >= frame_dt {
            terminal.draw(|frame| {
                ui::draw(frame, app);
                ui::draw_settings(frame, app);
            })?;
            last_draw_at = Instant::now();
            needs_redraw = false;
            if app.should_quit {
                app.persist_playback_memory_on_exit();
                break;
            }
        }

        let wait_timeout = if animating {
            frame_dt.saturating_sub(last_draw_at.elapsed())
        } else {
            Duration::from_millis(160)
        };

        if event::poll(wait_timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    app.handle_key(key).await;
                    needs_redraw = true;
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse(mouse).await;
                    needs_redraw = true;
                }
                Event::Resize(_, _) => {
                    needs_redraw = true;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

async fn launch_tmplayer_fullscreen(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    bootstrap: tmplayer::FullscreenBootstrap,
) -> Result<()> {
    play_fullscreen_transition(terminal, app, true).await?;
    app.suspend_main_cava_for_fullscreen();
    restore_terminal(terminal)?;

    let config = app.config.clone();
    let mut bridge = AppFullscreenBridge { app };
    let status_text = match tmplayer::run_fullscreen(&config, bootstrap, Some(&mut bridge)).await {
        Ok(tmplayer::FullscreenExit::BackToHost) => String::new(),
        Ok(tmplayer::FullscreenExit::BackToHostOpenSettings) => {
            bridge.app.open_settings_from_fullscreen();
            String::new()
        }
        Err(err) => format!("TMPlayer 运行失败: {}", err),
    };

    *terminal = init_terminal()?;
    app.resume_main_cava_after_fullscreen();
    play_fullscreen_transition(terminal, app, false).await?;
    if !status_text.is_empty() {
        app.set_runtime_status(status_text);
    }
    Ok(())
}

async fn play_fullscreen_transition(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    opening: bool,
) -> Result<()> {
    let steps: u16 = 10;

    for step in 0..=steps {
        let progress = if opening { step } else { steps - step };
        terminal.draw(|frame| {
            ui::draw(frame, app);

            let full = frame.area();
            if full.height == 0 || full.width == 0 {
                return;
            }

            let bar_h = 5_u16.min(full.height);
            let span = full.height.saturating_sub(bar_h);
            let animated = bar_h + ((span as u32 * progress as u32) / steps as u32) as u16;
            let overlay = Rect {
                x: full.x,
                y: full.y + full.height.saturating_sub(animated),
                width: full.width,
                height: animated,
            };

            frame.render_widget(Clear, overlay);
            frame.render_widget(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(app.theme.color_surface()))
                    .style(Style::default().bg(app.theme.color_base())),
                overlay,
            );
        })?;

        sleep(Duration::from_millis(14)).await;
    }

    Ok(())
}
