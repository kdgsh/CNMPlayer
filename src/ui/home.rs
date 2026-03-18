use crate::app::App;
use crate::data::config::Language;
use crate::ui::page_lyrics;
use crate::ui::player_bar;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub fn draw_home(frame: &mut Frame, app: &mut App) {
    app.clear_player_bar_hits();
    app.clear_content_hits();

    let size = frame.size();
    frame.render_widget(
        Block::default().style(base_bg_style(app)),
        size,
    );

    if size.width < 32 || size.height < 12 {
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

    draw_tiles(frame, app, content_area);
    if app.config.page_lyrics {
        let panel_area = page_lyrics::overlay_panel_area(content_area);
        page_lyrics::draw_page_lyrics_panel(frame, app, panel_area);
    }
    if app.config.show_hints {
        draw_home_hint(frame, app, hint_area);
    }

    player_bar::draw_collapsed_player_bar(frame, app, rows[1]);
}

fn draw_tiles(frame: &mut Frame, app: &mut App, area: Rect) {
    let margin = ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    };
    let inner = area.inner(&margin);
    if inner.width < 14 || inner.height < 8 {
        return;
    }

    let tile_h = 12_u16.min(inner.height.saturating_sub(1)).max(6);
    let tile_w = tile_h.saturating_mul(2).saturating_add(4);
    let col_step = tile_w.saturating_add(2);
    let row_step = tile_h.saturating_add(1);
    let columns = usize::from((inner.width / col_step).max(1));
    app.home.set_columns(columns);

    let visible_rows = usize::from((inner.height / row_step).max(1));
    let focused_row = app.home.focused_idx / columns;
    let row_offset = focused_row.saturating_sub(visible_rows.saturating_sub(1));

    for index in 0..app.home.tiles.len() {
        let row = index / columns;
        if row < row_offset {
            continue;
        }
        let visual_row = row - row_offset;
        let col = index % columns;
        let x = inner.x + (col as u16) * col_step;
        let y = inner.y + (visual_row as u16) * row_step;
        if x >= inner.x + inner.width || y >= inner.y + inner.height {
            continue;
        }

        let rect = Rect {
            x,
            y,
            width: tile_w.min(inner.x + inner.width - x),
            height: tile_h.min(inner.y + inner.height - y),
        };

        app.push_home_tile_hit(
            crate::app::HitRect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            },
            index,
        );

        let focused = index == app.home.focused_idx;
        let tile_bg = if focused {
            app.theme.color_surface()
        } else {
            app.theme.color_base()
        };
        let tile_style = if app.config.transparent_background {
            Style::default()
        } else {
            Style::default().bg(tile_bg)
        };
        let border_style = if focused {
            Style::default()
                .fg(app.theme.color_accent())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.color_surface())
        };

        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(tile_style),
            rect,
        );

        let inner_rect = rect.inner(&ratatui::layout::Margin {
            horizontal: 1,
            vertical: 1,
        });
        if inner_rect.width < 2 || inner_rect.height < 2 {
            continue;
        }

        let text_rows = if inner_rect.height >= 4 { 2 } else { 1 };
        let cover_height = inner_rect.height.saturating_sub(text_rows);
        let cover_rect = Rect {
            x: inner_rect.x,
            y: inner_rect.y,
            width: inner_rect.width,
            height: cover_height,
        };
        let text_rect = Rect {
            x: inner_rect.x,
            y: inner_rect.y + cover_height,
            width: inner_rect.width,
            height: text_rows,
        };

        if cover_rect.width > 0 && cover_rect.height > 0 {
            let ascii = {
                let tile = &mut app.home.tiles[index];
                tile.cover_ascii(cover_rect.width, cover_rect.height)
            };
            let cover_style = if focused {
                Style::default().fg(app.theme.color_accent2())
            } else {
                Style::default().fg(app.theme.color_text())
            };
            frame.render_widget(Paragraph::new(ascii).style(cover_style), cover_rect);
        }

        let (title, subtitle) = {
            let tile = &app.home.tiles[index];
            (tile.title.clone(), tile.subtitle.clone())
        };

        let title_style = if focused {
            Style::default()
                .fg(app.theme.color_accent2())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.color_text())
        };
        let subtitle_style = Style::default().fg(app.theme.color_subtext());

        let mut lines = vec![Line::from(Span::styled(title, title_style))];
        if text_rows > 1 {
            lines.push(Line::from(Span::styled(subtitle, subtitle_style)));
        }

        let content = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);

        frame.render_widget(content, text_rect);
    }

}

fn draw_home_hint(frame: &mut Frame, app: &App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let text = match app.config.language {
        Language::Zh => format!(
            "{} 搜索  {} 设置  {} 全屏  {} 退出",
            app.config.keybind_search_box,
            app.config.keybind_settings,
            app.config.keybind_fullscreen,
            app.config.keybind_quit
        ),
        Language::En => format!(
            "{} Search  {} Settings  {} Fullscreen  {} Quit",
            app.config.keybind_search_box,
            app.config.keybind_settings,
            app.config.keybind_fullscreen,
            app.config.keybind_quit
        ),
    };

    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(app.theme.color_subtext()))
            .alignment(Alignment::Left),
        area,
    );
}

fn base_bg_style(app: &App) -> Style {
    if app.config.transparent_background {
        Style::default()
    } else {
        Style::default().bg(app.theme.color_base())
    }
}
