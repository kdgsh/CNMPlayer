use crate::app::{App, Overlay};
use crate::data::config::{AudioQuality, BarChannels, BarNumber, Language, VisualizeMode};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn draw_settings_modal(frame: &mut Frame, app: &App) {
    let size = frame.size();
    let area = centered_rect(70, 20, size);

    frame.render_widget(Clear, area);

    let title = match app.overlay {
        Some(Overlay::Settings) => l(app, " 设置 ", " Settings "),
        Some(Overlay::SettingsPlayback) => l(app, " 播放页设置 ", " Playback Settings "),
        Some(Overlay::SettingsKeybinds) => l(app, " 按键绑定 ", " Keybinds "),
        Some(Overlay::SettingsAbout) => " about ",
        _ => l(app, " 设置 ", " Settings "),
    };

    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(app.theme.color_accent()))
            .style(base_bg_style(app)),
        area,
    );

    let inner = area.inner(&ratatui::layout::Margin {
        horizontal: 2,
        vertical: 1,
    });

    match app.overlay {
        Some(Overlay::SettingsPlayback) => draw_playback_settings(frame, app, inner),
        Some(Overlay::SettingsKeybinds) => draw_keybind_settings(frame, app, inner),
        Some(Overlay::SettingsAbout) => draw_about(frame, app, inner),
        _ => draw_root_settings(frame, app, inner),
    }
}

fn draw_root_settings(frame: &mut Frame, app: &App, inner: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    frame.render_widget(Paragraph::new(""), rows[0]);

    let items = vec![
        format!("{}: {}", l(app, "主题", "Theme"), app.config.theme),
        format!(
            "{}: {}",
            l(app, "背景透明", "Transparent Background"),
            on_off(app, app.config.transparent_background)
        ),
        format!(
            "{}: {}",
            l(app, "语言", "Language"),
            match app.config.language {
                Language::Zh => l(app, "中文", "Chinese"),
                Language::En => "English",
            }
        ),
        format!("{}...", l(app, "播放页设置", "Playback Settings")),
        format!("{}...", l(app, "按键绑定", "Keybinds")),
        format!(
            "{}: {}",
            l(app, "显示提示", "Show Hints"),
            on_off(app, app.config.show_hints)
        ),
        l(app, "退出登录", "Logout").to_string(),
        "about".to_string(),
    ];

    let lines: Vec<Line> = items
        .iter()
        .enumerate()
        .map(|(idx, text)| {
            let style = if idx == app.settings_selected {
                Style::default()
                    .fg(app.theme.color_base())
                    .bg(app.theme.color_accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.color_text())
            };
            Line::from(Span::styled(format!("  {}", text), style))
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), rows[1]);

    frame.render_widget(Paragraph::new(""), rows[2]);
}

fn draw_playback_settings(frame: &mut Frame, app: &App, inner: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    frame.render_widget(Paragraph::new(""), rows[0]);

    let bar_number = match app.config.bar_number {
        BarNumber::Auto => l(app, "自动", "Auto"),
        BarNumber::N16 => "16",
        BarNumber::N32 => "32",
        BarNumber::N48 => "48",
        BarNumber::N64 => "64",
        BarNumber::N80 => "80",
        BarNumber::N96 => "96",
    };

    let channels = match app.config.bar_channels {
        BarChannels::Mono => "Mono",
        BarChannels::Stereo => "Stereo",
    };

    let items = vec![
        format!(
            "{}: {}",
            l(app, "可视化", "Visualization"),
            match app.config.visualize {
                VisualizeMode::Bars => l(app, "频谱", "Bars"),
                VisualizeMode::Oscilloscope => l(app, "示波器", "Oscilloscope"),
            }
        ),
        format!(
            "{}: {}",
            l(app, "超级流畅", "Super Smooth"),
            on_off(app, app.config.super_smooth_bar)
        ),
        format!("{}: {}", l(app, "频谱间隔", "Bars Gap"), on_off(app, app.config.bars_gap)),
        format!("{}: {}", l(app, "频谱数", "Bars Count"), bar_number),
        format!("{}: {}", l(app, "声道", "Channels"), channels),
        format!(
            "{}: {}",
            l(app, "Kitty 图形", "Kitty Graphics"),
            on_off(app, app.config.kitty_graphics)
        ),
        format!(
            "{}: {}",
            l(app, "页面歌词", "Page Lyrics"),
            on_off(app, app.config.page_lyrics)
        ),
        format!(
            "{}: {}",
            l(app, "音质", "Audio Quality"),
            audio_quality_label(app, app.config.audio_quality)
        ),
    ];

    let lines: Vec<Line> = items
        .iter()
        .enumerate()
        .map(|(idx, text)| {
            let style = if idx == app.settings_playback_selected {
                Style::default()
                    .fg(app.theme.color_base())
                    .bg(app.theme.color_accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.color_text())
            };
            Line::from(Span::styled(format!("  {}", text), style))
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), rows[1]);

    frame.render_widget(Paragraph::new(""), rows[2]);
}

fn draw_keybind_settings(frame: &mut Frame, app: &App, inner: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(2)])
        .split(inner);
    frame.render_widget(Paragraph::new(""), rows[0]);

    let lines: Vec<Line> = (0..9)
        .map(|idx| {
            let is_rebinding = app.settings_keybind_rebinding == Some(idx);
            let style = if is_rebinding {
                Style::default()
                    .fg(app.theme.color_base())
                    .bg(app.theme.color_accent2())
                    .add_modifier(Modifier::BOLD)
            } else if idx == app.settings_keybind_selected {
                Style::default()
                    .fg(app.theme.color_base())
                    .bg(app.theme.color_accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.color_text())
            };

            let mut label = app.keybind_label_for_index(idx);
            if is_rebinding {
                label.push_str(l(app, "  [等待输入]", "  [Waiting Input]"));
            }

            Line::from(Span::styled(format!("  {}", label), style))
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), rows[1]);

    let hint = if let Some(index) = app.settings_keybind_rebinding {
        format!(
            "{}: {}  {}",
            l(app, "正在重绑", "Rebinding"),
            app.keybind_label_for_index(index)
                ,
            l(app, "按下新快捷键，Esc 取消", "Press a new shortcut, Esc to cancel")
        )
    } else {
        l(
            app,
            "Enter 重绑  Ctrl+Alt+R 重置  Esc 返回",
            "Enter rebind  Ctrl+Alt+R reset  Esc back",
        )
        .to_string()
    };

    frame.render_widget(
        Paragraph::new(hint)
            .style(Style::default().fg(app.theme.color_subtext()))
            .wrap(ratatui::widgets::Wrap { trim: true }),
        rows[2],
    );
}

fn draw_about(frame: &mut Frame, app: &App, inner: Rect) {
    let text = vec![
        Line::from(Span::styled(
            "CNMPlayer",
            Style::default()
                .fg(app.theme.color_accent())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            l(
                app,
                "全屏播放页基于 TMPlayer 原始代码整合。",
                "Fullscreen playback is integrated from original TMPlayer code.",
            ),
            Style::default().fg(app.theme.color_text()),
        )),
    ];

    frame.render_widget(Paragraph::new(text).alignment(Alignment::Left), inner);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width.saturating_sub(2)).max(12);
    let h = height.min(area.height.saturating_sub(2)).max(5);
    Rect {
        x: area.x + area.width.saturating_sub(w) / 2,
        y: area.y + area.height.saturating_sub(h) / 2,
        width: w,
        height: h,
    }
}

fn base_bg_style(app: &App) -> Style {
    Style::default().bg(app.theme.color_base())
}

fn audio_quality_label(app: &App, quality: AudioQuality) -> &'static str {
    match app.config.language {
        Language::Zh => match quality {
            AudioQuality::Standard => "标准",
            AudioQuality::Higher => "较高",
            AudioQuality::Exhigh => "极高",
            AudioQuality::Lossless => "无损",
            AudioQuality::Hires => "Hi-Res",
            AudioQuality::Jyeffect => "高清环绕声",
            AudioQuality::Sky => "沉浸环绕声",
            AudioQuality::Dolby => "杜比全景声",
            AudioQuality::Jymaster => "超清母带",
        },
        Language::En => match quality {
            AudioQuality::Standard => "Standard",
            AudioQuality::Higher => "Higher",
            AudioQuality::Exhigh => "Exhigh",
            AudioQuality::Lossless => "Lossless",
            AudioQuality::Hires => "Hi-Res",
            AudioQuality::Jyeffect => "JYEffect",
            AudioQuality::Sky => "Sky",
            AudioQuality::Dolby => "Dolby",
            AudioQuality::Jymaster => "JYMaster",
        },
    }
}

fn l<'a>(app: &App, zh: &'a str, en: &'a str) -> &'a str {
    match app.config.language {
        Language::Zh => zh,
        Language::En => en,
    }
}

fn on_off(app: &App, enabled: bool) -> &'static str {
    match app.config.language {
        Language::Zh => {
            if enabled {
                "开"
            } else {
                "关"
            }
        }
        Language::En => {
            if enabled {
                "On"
            } else {
                "Off"
            }
        }
    }
}
