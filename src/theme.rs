use gpui_kit::{Hsla, Rgba};

#[derive(Clone, Debug)]
pub struct ColorItem {
    pub base: u32,
}

pub fn standard_palette() -> Vec<ColorItem> {
    let raw = include_str!("../assets/colors.json");
    let v: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(raw).unwrap_or_default();
    v.keys()
        .filter_map(|k| u32::from_str_radix(k, 16).ok())
        .map(|base| ColorItem { base })
        .collect()
}

pub fn classic_colors() -> Vec<u32> {
    vec![
        0xFFFF99, 0xFFCC99, 0xFFCCFF, 0xCCFFCC, 0xCCFFFF, 0xCCCCFF, 0xE0E0E0, 0xFF9999, 0xFFCC66,
        0xFF99FF, 0x99FF99, 0x99FFFF, 0x9999FF, 0xC0C0C0, 0xFFE699, 0xFFB366, 0xFF66FF, 0x66FF66,
        0x66FFFF, 0x6666FF, 0xFFFFFF, 0xFFFFCC, 0xFFCCCC, 0xFFCCFF, 0xCCFFCC, 0xCCFFFF, 0xCCCCFF,
        0xF2F2F2, 0xE6E6E6, 0xD9D9D9, 0xBFBFBF, 0xA6A6A6,
    ]
}

pub fn pastel_palette() -> Vec<u32> {
    vec![
        0xFFF1B8, 0xFFD9B8, 0xFFC9C9, 0xFFC9E5, 0xE5C9FF, 0xC9D9FF, 0xC9F2FF, 0xC9FFE5, 0xD9FFC9,
        0xEFFFC9, 0xF2E6FF, 0xFFE6F2, 0xE6F2FF, 0xE6FFF2, 0xF2FFE6, 0xFFF2E6,
    ]
    .into_iter()
    .collect()
}

pub fn note_color_dark(color: u32) -> u32 {
    brightness(color, -88)
}

pub fn brightness(color: u32, percent: i32) -> u32 {
    let r = ((color >> 16) & 0xFF) as i32;
    let g = ((color >> 8) & 0xFF) as i32;
    let b = (color & 0xFF) as i32;
    let delta = (255 * percent) / 100;
    let clamp = |v: i32| v.clamp(0, 255) as u32;
    (clamp(r + delta) << 16) | (clamp(g + delta) << 8) | clamp(b + delta)
}

pub fn auto_font_color(bg: u32) -> u32 {
    let r = ((bg >> 16) & 0xFF) as f32;
    let g = ((bg >> 8) & 0xFF) as f32;
    let b = (bg & 0xFF) as f32;
    let lum = 0.299 * r + 0.587 * g + 0.114 * b;
    if lum < 140.0 { 0xEEEEEE } else { 0x1A1A1A }
}

pub fn rgba(color: u32) -> Rgba {
    Rgba {
        r: ((color >> 16) & 0xFF) as f32 / 255.0,
        g: ((color >> 8) & 0xFF) as f32 / 255.0,
        b: (color & 0xFF) as f32 / 255.0,
        a: 1.0,
    }
}

pub fn hsla(color: u32, alpha: f32) -> Hsla {
    let mut c: Hsla = rgba(color).into();
    c.a = alpha;
    c
}

/// Standard 256-colour palette: 16 system colours, 6x6x6 colour cube, 24 greys.
pub fn palette_256() -> Vec<u32> {
    let mut out = Vec::with_capacity(256);
    out.extend_from_slice(&classic_colors()[..16]);
    let levels = [0u32, 51, 102, 153, 204, 255];
    for r in levels {
        for g in levels {
            for b in levels {
                out.push((r << 16) | (g << 8) | b);
            }
        }
    }
    for i in 0..24u32 {
        let v = 8 + i * 10;
        out.push((v << 16) | (v << 8) | v);
    }
    out.truncate(256);
    out
}
