use std::cell::RefCell;
use std::f32::consts::TAU;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

use gpui_kit::component::{
    ActiveTheme as _, WindowExt as _, h_flex,
    input::{Input, InputEvent, InputState},
    slider::{Slider, SliderEvent, SliderState, SliderValue},
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Bounds, Context, Entity, ImageSource, InteractiveElement as _,
    IntoElement, MouseButton, ParentElement as _, Pixels, Render, RenderImage, Styled as _,
    Subscription, Window, canvas, div, img, px,
};
use smallvec::SmallVec;

/// On-screen diameter of the wheel.
const WHEEL_SIZE: f32 = 220.;
const WHEEL_TEXTURE: u32 = 256;

pub type ColorChange = Rc<dyn Fn(u32, &mut App)>;

/// Open a circular HSV color wheel. `on_change` is invoked live while the
/// user drags the wheel or moves the brightness slider.
pub fn open_color_wheel(
    window: &mut Window,
    cx: &mut App,
    initial: u32,
    on_change: impl Fn(u32, &mut App) + 'static,
) {
    let view = cx.new(|cx| ColorWheelView::new(window, cx, initial, Rc::new(on_change)));
    window.open_dialog(cx, move |dialog, _window, _cx| {
        dialog.title("Color").w(px(300.)).child(view.clone())
    });
}

pub struct ColorWheelView {
    hue: f32,
    sat: f32,
    val: f32,
    preview: u32,
    hex_input: Entity<InputState>,
    brightness: Entity<SliderState>,
    bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    syncing: bool,
    on_change: ColorChange,
    _subs: Vec<Subscription>,
}

impl ColorWheelView {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        initial: u32,
        on_change: ColorChange,
    ) -> Self {
        let (hue, sat, val) = rgb_to_hsv(initial);
        let hex_input = cx.new(|cx| InputState::new(window, cx).placeholder("#RRGGBB"));
        hex_input.update(cx, |state, cx| {
            state.set_value(hex_string(initial), window, cx);
        });
        let brightness = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(1.)
                .step(0.01)
                .default_value(val)
        });
        let bounds: Rc<RefCell<Option<Bounds<Pixels>>>> = Rc::new(RefCell::new(None));

        let hex_sub = cx.subscribe_in(
            &hex_input,
            window,
            |this, input, event: &InputEvent, window, cx| {
                if this.syncing {
                    return;
                }
                if matches!(event, InputEvent::Change | InputEvent::PressEnter { .. }) {
                    let text = input.read(cx).value().to_string();
                    if let Some(rgb) = parse_hex(&text) {
                        let (h, s, v) = rgb_to_hsv(rgb);
                        this.hue = h;
                        this.sat = s;
                        this.val = v;
                        let slider = this.brightness.clone();
                        slider.update(cx, |state, cx| state.set_value(v, window, cx));
                        this.emit(window, cx);
                    }
                }
            },
        );

        let slider_sub = cx.subscribe_in(
            &brightness,
            window,
            |this, _, event: &SliderEvent, window, cx| {
                if this.syncing {
                    return;
                }
                let value = match event {
                    SliderEvent::Change(value) | SliderEvent::Release(value) => *value,
                };
                if let SliderValue::Single(value) = value {
                    this.val = value.clamp(0., 1.);
                    this.emit(window, cx);
                }
            },
        );

        Self {
            hue,
            sat,
            val,
            preview: initial,
            hex_input,
            brightness,
            bounds,
            syncing: false,
            on_change,
            _subs: vec![hex_sub, slider_sub],
        }
    }

    fn emit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (r, g, b) = hsv_to_rgb(self.hue, self.sat, self.val);
        let rgb = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
        self.preview = rgb;
        self.syncing = true;
        let input = self.hex_input.clone();
        input.update(cx, |state, cx| {
            state.set_value(hex_string(rgb), window, cx);
        });
        self.syncing = false;
        (self.on_change)(rgb, cx);
    }

    fn pick_at(
        &mut self,
        position: gpui_kit::Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(bounds) = *self.bounds.borrow() else {
            return;
        };
        let radius = WHEEL_SIZE / 2.;
        let dx = (position.x - bounds.origin.x).as_f32() - radius;
        let dy = (position.y - bounds.origin.y).as_f32() - radius;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > radius + 1. {
            return;
        }
        self.hue = (dy.atan2(dx) / TAU + 1.0) % 1.0;
        self.sat = (dist / radius).clamp(0., 1.);
        self.emit(window, cx);
    }
}

impl Render for ColorWheelView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let image = wheel_image();
        let bounds_for_paint = self.bounds.clone();
        let marker_x = WHEEL_SIZE / 2. + (self.hue * TAU).cos() * self.sat * (WHEEL_SIZE / 2.);
        let marker_y = WHEEL_SIZE / 2. + (self.hue * TAU).sin() * self.sat * (WHEEL_SIZE / 2.);
        let shade = crate::theme::hsla(0x000000, (1.0 - self.val) * 0.9);

        v_flex()
            .gap_3()
            .items_center()
            .child(
                div()
                    .relative()
                    .w(px(WHEEL_SIZE))
                    .h(px(WHEEL_SIZE))
                    .rounded_full()
                    .child(img(ImageSource::Render(image)).w_full().h_full())
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .rounded_full()
                            .bg(shade),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(marker_x - 6.))
                            .top(px(marker_y - 6.))
                            .w(px(12.))
                            .h(px(12.))
                            .rounded_full()
                            .border_2()
                            .border_color(cx.theme().background)
                            .bg(crate::theme::rgba(self.preview)),
                    )
                    .child(
                        canvas(
                            move |bounds, _, _| {
                                *bounds_for_paint.borrow_mut() = Some(bounds);
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &gpui_kit::MouseDownEvent, window, cx| {
                            this.pick_at(event.position, window, cx);
                        }),
                    )
                    .on_mouse_move(cx.listener(
                        |this, event: &gpui_kit::MouseMoveEvent, window, cx| {
                            if event.dragging() {
                                this.pick_at(event.position, window, cx);
                            }
                        },
                    )),
            )
            .child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .items_center()
                    .child(div().text_xs().child("Brightness"))
                    .child(Slider::new(&self.brightness).flex_1())
                    .child(
                        div()
                            .w(px(36.))
                            .text_xs()
                            .child(format!("{:.0}%", self.val * 100.)),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .w(px(22.))
                            .h(px(22.))
                            .rounded(px(4.))
                            .bg(crate::theme::rgba(self.preview)),
                    )
                    .child(Input::new(&self.hex_input).flex_1()),
            )
    }
}

fn wheel_image() -> Arc<RenderImage> {
    static IMAGE: OnceLock<Arc<RenderImage>> = OnceLock::new();
    IMAGE
        .get_or_init(|| {
            let mut buffer = image::RgbaImage::new(WHEEL_TEXTURE, WHEEL_TEXTURE);
            let radius = (WHEEL_TEXTURE as f32 - 1.) / 2.;
            let center = radius;
            for y in 0..WHEEL_TEXTURE {
                for x in 0..WHEEL_TEXTURE {
                    let dx = x as f32 - center;
                    let dy = y as f32 - center;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist > radius + 1. {
                        continue;
                    }
                    let hue = (dy.atan2(dx) / TAU + 1.0) % 1.0;
                    let sat = (dist / radius).clamp(0., 1.);
                    let (r, g, b) = hsv_to_rgb(hue, sat, 1.0);
                    let alpha = ((radius + 1. - dist).clamp(0., 1.) * 255.) as u8;
                    buffer.put_pixel(x, y, image::Rgba([r, g, b, alpha]));
                }
            }
            let frame = image::Frame::new(buffer);
            Arc::new(RenderImage::new(SmallVec::from_elem(frame, 1)))
        })
        .clone()
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let h = (h.fract() + 1.0) % 1.0;
    let sector = (h * 6.).floor();
    let f = h * 6. - sector;
    let p = v * (1. - s);
    let q = v * (1. - f * s);
    let t = v * (1. - (1. - f) * s);
    let (r, g, b) = match sector as i32 % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    (
        (r * 255.).round() as u8,
        (g * 255.).round() as u8,
        (b * 255.).round() as u8,
    )
}

fn rgb_to_hsv(color: u32) -> (f32, f32, f32) {
    let r = ((color >> 16) & 0xFF) as f32 / 255.;
    let g = ((color >> 8) & 0xFF) as f32 / 255.;
    let b = (color & 0xFF) as f32 / 255.;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta == 0. {
        0.
    } else if max == r {
        ((g - b) / delta).rem_euclid(6.) / 6.
    } else if max == g {
        ((b - r) / delta + 2.) / 6.
    } else {
        ((r - g) / delta + 4.) / 6.
    };
    let sat = if max == 0. { 0. } else { delta / max };
    (hue, sat, max)
}

fn parse_hex(value: &str) -> Option<u32> {
    let value = value.trim().trim_start_matches('#');
    if value.len() != 6 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(value, 16).ok()
}

pub fn hex_string(color: u32) -> String {
    format!("#{color:06X}")
}
