use image::{DynamicImage, GenericImageView};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::{cell::LazyCell, collections::HashMap};

use crate::{
    data::config::GraphicsProtocol, render::graphics_overlay::map_segment_to_cover_crop,
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

pub struct GraphicsOverlay {
    picker: Picker,
    last_term_size: Option<(u16, u16)>,
    last_content_hash: Option<u64>,
    segment_protocols: HashMap<SegmentKey, StatefulProtocol>,
}

impl GraphicsOverlay {
    pub fn new(graphics_protocol: GraphicsProtocol) -> Self {
        let mut picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
        if let Some(proto) = graphics_protocol.to_ratatui_protocol() {
            picker.set_protocol_type(proto);
        }
        Self {
            picker,
            last_term_size: None,
            last_content_hash: None,
            segment_protocols: HashMap::new(),
        }
    }

    pub fn paint(
        &mut self,
        app: &AppState,
        frame: &mut Frame<'_>,
        info_cover: Option<&[u8]>,
        playlist_cover: Option<&[u8]>,
        info_rect: Option<Rect>,
        playlist_rect: Option<Rect>,
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

        let hash = compute_content_hash(info_cover, playlist_cover, info_rect, playlist_rect);

        if self.last_content_hash != Some(hash) {
            self.last_content_hash = Some(hash);
            self.clear_all();
        }

        let info_image_fn = || info_cover.and_then(|x| image::load_from_memory(x).ok());
        let playlist_image_fn = || playlist_cover.and_then(|x| image::load_from_memory(x).ok());

        let mut paint = |rect: Rect, slot: TmCoverSlot, img: &dyn Fn() -> Option<DynamicImage>| {
            let segments = vec![rect];
            let img = LazyCell::new(img);

            for segment in segments {
                let segment_key = SegmentKey {
                    slot,
                    x: segment.x,
                    y: segment.y,
                    width: segment.width,
                    height: segment.height,
                };

                let need_init = !self.segment_protocols.contains_key(&segment_key);
                if need_init && let Some(img) = &*img {
                    let (img_w, img_h) = img.dimensions();
                    let crop = map_segment_to_cover_crop(rect, segment, img_w, img_h);
                    let Some((crop_x, crop_y, crop_w, crop_h)) = crop else {
                        continue;
                    };
                    let cropped = img.crop_imm(crop_x, crop_y, crop_w, crop_h);
                    let proto = self.picker.new_resize_protocol(cropped);
                    self.segment_protocols.insert(segment_key.clone(), proto);
                }

                if let Some(proto) = self.segment_protocols.get_mut(&segment_key) {
                    let widget = StatefulImage::default();
                    frame.render_stateful_widget(widget, segment, proto);
                }
            }
        };

        if let Some(rect) = info_rect {
            paint(rect, TmCoverSlot::InfoCover, &info_image_fn);
        }

        if let Some(rect) = playlist_rect {
            paint(rect, TmCoverSlot::PlaylistCover, &playlist_image_fn);
        }
    }

    fn clear_all(&mut self) {
        self.segment_protocols.clear();
    }
}

fn compute_content_hash(
    info: Option<&[u8]>,
    playlist: Option<&[u8]>,
    info_rect: Option<Rect>,
    playlist_rect: Option<Rect>,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    info.map(|x| x.as_ptr()).hash(&mut hasher);
    playlist.map(|x| x.as_ptr()).hash(&mut hasher);
    info_rect.hash(&mut hasher);
    playlist_rect.hash(&mut hasher);
    hasher.finish()
}
