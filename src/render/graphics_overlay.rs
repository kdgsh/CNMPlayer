use ratatui::layout::Rect;

const CELL_W_PX: u32 = 8;
const CELL_H_PX: u32 = 16;

pub fn map_segment_to_cover_crop(
    base: Rect,
    segment: Rect,
    image_w: u32,
    image_h: u32,
) -> Option<(u32, u32, u32, u32)> {
    if base.width == 0 || base.height == 0 || segment.width == 0 || segment.height == 0 {
        return None;
    }
    if image_w == 0 || image_h == 0 {
        return None;
    }

    let (view_x, view_y, view_w, view_h) =
        cover_viewport(image_w, image_h, base.width, base.height);

    let base_w = base.width as u64;
    let base_h = base.height as u64;

    let rel_x0 = segment.x.saturating_sub(base.x) as u64;
    let rel_y0 = segment.y.saturating_sub(base.y) as u64;
    let rel_x1 = rel_x0.saturating_add(segment.width as u64);
    let rel_y1 = rel_y0.saturating_add(segment.height as u64);

    let view_w_u64 = view_w as u64;
    let view_h_u64 = view_h as u64;

    let src_x = view_x.saturating_add((rel_x0.saturating_mul(view_w_u64) / base_w) as u32);
    let src_y = view_y.saturating_add((rel_y0.saturating_mul(view_h_u64) / base_h) as u32);

    let src_x_end = view_x.saturating_add(
        ((rel_x1.saturating_mul(view_w_u64) + base_w.saturating_sub(1)) / base_w).min(view_w_u64)
            as u32,
    );
    let src_y_end = view_y.saturating_add(
        ((rel_y1.saturating_mul(view_h_u64) + base_h.saturating_sub(1)) / base_h).min(view_h_u64)
            as u32,
    );

    if src_x >= image_w || src_y >= image_h {
        return None;
    }

    let src_w = src_x_end
        .saturating_sub(src_x)
        .max(1)
        .min(image_w.saturating_sub(src_x));
    let src_h = src_y_end
        .saturating_sub(src_y)
        .max(1)
        .min(image_h.saturating_sub(src_y));

    if src_w == 0 || src_h == 0 {
        return None;
    }

    Some((src_x, src_y, src_w, src_h))
}

pub fn cover_viewport(
    image_w: u32,
    image_h: u32,
    target_w: u16,
    target_h: u16,
) -> (u32, u32, u32, u32) {
    if target_w == 0 || target_h == 0 || image_w == 0 || image_h == 0 {
        return (0, 0, image_w.max(1), image_h.max(1));
    }

    let image_ratio = image_w as f64 / image_h as f64;
    // Cell geometry in terminal space: 2 columns ~= 1 row in physical size.
    // Keep image aspect visually correct by using cell pixel ratio for viewport fitting.
    let target_ratio = (target_w as f64 * CELL_W_PX as f64) / (target_h as f64 * CELL_H_PX as f64);

    if (image_ratio - target_ratio).abs() < f64::EPSILON {
        return (0, 0, image_w, image_h);
    }

    if image_ratio > target_ratio {
        let crop_w = ((image_h as f64) * target_ratio)
            .round()
            .clamp(1.0, image_w as f64) as u32;
        let crop_x = (image_w.saturating_sub(crop_w)) / 2;
        (crop_x, 0, crop_w, image_h)
    } else {
        let crop_h = ((image_w as f64) / target_ratio)
            .round()
            .clamp(1.0, image_h as f64) as u32;
        let crop_y = (image_h.saturating_sub(crop_h)) / 2;
        (0, crop_y, image_w, crop_h)
    }
}
