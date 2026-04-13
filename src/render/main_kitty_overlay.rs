use crate::app::peek_shared_future;
use crate::app::{App, HitRect, Overlay, Page};
use crate::data::config::GraphicsProtocol;
use image::{GenericImage, RgbImage};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const CELL_W_PX: u32 = 8;
const CELL_H_PX: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CoverSlotKey {
    PlaylistHeader,
    AuthorHeader,
    HomeTile(usize),
    AuthorTile(usize),
}

struct CoverTarget<'a> {
    slot: CoverSlotKey,
    base_rect: Rect,
    bytes: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SegmentKey {
    slot: CoverSlotKey,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

pub struct MainKittyOverlay {
    picker: Picker,
    last_term_size: Option<(u16, u16)>,
    last_content_hash: Option<u64>,
    playlist_header_image: Option<RgbImage>,
    author_header_image: Option<RgbImage>,
    home_tile_images: HashMap<usize, RgbImage>,
    author_tile_images: HashMap<usize, RgbImage>,
    segment_protocols: HashMap<SegmentKey, StatefulProtocol>,
}

impl MainKittyOverlay {
    pub fn new(graphics_protocol: GraphicsProtocol) -> Self {
        let mut picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
        if let Some(proto) = graphics_protocol.to_ratatui_protocol() {
            picker.set_protocol_type(proto);
        }
        Self {
            picker,
            last_term_size: None,
            last_content_hash: None,
            playlist_header_image: None,
            author_header_image: None,
            home_tile_images: HashMap::new(),
            author_tile_images: HashMap::new(),
            segment_protocols: HashMap::new(),
        }
    }

    pub fn on_terminal_reset(&mut self) {
        self.last_term_size = None;
        self.last_content_hash = None;
        self.playlist_header_image = None;
        self.author_header_image = None;
        self.home_tile_images.clear();
        self.author_tile_images.clear();
        self.segment_protocols.clear();
    }

    pub fn paint(&mut self, app: &mut App, frame: &mut Frame<'_>) {
        if app.config.graphics_protocol == GraphicsProtocol::Off {
            self.clear_all();
            return;
        }

        let size = frame.area();
        let current_size = (size.width, size.height);

        if self.last_term_size != Some(current_size) {
            self.last_term_size = Some(current_size);
            self.playlist_header_image = None;
            self.author_header_image = None;
            self.home_tile_images.clear();
            self.author_tile_images.clear();
            self.segment_protocols.clear();
        }

        let occluders = collect_occluders(app, size);
        let targets = collect_cover_targets(app, size);

        let content_hash = compute_targets_content_hash(&targets);
        if self.last_content_hash != Some(content_hash) {
            self.last_content_hash = Some(content_hash);
            self.playlist_header_image = None;
            self.author_header_image = None;
            self.home_tile_images.clear();
            self.author_tile_images.clear();
            self.segment_protocols.clear();
        }

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

            let img = match target.slot {
                CoverSlotKey::PlaylistHeader => {
                    if self.playlist_header_image.is_none() {
                        if let Ok(dyn_img) = image::load_from_memory(target.bytes) {
                            self.playlist_header_image = Some(dyn_img.to_rgb8());
                        }
                    }
                    match &self.playlist_header_image {
                        Some(img) => img,
                        None => continue,
                    }
                }
                CoverSlotKey::AuthorHeader => {
                    if self.author_header_image.is_none() {
                        if let Ok(dyn_img) = image::load_from_memory(target.bytes) {
                            self.author_header_image = Some(dyn_img.to_rgb8());
                        }
                    }
                    match &self.author_header_image {
                        Some(img) => img,
                        None => continue,
                    }
                }
                CoverSlotKey::HomeTile(index) => {
                    if !self.home_tile_images.contains_key(&index) {
                        if let Ok(dyn_img) = image::load_from_memory(target.bytes) {
                            self.home_tile_images.insert(index, dyn_img.to_rgb8());
                        }
                    }
                    match self.home_tile_images.get(&index) {
                        Some(img) => img,
                        None => continue,
                    }
                }
                CoverSlotKey::AuthorTile(index) => {
                    if !self.author_tile_images.contains_key(&index) {
                        if let Ok(dyn_img) = image::load_from_memory(target.bytes) {
                            self.author_tile_images.insert(index, dyn_img.to_rgb8());
                        }
                    }
                    match self.author_tile_images.get(&index) {
                        Some(img) => img,
                        None => continue,
                    }
                }
            };
            let (img_w, img_h) = img.dimensions();

            for segment in segments {
                let segment_key = SegmentKey {
                    slot: target.slot,
                    x: segment.x,
                    y: segment.y,
                    width: segment.width,
                    height: segment.height,
                };

                if !self.segment_protocols.contains_key(&segment_key) {
                    let Some((src_x, src_y, src_w, src_h)) =
                        map_segment_to_cover_crop(target.base_rect, segment, img_w, img_h)
                    else {
                        continue;
                    };
                    let cropped = img.clone().sub_image(src_x, src_y, src_w, src_h).to_image();
                    let proto = self
                        .picker
                        .new_resize_protocol(image::DynamicImage::ImageRgb8(cropped));
                    self.segment_protocols.insert(segment_key.clone(), proto);
                }

                if let Some(ref mut proto) = self.segment_protocols.get_mut(&segment_key) {
                    let widget = StatefulImage::<StatefulProtocol>::default();
                    frame.render_stateful_widget(widget, segment, proto);
                }
            }
        }
    }

    fn clear_all(&mut self) {
        self.last_content_hash = None;
        self.playlist_header_image = None;
        self.author_header_image = None;
        self.home_tile_images.clear();
        self.author_tile_images.clear();
        self.segment_protocols.clear();
    }
}

fn collect_cover_targets(app: &mut App, size: Rect) -> Vec<CoverTarget<'_>> {
    let mut targets = Vec::new();

    match app.page {
        Page::Home => {
            let home_data: Vec<(HitRect, usize, &[u8])> = app
                .home_tile_hits
                .iter()
                .filter_map(|(hit, index)| {
                    let tile = app.home.tiles.get(*index)?;
                    let bytes = peek_shared_future(&tile.cover_bytes)?;
                    Some((*hit, *index, &bytes[..]))
                })
                .collect();
            for (hit, index, bytes) in home_data {
                let tile_rect = rect_from_hit(hit);
                let cover_rect = tile_cover_rect(tile_rect);
                if cover_rect.width == 0 || cover_rect.height == 0 {
                    continue;
                }
                targets.push(CoverTarget {
                    slot: CoverSlotKey::HomeTile(index),
                    base_rect: cover_rect,
                    bytes,
                });
            }
        }
        Page::Playlist => {
            if let Some(cover_rect) = playlist_header_cover_rect(app, size) {
                if let Some(bytes) = peek_shared_future(&app.playlist.cover_bytes) {
                    targets.push(CoverTarget {
                        slot: CoverSlotKey::PlaylistHeader,
                        base_rect: cover_rect,
                        bytes: bytes.as_ref(),
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

fn rect_from_hit(hit: HitRect) -> Rect {
    Rect {
        x: hit.x,
        y: hit.y,
        width: hit.width,
        height: hit.height,
    }
}

fn compute_targets_content_hash(targets: &[CoverTarget]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for target in targets {
        target.slot.hash(&mut hasher);
        target.base_rect.hash(&mut hasher);
        target.bytes.as_ptr().hash(&mut hasher);
    }
    hasher.finish()
}
