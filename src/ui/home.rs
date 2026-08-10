use crate::app::{App, TopBarTab};
use crate::data::config::Language;
use crate::ui::page_lyrics;
use crate::ui::player_bar;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

const TOPBAR_HEIGHT: u16 = 1;

pub fn draw_home(frame: &mut Frame, app: &mut App) {
    app.clear_player_bar_hits();
    app.clear_content_hits();

    let size = frame.area();
    frame.render_widget(Block::default().style(base_bg_style(app)), size);

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

    let layout = if app.topbar.active_tab == TopBarTab::Recommend {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(TOPBAR_HEIGHT),
                Constraint::Length(TOPBAR_HEIGHT),
                Constraint::Min(1),
            ])
            .split(content_area);
        (split[0], Some(split[1]), split[2])
    } else {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(TOPBAR_HEIGHT), Constraint::Min(1)])
            .split(content_area);
        (split[0], None, split[1])
    };

    draw_home_topbar(frame, app, layout.0);
    if let Some(categories_area) = layout.1 {
        draw_home_topbar_categories(frame, app, categories_area);
    }
    draw_home_tiles(frame, app, layout.2);

    if app.config.page_lyrics {
        let panel_area = page_lyrics::overlay_panel_area(content_area);
        page_lyrics::draw_page_lyrics_panel(frame, app, panel_area);
    }
    if app.config.show_hints {
        draw_home_hint(frame, app, hint_area);
    }

    player_bar::draw_collapsed_player_bar(frame, app, rows[1]);
}

fn draw_home_topbar(frame: &mut Frame, app: &mut App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let zh = app.config.language == Language::Zh;
    let mut spans: Vec<Span> = Vec::new();
    let mut cursor = area.x;

    for tab in TopBarTab::all() {
        let active = tab == app.topbar.active_tab;
        let text = format!(" {} ", tab.label(zh));
        let width = display_width(&text) as u16;
        app.push_topbar_tab_hit(
            crate::app::HitRect {
                x: cursor,
                y: area.y,
                width: width.max(1),
                height: 1,
            },
            tab,
        );

        let style = if active && app.topbar.tab_focus {
            Style::default()
                .fg(app.theme.color_base())
                .bg(app.theme.color_accent())
                .add_modifier(Modifier::BOLD)
        } else if active {
            Style::default()
                .fg(app.theme.color_accent())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.color_subtext())
        };
        spans.push(Span::styled(text, style));
        spans.push(Span::styled("  ", Style::default()));

        cursor = cursor.saturating_add(width).saturating_add(2);
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_home_topbar_categories(frame: &mut Frame, app: &mut App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let total = app.topbar.recommend_categories_len();
    if total == 0 {
        return;
    }

    let focus = app.topbar.recommend_cat_index.min(total - 1);
    let mut order: Vec<usize> = vec![focus];
    let mut used = 2 + display_width(app.topbar.recommend_category_name(focus));
    let mut left = focus;
    let mut right = focus;
    let mut can_left = true;
    let mut can_right = true;

    while (can_left || can_right) && used < usize::from(area.width) {
        if can_left && left > 0 {
            left -= 1;
            used = used.saturating_add(2 + display_width(app.topbar.recommend_category_name(left)));
            order.insert(0, left);
        } else {
            can_left = false;
        }
        if used >= usize::from(area.width) {
            break;
        }
        if can_right && right + 1 < total {
            right += 1;
            used = used.saturating_add(2 + display_width(app.topbar.recommend_category_name(right)));
            order.push(right);
        } else {
            can_right = false;
        }
    }

    let mut spans: Vec<Span> = Vec::new();
    let mut cursor = area.x;
    for idx in order {
        let text = format!(" {} ", app.topbar.recommend_category_name(idx));
        let width = display_width(&text) as u16;
        if cursor.saturating_add(width) > area.x.saturating_add(area.width) {
            break;
        }
        app.push_topbar_cat_hit(
            crate::app::HitRect {
                x: cursor,
                y: area.y,
                width: width.max(1),
                height: 1,
            },
            idx,
        );

        let active = idx == focus;
        let style = if active && app.topbar.recommend_cat_focus {
            Style::default()
                .fg(app.theme.color_base())
                .bg(app.theme.color_accent())
                .add_modifier(Modifier::BOLD)
        } else if active {
            Style::default()
                .fg(app.theme.color_accent())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.color_subtext())
        };
        spans.push(Span::styled(text, style));
        spans.push(Span::styled("  ", Style::default()));
        cursor = cursor.saturating_add(width).saturating_add(2);
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_home_tiles(frame: &mut Frame, app: &mut App, area: Rect) {
    let zh = app.config.language == Language::Zh;
    let total = app.topbar.tab_len();

    if total == 0 {
        let text = if app.topbar.loading {
            if zh { "正在加载..." } else { "Loading..." }
        } else {
            if zh { "暂无内容，按 Tab 切换分类" } else { "Nothing here yet" }
        };
        frame.render_widget(
            Paragraph::new(text).style(Style::default().fg(app.theme.color_subtext())),
            area,
        );
        return;
    }

    let margin = ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    };
    let inner = area.inner(margin);
    if inner.width < 14 || inner.height < 8 {
        return;
    }

    let tile_h = 12_u16.min(inner.height.saturating_sub(1)).max(6);
    let tile_w = tile_h.saturating_mul(2).saturating_add(4);
    let col_step = tile_w.saturating_add(2);
    let row_step = tile_h.saturating_add(1);
    let columns = usize::from((inner.width / col_step).max(1));
    let visible_rows = usize::from((inner.height / row_step).max(1));
    app.topbar.set_columns(columns);
    app.topbar.set_visible_rows(visible_rows);
    let row_offset = app.topbar.effective_scroll_row_offset();
    let draw_ascii = app.draw_ascii();

    for index in 0..total {
        let row = index / columns;
        if row < row_offset {
            continue;
        }
        let visual_row = row - row_offset;
        if visual_row >= visible_rows {
            continue;
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

        app.push_topbar_entry_hit(
            crate::app::HitRect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            },
            index,
        );

        let focused = !app.topbar.tab_focus
            && !app.topbar.recommend_cat_focus
            && index == app.topbar.tab_focused();
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

        let inner_rect = rect.inner(ratatui::layout::Margin {
            horizontal: 1,
            vertical: 1,
        });
        if inner_rect.width < 2 || inner_rect.height < 2 {
            continue;
        }

        let text_rows = 1;
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

        if !cover_rect.is_empty() {
            let text_style = if focused {
                Style::default().fg(app.theme.color_accent2())
            } else {
                Style::default().fg(app.theme.color_text())
            };
            if let Some(cover) = app.topbar.cover_at_mut(index) {
                cover.render(
                    frame,
                    &mut app.graphics_picker,
                    cover_rect,
                    text_style,
                    None,
                    draw_ascii,
                );
            }
        }

        let title = app
            .topbar
            .item_at(index)
            .map(|item| item.title.clone())
            .unwrap_or_default();

        let title_style = if focused {
            Style::default()
                .fg(app.theme.color_accent2())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.color_text())
        };

        let lines = vec![Line::from(Span::styled(title, title_style))];

        let content = Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);

        frame.render_widget(content, text_rect);
    }

    let total_rows = total.saturating_sub(1) / columns + 1;
    if total_rows > visible_rows {
        let page = (row_offset + 1).min(total_rows);
        let text = format!("{}/{}", page, total_rows);
        let width = display_width(&text) as u16;
        let indicator = Rect {
            x: inner.x + inner.width - width - 1,
            y: inner.y + inner.height - 1,
            width: width + 1,
            height: 1,
        };
        if indicator.y >= inner.y {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    text,
                    Style::default().fg(app.theme.color_subtext()),
                )),
                indicator,
            );
        }
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

fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

fn base_bg_style(app: &App) -> Style {
    if app.config.transparent_background {
        Style::default()
    } else {
        Style::default().bg(app.theme.color_base())
    }
}
