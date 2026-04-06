use crate::app::{App, HitRect, Overlay, Page};
use crate::tmplayer::render::kitty_graphics;
use crate::tmplayer::utils::kitty::kitty_graphics_supported;
use ratatui::layout::Rect;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

const CELL_W_PX: u32 = 8;
const CELL_H_PX: u32 = 16;
const SLOT_SEGMENT_CAP: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CoverSlotKey {
    PlaylistHeader,
    AuthorHeader,
    HomeTile(usize),
    AuthorTile(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SlotPlacementState {
    hash: u64,
    image_id: u32,
    segments: Vec<(u16, u16, u16, u16)>,
}

struct CoverTarget<'a> {
    slot: CoverSlotKey,
    base_rect: Rect,
    bytes: &'a [u8],
}

pub struct MainKittyOverlay {
    image_ids: HashMap<u64, u32>,
    image_sizes: HashMap<u64, (u32, u32)>,
    transmitted: HashSet<u64>,
    next_image_id: u32,
    last_slots: HashMap<CoverSlotKey, SlotPlacementState>,
    last_term_size: Option<(u16, u16)>,
    last_quality: u8,
}

impl MainKittyOverlay {
    pub fn new() -> Self {
        Self {
            image_ids: HashMap::new(),
            image_sizes: HashMap::new(),
            transmitted: HashSet::new(),
            next_image_id: 5000,
            last_slots: HashMap::new(),
            last_term_size: None,
            last_quality: 0,
        }
    }

    pub fn on_terminal_reset(&mut self) {
        self.last_slots.clear();
        self.transmitted.clear();
        self.image_sizes.clear();
        self.last_term_size = None;
    }

    pub fn paint(&mut self, app: &App, size: Rect) {
        let kitty_enabled = app.config.kitty_graphics && kitty_graphics_supported();
        if !kitty_enabled {
            self.clear_all();
            return;
        }

        let quality = app.config.kitty_cover_scale_percent.clamp(25, 100);
        if self.last_quality == 0 {
            self.last_quality = quality;
        }

        if self.last_quality != quality {
            self.last_quality = quality;
            self.clear_all();
        }

        let current_size = (size.width, size.height);
        if self.last_term_size != Some(current_size) {
            self.last_term_size = Some(current_size);
            // Terminal resize can drop placements; force re-place on next step.
            self.last_slots.clear();
        }

        let targets = collect_cover_targets(app, size);
        let occluders = collect_occluders(app, size);
        let old_slots = self.last_slots.clone();
        let mut new_slots: HashMap<CoverSlotKey, SlotPlacementState> = HashMap::new();

        for target in targets {
            if target.base_rect.width == 0
                || target.base_rect.height == 0
                || target.bytes.is_empty()
            {
                continue;
            }

            let segments = visible_segments_after_occluders(target.base_rect, &occluders);
            if segments.is_empty() {
                continue;
            }

            let (max_w, max_h) =
                target_px(target.base_rect.width, target.base_rect.height, quality);

            let bytes_hash = hash_bytes(target.bytes);
            let render_hash = hash_render_variant(bytes_hash, max_w, max_h, quality);

            if !self.transmitted.contains(&render_hash) {
                self.transmit_image_variant(render_hash, target.bytes, max_w, max_h);
            }

            if !self.transmitted.contains(&render_hash) {
                continue;
            }

            let image_id = self.image_id_for_hash(render_hash);
            let Some(&(img_w, img_h)) = self.image_sizes.get(&render_hash) else {
                continue;
            };

            let new_state = SlotPlacementState {
                hash: render_hash,
                image_id,
                segments: rect_segments_signature(&segments),
            };

            let prev_state = old_slots.get(&target.slot);
            if prev_state != Some(&new_state) {
                if let Some(prev) = prev_state {
                    self.delete_slot_placements(target.slot, prev.image_id);
                }

                for (idx, segment) in segments.iter().take(SLOT_SEGMENT_CAP).enumerate() {
                    if let Some((src_x, src_y, src_w, src_h)) =
                        map_segment_to_cover_crop(target.base_rect, *segment, img_w, img_h)
                    {
                        let placement_id = placement_id_for_segment(target.slot, idx as u32);
                        let _ = kitty_graphics::place_image_cropped(
                            *segment,
                            image_id,
                            placement_id,
                            src_x,
                            src_y,
                            src_w,
                            src_h,
                        );
                    }
                }
            }

            new_slots.insert(target.slot, new_state);
        }

        for (slot, state) in old_slots {
            if !new_slots.contains_key(&slot) {
                self.delete_slot_placements(slot, state.image_id);
            }
        }

        self.last_slots = new_slots;
    }

    fn image_id_for_hash(&mut self, hash: u64) -> u32 {
        if let Some(id) = self.image_ids.get(&hash).copied() {
            return id;
        }
        let id = self.next_image_id;
        self.next_image_id = self.next_image_id.saturating_add(1);
        self.image_ids.insert(hash, id);
        id
    }

    fn transmit_image_variant(&mut self, hash: u64, bytes: &[u8], max_w: u32, max_h: u32) {
        let image_id = self.image_id_for_hash(hash);

        let Some((b64, w, h)) =
            kitty_graphics::encode_image_bytes_to_png_base64(bytes, max_w, max_h)
        else {
            return;
        };

        if kitty_graphics::transmit_png_base64(image_id, &b64).is_ok() {
            self.transmitted.insert(hash);
            self.image_sizes.insert(hash, (w, h));
        }
    }

    fn delete_slot_placements(&self, slot: CoverSlotKey, image_id: u32) {
        for seg_idx in 0..SLOT_SEGMENT_CAP {
            let placement_id = placement_id_for_segment(slot, seg_idx as u32);
            let _ = kitty_graphics::delete_image_placement(image_id, placement_id, false);
        }
    }

    fn clear_all(&mut self) {
        let stale_slots: Vec<(CoverSlotKey, SlotPlacementState)> =
            self.last_slots.drain().collect();
        for (slot, state) in stale_slots {
            self.delete_slot_placements(slot, state.image_id);
        }

        for &image_id in self.image_ids.values() {
            let _ = kitty_graphics::delete_image(image_id, true);
        }

        self.image_ids.clear();
        self.image_sizes.clear();
        self.transmitted.clear();
        self.next_image_id = 5000;
        self.last_term_size = None;
        self.last_quality = 0;
    }
}

fn collect_cover_targets<'a>(app: &'a App, size: Rect) -> Vec<CoverTarget<'a>> {
    let mut targets = Vec::new();

    match app.page {
        Page::Home => {
            for (hit, index) in &app.home_tile_hits {
                let Some(tile) = app.home.tiles.get(*index) else {
                    continue;
                };
                let Some(bytes) = tile.cover_bytes.as_deref() else {
                    continue;
                };

                let tile_rect = rect_from_hit(*hit);
                let cover_rect = tile_cover_rect(tile_rect);
                if cover_rect.width == 0 || cover_rect.height == 0 {
                    continue;
                }

                targets.push(CoverTarget {
                    slot: CoverSlotKey::HomeTile(*index),
                    base_rect: cover_rect,
                    bytes,
                });
            }
        }
        Page::Playlist => {
            if let Some(cover_rect) = playlist_header_cover_rect(app, size) {
                if let Some(bytes) = app.playlist.cover_bytes.as_deref() {
                    targets.push(CoverTarget {
                        slot: CoverSlotKey::PlaylistHeader,
                        base_rect: cover_rect,
                        bytes,
                    });
                }
            }
        }
        Page::Author => {
            if let Some(cover_rect) = author_header_cover_rect(app, size) {
                if let Some(bytes) = app.author.cover_bytes.as_deref() {
                    targets.push(CoverTarget {
                        slot: CoverSlotKey::AuthorHeader,
                        base_rect: cover_rect,
                        bytes,
                    });
                }
            }

            for (hit, index) in &app.author_tile_hits {
                let Some(tile) = app.author.tiles.get(*index) else {
                    continue;
                };
                let Some(bytes) = tile.cover_bytes.as_deref() else {
                    continue;
                };

                let tile_rect = rect_from_hit(*hit);
                let cover_rect = tile_cover_rect(tile_rect);
                if cover_rect.width == 0 || cover_rect.height == 0 {
                    continue;
                }

                targets.push(CoverTarget {
                    slot: CoverSlotKey::AuthorTile(*index),
                    base_rect: cover_rect,
                    bytes,
                });
            }
        }
        _ => {}
    }

    targets
}

fn playlist_header_cover_rect(app: &App, size: Rect) -> Option<Rect> {
    if size.width < 40 || size.height < 14 {
        return None;
    }

    let rows = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(crate::ui::player_bar::PLAYER_BAR_HEIGHT),
        ])
        .split(size);

    let content_area = if app.config.show_hints {
        ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Min(1),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(rows[0])[0]
    } else {
        rows[0]
    };

    let main = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage(34),
            ratatui::layout::Constraint::Percentage(66),
        ])
        .split(content_area);

    let header = main[0];
    let inner = header.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    if inner.width < 6 || inner.height < 3 {
        return None;
    }

    let cols = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Length((inner.height * 2).min(26)),
            ratatui::layout::Constraint::Min(1),
        ])
        .split(inner);

    let cover_block = centered_visual_square_block(cols[0]);
    if cover_block.width == 0 || cover_block.height == 0 {
        return None;
    }

    let cover_rect = cover_block.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    if cover_rect.width == 0 || cover_rect.height == 0 {
        return None;
    }

    Some(cover_rect)
}

fn author_header_cover_rect(app: &App, size: Rect) -> Option<Rect> {
    if size.width < 40 || size.height < 14 {
        return None;
    }

    let rows = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(crate::ui::player_bar::PLAYER_BAR_HEIGHT),
        ])
        .split(size);

    let content_area = if app.config.show_hints {
        ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Min(1),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(rows[0])[0]
    } else {
        rows[0]
    };

    let main = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage(34),
            ratatui::layout::Constraint::Percentage(66),
        ])
        .split(content_area);

    let header = main[0];
    let inner = header.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    if inner.width < 6 || inner.height < 3 {
        return None;
    }

    let cols = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Length((inner.height * 2).min(26)),
            ratatui::layout::Constraint::Min(1),
        ])
        .split(inner);

    let cover_block = centered_visual_square_block(cols[0]);
    if cover_block.width == 0 || cover_block.height == 0 {
        return None;
    }

    let cover_rect = cover_block.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    if cover_rect.width == 0 || cover_rect.height == 0 {
        return None;
    }

    Some(cover_rect)
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

fn tile_cover_rect(tile_rect: Rect) -> Rect {
    if tile_rect.width == 0 || tile_rect.height == 0 {
        return Rect::default();
    }

    let inner_rect = tile_rect.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    if inner_rect.width < 2 || inner_rect.height < 2 {
        return Rect::default();
    }

    let text_rows = if inner_rect.height >= 4 { 2 } else { 1 };
    let cover_height = inner_rect.height.saturating_sub(text_rows);
    Rect {
        x: inner_rect.x,
        y: inner_rect.y,
        width: inner_rect.width,
        height: cover_height,
    }
}

fn collect_occluders(app: &App, size: Rect) -> Vec<Rect> {
    let mut occluders = Vec::new();

    if let Some(rect) = settings_modal_area(size, app.overlay) {
        occluders.push(rect);
    }

    if let Some(rect) = search_box_area(size, app.search_box_anim_height) {
        occluders.push(rect);
    }

    if app.page == Page::Home {
        if let Some(hit) = app.home_sidebar_panel_hit {
            let rect = rect_from_hit(hit);
            if rect.width > 0 && rect.height > 0 {
                occluders.push(rect);
            }
        }
    }

    occluders
}

fn settings_modal_area(size: Rect, overlay: Option<Overlay>) -> Option<Rect> {
    match overlay {
        Some(Overlay::Settings)
        | Some(Overlay::SettingsPlayback)
        | Some(Overlay::SettingsKeybinds) => Some(centered_rect(70, 20, size)),
        Some(Overlay::SettingsAbout) => Some(centered_rect(70, 22, size)),
        _ => None,
    }
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

fn search_box_area(size: Rect, anim_h: u16) -> Option<Rect> {
    if size.width < 20 || size.height < 2 {
        return None;
    }

    let visible_h = anim_h
        .min(crate::ui::search_box::TARGET_HEIGHT)
        .min(size.height);
    if visible_h == 0 {
        return None;
    }

    let width = (size.width / 2).max(24).min(size.width.saturating_sub(2));
    Some(Rect {
        x: size.x + size.width.saturating_sub(width) / 2,
        y: size.y,
        width,
        height: visible_h,
    })
}

fn visible_segments_after_occluders(base: Rect, occluders: &[Rect]) -> Vec<Rect> {
    let mut segments = vec![base];

    for occluder in occluders {
        let mut next = Vec::new();
        for segment in segments {
            next.extend(subtract_rect_segments(segment, *occluder));
        }
        segments = next;
        if segments.is_empty() {
            break;
        }
    }

    segments
}

fn intersect_rect(a: Rect, b: Rect) -> Option<Rect> {
    let ax2 = a.x.saturating_add(a.width);
    let ay2 = a.y.saturating_add(a.height);
    let bx2 = b.x.saturating_add(b.width);
    let by2 = b.y.saturating_add(b.height);

    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = ax2.min(bx2);
    let y2 = ay2.min(by2);

    if x2 <= x1 || y2 <= y1 {
        return None;
    }

    Some(Rect {
        x: x1,
        y: y1,
        width: x2 - x1,
        height: y2 - y1,
    })
}

fn subtract_rect_segments(base: Rect, cut: Rect) -> Vec<Rect> {
    let Some(overlap) = intersect_rect(base, cut) else {
        return vec![base];
    };

    let base_right = base.x.saturating_add(base.width);
    let base_bottom = base.y.saturating_add(base.height);
    let overlap_right = overlap.x.saturating_add(overlap.width);
    let overlap_bottom = overlap.y.saturating_add(overlap.height);

    let mut segments = Vec::with_capacity(4);

    if overlap.y > base.y {
        segments.push(Rect {
            x: base.x,
            y: base.y,
            width: base.width,
            height: overlap.y - base.y,
        });
    }

    if overlap_bottom < base_bottom {
        segments.push(Rect {
            x: base.x,
            y: overlap_bottom,
            width: base.width,
            height: base_bottom - overlap_bottom,
        });
    }

    if overlap.height > 0 {
        if overlap.x > base.x {
            segments.push(Rect {
                x: base.x,
                y: overlap.y,
                width: overlap.x - base.x,
                height: overlap.height,
            });
        }
        if overlap_right < base_right {
            segments.push(Rect {
                x: overlap_right,
                y: overlap.y,
                width: base_right - overlap_right,
                height: overlap.height,
            });
        }
    }

    segments
        .into_iter()
        .filter(|rect| rect.width > 0 && rect.height > 0)
        .collect()
}

fn map_segment_to_cover_crop(
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

fn cover_viewport(
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

fn target_px(w_cells: u16, h_cells: u16, quality: u8) -> (u32, u32) {
    let q = quality.clamp(25, 100);
    if q >= 100 {
        return (u32::MAX, u32::MAX);
    }

    let scale = q as u32;
    let w = (w_cells as u32)
        .saturating_mul(CELL_W_PX)
        .saturating_mul(scale)
        / 100;
    let h = (h_cells as u32)
        .saturating_mul(CELL_H_PX)
        .saturating_mul(scale)
        / 100;

    (w.clamp(64, 1024), h.clamp(64, 1024))
}

fn placement_base_for_slot(slot: CoverSlotKey) -> u32 {
    match slot {
        CoverSlotKey::PlaylistHeader => 1_000_000,
        CoverSlotKey::AuthorHeader => 1_010_000,
        CoverSlotKey::HomeTile(index) => {
            2_000_000_u32.saturating_add((index as u32).saturating_mul(SLOT_SEGMENT_CAP as u32))
        }
        CoverSlotKey::AuthorTile(index) => {
            3_000_000_u32.saturating_add((index as u32).saturating_mul(SLOT_SEGMENT_CAP as u32))
        }
    }
}

fn placement_id_for_segment(slot: CoverSlotKey, seg_idx: u32) -> u32 {
    placement_base_for_slot(slot).saturating_add(seg_idx)
}

fn rect_segments_signature(rects: &[Rect]) -> Vec<(u16, u16, u16, u16)> {
    rects
        .iter()
        .map(|rect| (rect.x, rect.y, rect.width, rect.height))
        .collect()
}

fn rect_from_hit(hit: HitRect) -> Rect {
    Rect {
        x: hit.x,
        y: hit.y,
        width: hit.width,
        height: hit.height,
    }
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn hash_render_variant(content_hash: u64, max_w: u32, max_h: u32, quality: u8) -> u64 {
    let mut hasher = DefaultHasher::new();
    content_hash.hash(&mut hasher);
    max_w.hash(&mut hasher);
    max_h.hash(&mut hasher);
    quality.hash(&mut hasher);
    hasher.finish()
}
