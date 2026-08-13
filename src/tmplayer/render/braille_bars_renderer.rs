use crate::tmplayer::app::state::AppState;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub fn render(f: &mut Frame, area: Rect, app: &AppState) {
    let w_cells = area.width as usize;
    let h_cells = area.height as usize;
    if w_cells == 0 || h_cells == 0 {
        return;
    }

    let w_px = w_cells * 2;
    let h_px = h_cells * 4;

    let mut bits: Vec<u8> = vec![0u8; w_cells * h_cells];
    let bars = &app.spectrum.bars;
    if !bars.is_empty() {
        for px in 0..w_px {
            let val = sample_bar(bars, w_px, px);
            let height = ((val).sqrt() * h_px as f32).round() as usize;
            let start = h_px.saturating_sub(height);
            for y in start..h_px {
                set_pixel(&mut bits, w_cells, h_cells, px as i32, y as i32);
            }
        }
    }

    let mut lines: Vec<Line> = Vec::with_capacity(h_cells);
    for row in 0..h_cells {
        let t = if h_cells <= 1 {
            1.0
        } else {
            row as f32 / (h_cells - 1) as f32
        };
        let fg = vertical_gradient_color(app, t);
        let mut s = String::with_capacity(w_cells);
        let base = row * w_cells;
        for col in 0..w_cells {
            s.push(braille_from_bits(bits[base + col]));
        }
        lines.push(Line::from(Span::styled(s, Style::default().fg(fg))));
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn sample_bar(bars: &[f32], draw_len: usize, i: usize) -> f32 {
    let data_len = bars.len().max(1);
    let idx =
        ((i as u32) * (data_len as u32) / (draw_len as u32)).min((data_len - 1) as u32) as usize;
    bars.get(idx).copied().unwrap_or(0.0).clamp(0.0, 1.0)
}

fn set_pixel(bits: &mut [u8], w_cells: usize, h_cells: usize, x: i32, y: i32) {
    if x < 0 || y < 0 {
        return;
    }
    let w_px = (w_cells * 2) as i32;
    let h_px = (h_cells * 4) as i32;
    if x >= w_px || y >= h_px {
        return;
    }

    let cell_x = (x / 2) as usize;
    let cell_y = (y / 4) as usize;
    if cell_x >= w_cells || cell_y >= h_cells {
        return;
    }

    let dx = (x % 2) as usize;
    let dy = (y % 4) as usize;
    let bit = braille_bit(dx, dy);
    let idx = cell_y * w_cells + cell_x;
    bits[idx] |= bit;
}

fn braille_bit(dx: usize, dy: usize) -> u8 {
    // Braille dot mapping (dx: 0 left, 1 right; dy: 0..3 top..bottom)
    // (0,0)->1, (0,1)->2, (0,2)->3, (0,3)->7
    // (1,0)->4, (1,1)->5, (1,2)->6, (1,3)->8
    match (dx, dy) {
        (0, 0) => 0x01,
        (0, 1) => 0x02,
        (0, 2) => 0x04,
        (0, 3) => 0x40,
        (1, 0) => 0x08,
        (1, 1) => 0x10,
        (1, 2) => 0x20,
        (1, 3) => 0x80,
        _ => 0,
    }
}

fn braille_from_bits(bits: u8) -> char {
    // Unicode braille patterns start at 0x2800.
    char::from_u32(0x2800 + bits as u32).unwrap_or(' ')
}

fn vertical_gradient_color(app: &AppState, t: f32) -> Color {
    let top = app.theme.color_accent2();
    let bottom = app.theme.color_accent3();
    mix(top, bottom, t)
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            let r = (ar as f32 + (br as f32 - ar as f32) * t) as u8;
            let g = (ag as f32 + (bg as f32 - ag as f32) * t) as u8;
            let b = (ab as f32 + (bb as f32 - ab as f32) * t) as u8;
            Color::Rgb(r, g, b)
        }
        _ => a,
    }
}
