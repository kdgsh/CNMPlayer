use crate::app::{App, SuggestItem};
use crate::data::config::Language;
use crate::tmplayer::ui::borders::SOLID_BORDER;
use ratatui::Frame;
use ratatui::layout::{Alignment, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub const TARGET_HEIGHT: u16 = 3;
pub const MAX_SUGGEST_ROWS: u16 = 8;

pub fn search_box_target_height(app: &App) -> u16 {
    TARGET_HEIGHT + app.search_suggest.len().min(MAX_SUGGEST_ROWS as usize) as u16
}

pub fn draw_search_box_overlay(frame: &mut Frame, app: &App) {
    let size = frame.area();
    if size.width < 20 || size.height < 2 {
        return;
    }

    let visible_h = app
        .search_box_anim_height
        .min(search_box_target_height(app))
        .min(size.height);
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

    let title = match app.config.language {
        Language::Zh => " 搜索 ",
        Language::En => " Search ",
    };

    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_set(SOLID_BORDER)
            .border_style(Style::default().fg(app.theme.color_subtext()))
            .title(title)
            .title_style(Style::default().fg(app.theme.color_subtext()))
            .style(base_bg_style(app)),
        area,
    );

    if visible_h < TARGET_HEIGHT {
        return;
    }

    let inner = area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let input = app.search_box_input.clone();
    let input_style = if input.trim().is_empty() {
        Style::default()
            .fg(app.theme.color_subtext())
            .bg(app.theme.color_surface())
    } else {
        Style::default()
            .fg(app.theme.color_text())
            .bg(app.theme.color_surface())
    };
    let prompt = Span::styled(
        "❯ ",
        Style::default()
            .fg(app.theme.color_accent())
            .bg(app.theme.color_surface()),
    );
    let content = if input.trim().is_empty() {
        match app.config.language {
            Language::Zh => "请输入搜索内容（后缀 @single/@album/@list，或仅输入 @author）",
            Language::En => "Type to search (suffix @single/@album/@list, or only @author)",
        }
    } else {
        input.as_str()
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            prompt,
            Span::styled(content, input_style),
        ]))
        .style(input_style)
        .alignment(Alignment::Left),
        inner,
    );

    let suggest_rows = inner.height.saturating_sub(1) as usize;
    for (i, suggest) in app
        .search_suggest
        .iter()
        .take(suggest_rows)
        .enumerate()
    {
        let row = Rect {
            x: inner.x,
            y: inner.y + 1 + i as u16,
            width: inner.width,
            height: 1,
        };
        let focused = Some(i) == app.search_suggest_focus;
        render_suggest_row(frame, app, row, suggest, focused);
    }

    // Use terminal-native block cursor without injecting extra glyphs into the text.
    let mut cursor_offset = 2u16;
    for (idx, ch) in input.chars().enumerate() {
        if idx >= app.search_box_cursor {
            break;
        }
        cursor_offset = cursor_offset.saturating_add(ch.width().unwrap_or(1).max(1) as u16);
    }
    let cursor_x = inner
        .x
        .saturating_add(cursor_offset.min(inner.width.saturating_sub(1)));
    frame.set_cursor_position((cursor_x, inner.y));
}

fn render_suggest_row(
    frame: &mut Frame,
    app: &App,
    row: Rect,
    suggest: &SuggestItem,
    focused: bool,
) {
    let row_style = if focused {
        Style::default()
            .fg(app.theme.color_base())
            .bg(app.theme.color_accent())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(app.theme.color_text())
            .bg(app.theme.color_surface())
    };

    let tag = suggest.item.type_tag.as_deref().unwrap_or_default();
    let body = if suggest.item.right_label.is_empty() {
        suggest.item.left_label.clone()
    } else {
        format!("{} - {}", suggest.item.left_label, suggest.item.right_label)
    };

    let tag_str = format!("[{}] ", tag);
    let tag_used = UnicodeWidthStr::width(tag_str.as_str());
    let body_max = usize::from(row.width).saturating_sub(tag_used);

    let mut spans = Vec::new();
    if !tag.is_empty() {
        let tag_style = if focused {
            row_style
        } else {
            Style::default()
                .fg(app.theme.color_accent())
                .bg(app.theme.color_surface())
        };
        spans.push(Span::styled(format!("[{}] ", tag), tag_style));
    }
    spans.push(Span::styled(
        clip_to_display_width(&body, body_max),
        row_style,
    ));

    frame.render_widget(Paragraph::new(Line::from(spans)), row);
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

fn base_bg_style(app: &App) -> Style {
    Style::default()
        .fg(app.theme.color_subtext())
        .bg(app.theme.color_surface())
}
