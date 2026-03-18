use crate::app::App;
use crate::data::config::Language;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthChar;

pub const TARGET_HEIGHT: u16 = 3;

pub fn draw_search_box_overlay(frame: &mut Frame, app: &App) {
    let size = frame.size();
    if size.width < 20 || size.height < 2 {
        return;
    }

    let visible_h = app.search_box_anim_height.min(TARGET_HEIGHT).min(size.height);
    if visible_h == 0 {
        return;
    }

    let width = (size.width / 2).max(24).min(size.width.saturating_sub(2));
    let area = Rect {
        x: size.x + size.width.saturating_sub(width) / 2,
        y: size.y,
        width,
        height: visible_h,
    };

    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.color_accent()))
            .style(base_bg_style(app)),
        area,
    );

    if visible_h < TARGET_HEIGHT {
        return;
    }

    let inner = area.inner(&ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let input = app.search_box_input.clone();
    let content = if input.trim().is_empty() {
        match app.config.language {
            Language::Zh => "请输入搜索内容（@single/@album/@list/@author）".to_string(),
            Language::En => "Type to search (@single/@album/@list/@author)".to_string(),
        }
    } else {
        input.clone()
    };

    let style = if input.trim().is_empty() {
        Style::default().fg(app.theme.color_subtext())
    } else {
        Style::default().fg(app.theme.color_text())
    };

    frame.render_widget(
        Paragraph::new(content).style(style).alignment(Alignment::Left),
        inner,
    );

    // Use terminal-native block cursor without injecting extra glyphs into the text.
    let mut cursor_offset = 0u16;
    for (idx, ch) in input.chars().enumerate() {
        if idx >= app.search_box_cursor {
            break;
        }
        cursor_offset = cursor_offset.saturating_add(ch.width().unwrap_or(1).max(1) as u16);
    }
    let cursor_x = inner
        .x
        .saturating_add(cursor_offset.min(inner.width.saturating_sub(1)));
    frame.set_cursor(cursor_x, inner.y);
}

fn base_bg_style(app: &App) -> Style {
    Style::default().bg(app.theme.color_base())
}
