use image::{DynamicImage, GenericImageView};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};
use std::collections::HashMap;

use crate::{
    data::config::GraphicsProtocol,
    render::graphics_overlay::{map_segment_to_cover_crop, visible_segments_after_occluders},
    tmplayer::app::state::AppState,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SegmentKey {
    slot: TmCoverSlot,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TmCoverSlot {
    InfoCover,
    PlaylistCover,
}

pub struct TmKittyOverlay {
    picker: Picker,
    last_term_size: Option<(u16, u16)>,
    last_content_hash: Option<u64>,
    info_cover_image: Option<DynamicImage>,
    playlist_cover_image: Option<DynamicImage>,
    segment_protocols: HashMap<SegmentKey, StatefulProtocol>,
}

impl TmKittyOverlay {
    pub fn new(graphics_protocol: GraphicsProtocol) -> Self {
        let mut picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
        if let Some(proto) = graphics_protocol.to_ratatui_protocol() {
            picker.set_protocol_type(proto);
        }
        Self {
            picker,
            last_term_size: None,
            last_content_hash: None,
            info_cover_image: None,
            playlist_cover_image: None,
            segment_protocols: HashMap::new(),
        }
    }

    pub fn paint(
        &mut self,
        app: &AppState,
        frame: &mut Frame<'_>,
        info_cover_bytes: Option<&[u8]>,
        playlist_cover_bytes: Option<&[u8]>,
        info_rect: Option<Rect>,
        playlist_rect: Option<Rect>,
        modal_area: Option<Rect>,
    ) {
        if app.config.graphics_protocol == GraphicsProtocol::Off {
            self.clear_all();
            return;
        }

        let size = frame.area();
        let current_size = (size.width, size.height);

        if self.last_term_size != Some(current_size) {
            self.last_term_size = Some(current_size);
            self.clear_all();
        }

        let content_hash = compute_content_hash(
            info_cover_bytes,
            playlist_cover_bytes,
            info_rect,
            playlist_rect,
        );

        if self.last_content_hash != Some(content_hash) {
            self.last_content_hash = Some(content_hash);
            self.clear_all();
        }

        if let Some(bytes) = info_cover_bytes {
            if self.info_cover_image.is_none() {
                if let Ok(img) = image::load_from_memory(bytes) {
                    self.info_cover_image = Some(img);
                }
            }
        } else {
            self.info_cover_image = None;
        }

        if let Some(bytes) = playlist_cover_bytes {
            if self.playlist_cover_image.is_none() {
                if let Ok(img) = image::load_from_memory(bytes) {
                    self.playlist_cover_image = Some(img);
                }
            }
        } else {
            self.playlist_cover_image = None;
        }

        let mut paint = |rect: Rect, slot: TmCoverSlot, img: &DynamicImage| {
            let occluders: Vec<Rect> = modal_area.into_iter().collect();
            let segments = visible_segments_after_occluders(rect, &occluders);

            for segment in segments {
                let segment_key = SegmentKey {
                    slot,
                    x: segment.x,
                    y: segment.y,
                    width: segment.width,
                    height: segment.height,
                };

                if !self.segment_protocols.contains_key(&segment_key) {
                    let (img_w, img_h) = img.dimensions();
                    let Some((crop_x, crop_y, crop_w, crop_h)) =
                        map_segment_to_cover_crop(rect, segment, img_w, img_h)
                    else {
                        continue;
                    };
                    let cropped = image::DynamicImage::ImageRgba8(img.to_rgba8())
                        .crop_imm(crop_x, crop_y, crop_w, crop_h);
                    let proto = self.picker.new_resize_protocol(cropped);
                    self.segment_protocols.insert(segment_key.clone(), proto);
                }

                if let Some(proto) = self.segment_protocols.get_mut(&segment_key) {
                    let widget = StatefulImage::default();
                    frame.render_stateful_widget(widget, segment, proto);
                }

                if let Some(proto) = self.segment_protocols.get_mut(&segment_key) {
                    let widget = StatefulImage::default();
                    frame.render_stateful_widget(widget, segment, proto);
                }
            }
        };

        if let Some(img) = &self.info_cover_image {
            if let Some(rect) = info_rect {
                paint(rect, TmCoverSlot::InfoCover, img);
            }
        }

        if let Some(img) = &self.playlist_cover_image {
            if let Some(rect) = playlist_rect {
                paint(rect, TmCoverSlot::PlaylistCover, img);
            }
        }
    }

    fn clear_all(&mut self) {
        self.info_cover_image = None;
        self.playlist_cover_image = None;
        self.segment_protocols.clear();
    }
}

fn compute_content_hash(
    info: Option<&[u8]>,
    playlist: Option<&[u8]>,
    info_rect: Option<Rect>,
    playlist_rect: Option<Rect>,
) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    if let Some(bytes) = info {
        bytes.as_ptr().hash(&mut hasher);
    }
    if let Some(bytes) = playlist {
        bytes.as_ptr().hash(&mut hasher);
    }
    if let Some(r) = info_rect {
        (r.x, r.y, r.width, r.height).hash(&mut hasher);
    }
    if let Some(r) = playlist_rect {
        (r.x, r.y, r.width, r.height).hash(&mut hasher);
    }
    hasher.finish()
}
