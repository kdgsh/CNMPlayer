use std::path::PathBuf;
use std::process::Command;

pub struct SystemColors {
    pub text: (u8, u8, u8),
    pub subtext: (u8, u8, u8),
    pub base: (u8, u8, u8),
    pub surface: (u8, u8, u8),
    pub buff: (u8, u8, u8),
    pub accent: (u8, u8, u8),
    pub accent2: (u8, u8, u8),
    pub accent3: (u8, u8, u8),
}

fn home_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(dir)
    } else if let Ok(home) = std::env::var("HOME") {
        home.parse::<PathBuf>().unwrap().join(".config")
    } else {
        PathBuf::from("/home").join(std::env::var("USER").unwrap_or_default()).join(".config")
    }
}

fn detect_dms_accent() -> Option<(u8, u8, u8)> {
    let colors_conf = home_config_dir().join("hypr/dms/colors.conf");
    if !colors_conf.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&colors_conf).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("$primary") {
            if let Some(start) = trimmed.find("rgb(") {
                let rest = &trimmed[start + 4..];
                if let Some(end) = rest.find(')') {
                    let hex = &rest[..end].trim();
                    return Some(parse_hex(hex));
                }
            }
        }
    }
    None
}

fn detect_system_dark_mode() -> bool {
    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            let lower = stdout.to_lowercase();
            if lower.contains("prefer-dark") {
                return true;
            }
            if lower.contains("prefer-light") {
                return false;
            }
        }
    }

    if let Ok(output) = Command::new("dconf")
        .args(["read", "/org/gnome/desktop/interface/color-scheme"])
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            let lower = stdout.to_lowercase();
            if lower.contains("prefer-dark") {
                return true;
            }
            if lower.contains("prefer-light") {
                return false;
            }
        }
    }

    if let Ok(dark) = std::env::var("DARK_MODE") {
        return matches!(dark.to_lowercase().as_str(), "1" | "true" | "yes");
    }

    true
}

fn generate_system_palette(accent: (u8, u8, u8), is_dark: bool) -> SystemColors {
    let accent = if is_dark { accent } else { ensure_contrast_on_light(accent) };
    if is_dark {
        SystemColors {
            base: (10, 10, 10),
            surface: (20, 20, 20),
            buff: (30, 30, 30),
            text: (224, 224, 224),
            subtext: (136, 136, 136),
            accent,
            accent2: adjust_lightness(accent, 0.12),
            accent3: adjust_lightness(accent, -0.12),
        }
    } else {
        SystemColors {
            base: (240, 240, 240),
            surface: (224, 224, 224),
            buff: (213, 213, 213),
            text: (26, 26, 26),
            subtext: (102, 102, 102),
            accent,
            accent2: adjust_lightness(accent, -0.12),
            accent3: adjust_lightness(accent, 0.10),
        }
    }
}

fn adjust_lightness(rgb: (u8, u8, u8), delta: f32) -> (u8, u8, u8) {
    let (h, s, l) = rgb_to_hsl(rgb);
    hsl_to_rgb((h, s, (l + delta).clamp(0.05, 0.95)))
}

fn rgb_to_hsl(rgb: (u8, u8, u8)) -> (f32, f32, f32) {
    let r = rgb.0 as f32 / 255.0;
    let g = rgb.1 as f32 / 255.0;
    let b = rgb.2 as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };

    let h = if (max - r).abs() < f32::EPSILON {
        ((g - b) / d) + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < f32::EPSILON {
        ((b - r) / d) + 2.0
    } else {
        ((r - g) / d) + 4.0
    };

    (h / 6.0 * 360.0, s, l)
}

fn hsl_to_rgb(hsl: (f32, f32, f32)) -> (u8, u8, u8) {
    let h = hsl.0 % 360.0;
    let s = hsl.1.clamp(0.0, 1.0);
    let l = hsl.2.clamp(0.0, 1.0);

    if s == 0.0 {
        let v = (l * 255.0).round() as u8;
        return (v, v, v);
    }

    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let r = hue_to_rgb(p, q, h / 360.0 + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h / 360.0);
    let b = hue_to_rgb(p, q, h / 360.0 - 1.0 / 3.0);

    (
        (r * 255.0).round().clamp(0.0, 255.0) as u8,
        (g * 255.0).round().clamp(0.0, 255.0) as u8,
        (b * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn hue_to_rgb(p: f32, q: f32, t: f32) -> f32 {
    let mut t = t;
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
    if t < 1.0 / 2.0 { return q; }
    if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    p
}

fn ensure_contrast_on_light(rgb: (u8, u8, u8)) -> (u8, u8, u8) {
    let (h, s, l) = rgb_to_hsl(rgb);
    if l > 0.55 {
        hsl_to_rgb((h, s, 0.45))
    } else {
        rgb
    }
}

fn parse_hex(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return (0, 0, 0);
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    (r, g, b)
}

pub fn detect_system_palette() -> SystemColors {
    let accent = detect_dms_accent().unwrap_or((128, 212, 215));
    let is_dark = detect_system_dark_mode();
    generate_system_palette(accent, is_dark)
}
