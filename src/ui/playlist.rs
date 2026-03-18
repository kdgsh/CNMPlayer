use crate::app::App;
use crate::data::config::Language;
use crate::ui::page_lyrics;
use crate::ui::player_bar;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn draw_playlist(frame: &mut Frame, app: &mut App) {
    app.clear_player_bar_hits();
    app.clear_content_hits();

    let size = frame.size();
    frame.render_widget(
        Block::default().style(base_bg_style(app)),
        size,
    );

    if size.width < 40 || size.height < 14 {
        frame.render_widget(
            Paragraph::new(match app.config.language {
                Language::Zh => "终端窗口过小",
                Language::En => "Terminal too small",
            })
                .style(Style::default().fg(app.theme.color_subtext())),
            size,
        );
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(player_bar::PLAYER_BAR_HEIGHT)])
        .split(size);

    let (content_area, hint_area) = if app.config.show_hints {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(rows[0]);
        (split[0], split[1])
    } else {
        (rows[0], Rect::default())
    };

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
        .split(content_area);

    draw_playlist_header(frame, app, main[0]);
    draw_playlist_tracks(frame, app, main[1]);
    if app.config.page_lyrics {
        let panel_area = page_lyrics::overlay_panel_area(content_area);
        page_lyrics::draw_page_lyrics_panel(frame, app, panel_area);
    }
    if app.config.show_hints {
        draw_playlist_hint(frame, app, hint_area);
    }
    player_bar::draw_collapsed_player_bar(frame, app, rows[1]);
}

fn draw_playlist_header(frame: &mut Frame, app: &mut App, area: Rect) {
    let inner = area.inner(&ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    if inner.width < 6 || inner.height < 3 {
        return;
    }

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length((inner.height * 2).min(26)), Constraint::Min(1)])
        .split(inner);

    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.color_surface()))
            .style(surface_bg_style(app)),
        cols[0],
    );

    let cover_area = cols[0].inner(&ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    if cover_area.width > 0 && cover_area.height > 0 {
        let cover_ascii = app.playlist.cover_ascii(cover_area.width, cover_area.height);
        frame.render_widget(
            Paragraph::new(cover_ascii)
                .alignment(Alignment::Center)
                .style(Style::default().fg(app.theme.color_text())),
            cover_area,
        );
    }

    let info = vec![
        Line::from(Span::styled(
            app.playlist.title.clone(),
            Style::default()
                .fg(app.theme.color_text())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            app.playlist.artist.clone(),
            Style::default().fg(app.theme.color_subtext()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.playlist.description.clone(),
            Style::default().fg(app.theme.color_text()),
        )),
        Line::from(Span::styled(
            format!(
                "{} {} {}",
                match app.config.language {
                    Language::Zh => "共",
                    Language::En => "Total",
                },
                app.playlist.tracks.len(),
                match app.config.language {
                    Language::Zh => "首",
                    Language::En => "tracks",
                }
            ),
            Style::default().fg(app.theme.color_subtext()),
        )),
    ];

    frame.render_widget(
        Paragraph::new(info).wrap(Wrap { trim: true }),
        cols[1].inner(&ratatui::layout::Margin {
            horizontal: 1,
            vertical: 0,
        }),
    );
}

fn draw_playlist_tracks(frame: &mut Frame, app: &mut App, area: Rect) {
    let inner = area.inner(&ratatui::layout::Margin {
        horizontal: 1,
        vertical: 0,
    });
    if inner.width < 8 || inner.height < 3 {
        return;
    }

    frame.render_widget(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(app.theme.color_surface())),
        area,
    );

    let visible = inner.height as usize;
    let offset = app
        .playlist
        .focused_idx
        .saturating_sub(visible.saturating_sub(1));

    for (line_idx, track_idx) in (offset..app.playlist.tracks.len()).take(visible).enumerate() {
        let y = inner.y + line_idx as u16;
        let row = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };

        app.push_playlist_track_hit(
            crate::app::HitRect {
                x: row.x,
                y: row.y,
                width: row.width,
                height: row.height,
            },
            track_idx,
        );

        let track = &app.playlist.tracks[track_idx];
        let focused = track_idx == app.playlist.focused_idx;
        let is_now_playing = app.is_now_playing_song(track.id.as_deref());
        let zebra_bg = if app.config.transparent_background {
            None
        } else if track_idx % 2 == 0 {
            Some(app.theme.color_base())
        } else {
            Some(app.theme.color_surface())
        };

        let style = if focused {
            let mut style = Style::default()
                .fg(if is_now_playing {
                    app.theme.color_accent3()
                } else {
                    app.theme.color_accent2()
                })
                .add_modifier(Modifier::BOLD);
            if !app.config.transparent_background {
                style = style.bg(app.theme.color_surface());
            }
            style
        } else {
            let mut style = Style::default().fg(if is_now_playing {
                app.theme.color_accent3()
            } else {
                app.theme.color_text()
            });
            if is_now_playing {
                style = style.add_modifier(Modifier::BOLD);
            }
            if let Some(bg) = zebra_bg {
                style = style.bg(bg);
            }
            style
        };

        let index_label = format!("{:>2}.", track_idx + 1);
        let left = format!("{} - {}", track.title, track.artist);
        let duration = track.duration.clone();
        let reserved = display_width(&index_label) + 1 + display_width(&duration) + 1;
        let max_left = usize::from(row.width).saturating_sub(reserved);
        let clipped_left = clip_to_display_width(&left, max_left);
        let used = display_width(&index_label) + 1 + display_width(&clipped_left) + display_width(&duration);
        let space = usize::from(row.width).saturating_sub(used).max(1);

        let index_style = if focused || is_now_playing {
            style.fg(if is_now_playing {
                app.theme.color_accent3()
            } else {
                app.theme.color_accent2()
            })
        } else {
            style.fg(app.theme.color_subtext())
        };
        let duration_style = if focused || is_now_playing {
            style.fg(if is_now_playing {
                app.theme.color_accent3()
            } else {
                app.theme.color_accent2()
            })
        } else {
            style.fg(app.theme.color_subtext())
        };

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(index_label, index_style),
                Span::styled(" ", style),
                Span::styled(clipped_left, style),
                Span::styled(" ".repeat(space), style),
                Span::styled(duration, duration_style),
            ])),
            row,
        );
    }
}

fn base_bg_style(app: &App) -> Style {
    if app.config.transparent_background {
        Style::default()
    } else {
        Style::default().bg(app.theme.color_base())
    }
}

fn surface_bg_style(app: &App) -> Style {
    if app.config.transparent_background {
        Style::default()
    } else {
        Style::default().bg(app.theme.color_surface())
    }
}

fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

fn clip_to_display_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }

    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let w = ch.width().unwrap_or(0);
        if used + w > max_width {
            break;
        }
        out.push(ch);
        used += w;
    }
    out
}

fn draw_playlist_hint(frame: &mut Frame, app: &App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let text = match app.config.language {
        Language::Zh => format!(
            "Enter 播放/打开专辑  Esc 返回  {} 搜索  {} 全屏",
            app.config.keybind_search_box,
            app.config.keybind_fullscreen
        ),
        Language::En => format!(
            "Enter play/open album  Esc back  {} Search  {} Fullscreen",
            app.config.keybind_search_box,
            app.config.keybind_fullscreen
        ),
    };

    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(app.theme.color_subtext()))
            .alignment(Alignment::Left),
        area,
    );
}
