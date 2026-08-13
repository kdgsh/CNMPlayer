use crate::tmplayer::app::state::{AppState, LyricLine};
use crate::tmplayer::data::config::VisualizeMode;
use crate::tmplayer::render::{oscilloscope_renderer, spectrum_renderer};
use crate::tmplayer::ui::borders::SOLID_BORDER;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use unicode_width::UnicodeWidthChar;

pub fn render(f: &mut Frame, lyric_area: Rect, spectrum_area: Rect, app: &mut AppState) {
    let outer = Rect {
        x: lyric_area.x,
        y: lyric_area.y,
        width: lyric_area.width,
        height: lyric_area.height.saturating_add(spectrum_area.height),
    };
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_set(SOLID_BORDER)
        .style(Style::default().fg(app.theme.color_subtext()));
    f.render_widget(outer_block, outer);

    let inner = outer.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if app.config.visualize == VisualizeMode::Off {
        render_full_lyrics(f, inner, app);
        return;
    }

    let lyric_h = lyric_area.height.saturating_sub(2).min(inner.height);
    let lyric_inner = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: lyric_h,
    };
    let spectrum_inner = Rect {
        x: inner.x,
        y: inner.y + lyric_h,
        width: inner.width,
        height: inner.height.saturating_sub(lyric_h),
    };

    let lines = centered_lyric_window(app, lyric_inner.height as usize, lyric_inner.width as usize);
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), lyric_inner);

    match app.config.visualize {
        VisualizeMode::Off => {}
        VisualizeMode::Bars => spectrum_renderer::render(f, spectrum_inner, app),
        VisualizeMode::Oscilloscope => oscilloscope_renderer::render(f, spectrum_inner, app),
    }
}

fn render_full_lyrics(f: &mut Frame, area: Rect, app: &AppState) {
    let lines = centered_lyric_window(app, area.height as usize, area.width as usize);
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

fn centered_lyric_window(app: &AppState, visible_rows: usize, width: usize) -> Vec<Line<'static>> {
    let rows_count = visible_rows.max(1);
    let current_row = rows_count / 2;
    let mut rows = vec![Line::from(String::new()); rows_count];

    let Some(lines) = app.player.track.lyrics.as_ref() else {
        rows[current_row] = Line::from(Span::styled(
            no_lyrics_label(app),
            Style::default().fg(app.theme.color_subtext()),
        ));
        return rows;
    };

    if lines.is_empty() {
        rows[current_row] = Line::from(Span::styled(
            no_lyrics_label(app),
            Style::default().fg(app.theme.color_subtext()),
        ));
        return rows;
    }

    let pos_ms = app.player.position.as_millis() as u64;
    let current_idx = current_lyric_index(lines, pos_ms);
    let wrap_width = width.max(1);

    let mut flat_text: Vec<String> = Vec::new();
    let mut flat_current: Vec<bool> = Vec::new();
    let mut flat_translation: Vec<bool> = Vec::new();
    let mut current_start = 0usize;
    let mut current_text_rows = 0usize;

    for (idx, lyric) in lines.iter().enumerate() {
        let is_current = idx == current_idx;
        let segments = wrap_text(&lyric.text, wrap_width);
        if is_current {
            current_start = flat_text.len();
            current_text_rows = segments.len().max(1);
        }
        for seg in &segments {
            flat_text.push(seg.clone());
            flat_current.push(is_current);
            flat_translation.push(false);
        }
        if app.config.show_lyric_translation
            && let Some(translation) = lyric
                .translation
                .as_ref()
                .filter(|translation| !translation.trim().is_empty())
        {
            for seg in wrap_text(translation, wrap_width) {
                flat_text.push(seg);
                flat_current.push(is_current);
                flat_translation.push(true);
            }
        }
    }

    let current_flat = current_start + current_text_rows.saturating_sub(1) / 2;
    let first_flat = current_flat as isize - current_row as isize;

    for (row, slot) in rows.iter_mut().enumerate() {
        let flat = first_flat + row as isize;
        if flat < 0 || flat >= flat_text.len() as isize {
            continue;
        }
        let flat = flat as usize;
        let is_current = flat_current[flat];
        let is_translation = flat_translation[flat];
        let style = if is_current && !is_translation {
            Style::default()
                .fg(app.theme.color_accent2())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.color_subtext())
        };
        *slot = Line::from(Span::styled(flat_text[flat].clone(), style));
    }

    rows
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    for ch in text.chars() {
        if ch == '\n' {
            lines.push(std::mem::take(&mut current));
            current_width = 0;
            continue;
        }
        let ch_width = ch.width().unwrap_or(0);
        if ch_width > max_width {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }
            lines.push(ch.to_string());
            continue;
        }
        if current_width + ch_width > max_width {
            lines.push(std::mem::take(&mut current));
            current_width = 0;
        }
        current.push(ch);
        current_width += ch_width;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn no_lyrics_label(app: &AppState) -> &'static str {
    match app.language {
        crate::data::config::Language::Zh => "暂无歌词",
        crate::data::config::Language::En => "No lyrics",
    }
}

fn current_lyric_index(lines: &[LyricLine], pos_ms: u64) -> usize {
    let mut idx = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.start_ms <= pos_ms {
            idx = i;
        } else {
            break;
        }
    }
    idx
}
