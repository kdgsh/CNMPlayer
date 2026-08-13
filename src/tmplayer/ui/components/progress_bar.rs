use crate::tmplayer::app::state::AppState;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::time::Duration;

pub fn render(f: &mut Frame, area: Rect, app: &AppState, pos: Duration, dur: Duration) {
    let w = area.width as usize;
    if w == 0 {
        return;
    }
    if w <= 2 {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "─".repeat(w),
                Style::default().fg(app.theme.color_subtext()),
            ))),
            area,
        );
        return;
    }

    let inner = w.saturating_sub(2);
    let ratio = if dur.as_secs_f32() > 0.0 {
        (pos.as_secs_f32() / dur.as_secs_f32()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let filled = (ratio * inner as f32).round() as usize;

    let filled_s = "█".repeat(filled.min(inner));
    let empty_s = "░".repeat(inner.saturating_sub(filled.min(inner)));

    let line = Line::from(vec![
        Span::styled("[", Style::default().fg(app.theme.color_subtext())),
        Span::styled(filled_s, Style::default().fg(app.theme.color_accent())),
        Span::styled(empty_s, Style::default().fg(app.theme.color_subtext())),
        Span::styled("]", Style::default().fg(app.theme.color_subtext())),
    ]);

    f.render_widget(Paragraph::new(line), area);
}
