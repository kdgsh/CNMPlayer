use crate::data::assets;
use crate::ui::theme::{detect_color_capability, Theme, ThemeName, ThemePalette};
use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

pub struct ThemeLoader;

#[derive(Debug, Deserialize)]
struct ThemeToml {
    text: String,
    subtext: String,
    base: String,
    surface: String,
    accent: String,
    accent2: String,
    accent3: String,
}

impl ThemeLoader {
    pub fn load(name: &str) -> Result<Theme> {
        let _ = assets::ensure_assets_ready();
        let name = ThemeName::from_str_or_system(name);

        let rel = match name {
            ThemeName::System => PathBuf::from("themes/system.toml"),
            ThemeName::Latte => PathBuf::from("themes/catppuccin_latte.toml"),
            ThemeName::Frappe => PathBuf::from("themes/catppuccin_frappe.toml"),
            ThemeName::Macchiato => PathBuf::from("themes/catppuccin_macchiato.toml"),
            ThemeName::Mocha => PathBuf::from("themes/catppuccin_mocha.toml"),
        };

        let path = assets::resolve_asset_path(&rel);
        let raw = fs::read_to_string(path)?;
        let parsed: ThemeToml = toml::from_str(&raw)?;

        Ok(Theme {
            name,
            capability: detect_color_capability(),
            palette: ThemePalette {
                text: parse_hex(&parsed.text),
                subtext: parse_hex(&parsed.subtext),
                base: parse_hex(&parsed.base),
                surface: parse_hex(&parsed.surface),
                accent: parse_hex(&parsed.accent),
                accent2: parse_hex(&parsed.accent2),
                accent3: parse_hex(&parsed.accent3),
            },
        })
    }
}

fn parse_hex(raw: &str) -> (u8, u8, u8) {
    let hex = raw.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return (255, 255, 255);
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
    (r, g, b)
}
