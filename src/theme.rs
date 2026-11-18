use gpui_kit::{Hsla, Rgba};

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
