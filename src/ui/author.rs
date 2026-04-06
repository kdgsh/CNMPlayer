use crate::app::App;
use crate::data::config::Language;
use crate::ui::page_lyrics;
use crate::ui::player_bar;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub fn draw_author(frame: &mut Frame, app: &mut App) {
    app.clear_player_bar_hits();
    app.clear_content_hits();

    let size = frame.size();
    frame.render_widget(Block::default().style(base_bg_style(app)), size);

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
        .constraints([
            Constraint::Min(1),
            Constraint::Length(player_bar::PLAYER_BAR_HEIGHT),
        ])
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

    draw_author_header(frame, app, main[0]);
    draw_author_tiles(frame, app, main[1]);

    if app.config.page_lyrics {
        let panel_area = page_lyrics::overlay_panel_area(content_area);
        page_lyrics::draw_page_lyrics_panel(frame, app, panel_area);
    }
    if app.config.show_hints {
        draw_author_hint(frame, app, hint_area);
    }

    player_bar::draw_collapsed_player_bar(frame, app, rows[1]);
}

fn draw_author_header(frame: &mut Frame, app: &mut App, area: Rect) {
    let inner = area.inner(&ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    if inner.width < 6 || inner.height < 3 {
        return;
    }

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((inner.height * 2).min(26)),
            Constraint::Min(1),
        ])
        .split(inner);

    let cover_block = centered_visual_square_block(cols[0]);
    if cover_block.width > 0 && cover_block.height > 0 {
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.color_surface())),
            cover_block,
        );

        let cover_area = cover_block.inner(&ratatui::layout::Margin {
            horizontal: 1,
            vertical: 1,
        });
        if cover_area.width > 0 && cover_area.height > 0 {
            frame.render_widget(Block::default().style(surface_bg_style(app)), cover_area);
            let cover_ascii = app.author.cover_ascii(cover_area.width, cover_area.height);
            frame.render_widget(
                Paragraph::new(cover_ascii)
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(app.theme.color_text())),
                cover_area,
            );
        }
    }

    let hot_count = app.author.hot_songs.len();
    let album_count = app.author.albums.len();
    let ep_count = app.author.eps.len();
    let single_count = app.author.singles.len();

    let info = vec![
        Line::from(Span::styled(
            app.author.title.clone(),
            Style::default()
                .fg(app.theme.color_text())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            app.author.artist.clone(),
            Style::default().fg(app.theme.color_subtext()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.author.description.clone(),
            Style::default().fg(app.theme.color_text()),
        )),
        Line::from(Span::styled(
            format!(
                "{} {}  |  {} {}  |  EP {}  |  Single {}",
                match app.config.language {
                    Language::Zh => "热门歌曲",
                    Language::En => "Hot Songs",
                },
                hot_count,
                match app.config.language {
                    Language::Zh => "专辑",
                    Language::En => "Albums",
                },
                album_count,
                ep_count,
                single_count
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

fn draw_author_tiles(frame: &mut Frame, app: &mut App, area: Rect) {
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
    app.author.set_columns(columns);

    let visible_rows = usize::from((inner.height / row_step).max(1));
    app.author.set_visible_rows(visible_rows);
    let row_offset = app.author.effective_scroll_row_offset();

    for index in 0..app.author.tiles.len() {
        let row = index / columns;
        if row < row_offset {
            continue;
        }
        let visual_row = row - row_offset;
        if visual_row >= visible_rows {
            break;
        }
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

        app.push_author_tile_hit(
            crate::app::HitRect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            },
            index,
        );

        let focused = index == app.author.focused_idx;
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
                let tile = &mut app.author.tiles[index];
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
            let tile = &app.author.tiles[index];
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

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center),
            text_rect,
        );
    }
}

fn centered_visual_square_block(area: Rect) -> Rect {
    if area.width < 4 || area.height < 3 {
        return Rect::default();
    }

    let content_width = area.width.saturating_sub(2);
    let content_height = area.height.saturating_sub(2);
    let side = content_height.min(content_width / 2);
    if side == 0 {
        return Rect::default();
    }

    let width = side.saturating_mul(2).saturating_add(2);
    let height = side.saturating_add(2);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
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

fn draw_author_hint(frame: &mut Frame, app: &App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let text = match app.config.language {
        Language::Zh => format!(
            "{} 搜索  Enter 进入歌单页  Esc 返回搜索  {} 全屏",
            app.config.keybind_search_box, app.config.keybind_fullscreen
        ),
        Language::En => format!(
            "{} Search  Enter open playlist page  Esc back  {} Fullscreen",
            app.config.keybind_search_box, app.config.keybind_fullscreen
        ),
    };

    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(app.theme.color_subtext()))
            .alignment(Alignment::Left),
        area,
    );
}
