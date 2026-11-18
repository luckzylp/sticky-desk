use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};
use gpui_kit::{
    InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Styled as _, Window,
    div, prelude::FluentBuilder as _, px,
};

fn layout_string() -> Option<String> {
    if cfg!(target_os = "linux") {
        if let Ok(out) = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.wm.preferences", "button-layout"])
            .output()
        {
            return Some(
                String::from_utf8_lossy(&out.stdout)
                    .trim()
                    .trim_matches('\'')
                    .to_string(),
            );
        }
    }
    None
}

/// Control ordering used by the desktop: 0 = close, 1 = minimize, 2 = maximize.
pub fn control_order() -> Vec<u8> {
    use std::sync::OnceLock;
    static ORDER: OnceLock<Vec<u8>> = OnceLock::new();
    ORDER
        .get_or_init(|| {
            if cfg!(target_os = "macos") {
                return vec![0, 1, 2];
            }
            if let Some(layout) = layout_string() {
                let spec = if controls_on_left() {
                    layout.split(':').next().unwrap_or("")
                } else {
                    layout.split(':').nth(1).unwrap_or("")
                };
                let mut order = Vec::new();
                for part in spec.split(',') {
                    match part.trim() {
                        "close" => order.push(0),
                        "minimize" => order.push(1),
                        "maximize" => order.push(2),
                        _ => {}
                    }
                }
                if !order.is_empty() {
                    return order;
                }
            }
            if cfg!(target_os = "windows") {
                vec![1, 2, 0]
            } else {
                vec![1, 2, 0]
            }
        })
        .clone()
}

pub fn controls_on_left() -> bool {
    use std::sync::OnceLock;
    static LEFT: OnceLock<bool> = OnceLock::new();
    *LEFT.get_or_init(|| {
        if cfg!(target_os = "macos") {
            return true;
        }
        if cfg!(target_os = "linux") {
            if let Ok(out) = std::process::Command::new("gsettings")
                .args(["get", "org.gnome.desktop.wm.preferences", "button-layout"])
                .output()
            {
                let text = String::from_utf8_lossy(&out.stdout);
                if let Some((left, _right)) = text.split_once(':') {
                    if left.contains("close")
                        || left.contains("minimize")
                        || left.contains("maximize")
                    {
                        return true;
                    }
                }
            }
            if let Ok(dest) = std::env::var("XDG_CURRENT_DESKTOP") {
                let dest = dest.to_ascii_lowercase();
                if dest.contains("kde") || dest.contains("plasma") {
                    if let Ok(cfg) = std::fs::read_to_string(
                        dirs::config_dir()
                            .unwrap_or_else(|| std::path::PathBuf::from("."))
                            .join("kdeglobals"),
                    ) {
                        if cfg.contains("ButtonsOnLeft=close")
                            || cfg.contains("ButtonsOnLeft=minimize")
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    })
}

/// A Root plugin installed after gpui-component's WindowState so the root
/// surface itself is transparent; the rounded content frame then defines the
/// visible window shape.
pub struct TransparentRoot;

impl gpui_kit::Render for TransparentRoot {
    fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
        div()
    }
}

impl gpui_kit::base::RootPlugin for TransparentRoot {
    fn style(
        &self,
        surface: &mut gpui_kit::Stateful<gpui_kit::Div>,
        _window: &mut Window,
        _cx: &mut gpui_kit::App,
    ) {
        use gpui_kit::Refineable as _;
        surface
            .style()
            .refine(&gpui_kit::StyleRefinement::default().bg(gpui_kit::transparent_black()));
    }
}

pub fn install_transparent_root(cx: &mut gpui_kit::App) {
    gpui_kit::base::Root::register_plugin::<TransparentRoot>(cx, |_, _| TransparentRoot);
}

pub const WINDOW_RADIUS: f32 = 12.0;

pub fn apply_window_theme(cx: &mut gpui_kit::App) {
    gpui_kit::component::Theme::update(cx, |theme| {
        theme.window_radius = px(WINDOW_RADIUS);
        theme.window_border.a = 0.0;
    });
}

fn controls() -> impl IntoElement {
    let mut items = Vec::new();
    for kind in control_order() {
        let button = match kind {
            0 => Button::new("chrome-close")
                .ghost()
                .small()
                .label("✕")
                .on_click(|_, window, _| window.remove_window()),
            1 => Button::new("chrome-min")
                .ghost()
                .small()
                .icon(IconName::Minus)
                .on_click(|_, window, _| window.minimize_window()),
            _ => Button::new("chrome-max")
                .ghost()
                .small()
                .icon(IconName::Maximize)
                .on_click(|_, window, _| window.zoom_window()),
        };
        items.push(button);
    }
    h_flex().gap_0p5().items_center().children(items)
}

pub fn title_bar(
    title: impl Into<String>,
    _window: &mut Window,
    cx: &gpui_kit::App,
) -> impl IntoElement {
    let title = title.into();
    let drag = div()
        .id("chrome-title-drag")
        .flex()
        .flex_1()
        .h_full()
        .items_center()
        .px_2()
        .text_size(px(13.))
        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
        .text_color(cx.theme().foreground)
        .on_mouse_down(MouseButton::Left, |event, window, _| {
            if event.click_count == 1 {
                window.start_window_move();
            }
        })
        .child(title);
    h_flex()
        .h(px(38.))
        .w_full()
        .items_center()
        .bg(cx.theme().background)
        .rounded_tl(px(WINDOW_RADIUS))
        .rounded_tr(px(WINDOW_RADIUS))
        .border_b_1()
        .border_color(cx.theme().border)
        .when(controls_on_left(), |b| {
            b.child(div().pl_2().child(controls()))
        })
        .child(
            Icon::new(IconName::NotebookPen)
                .small()
                .ml_2()
                .text_color(cx.theme().primary),
        )
        .child(drag)
        .when(!controls_on_left(), |b| {
            b.child(div().pr_2().child(controls()))
        })
}

pub fn content_frame(content: impl IntoElement, cx: &gpui_kit::App) -> impl IntoElement {
    v_flex()
        .size_full()
        .rounded(px(WINDOW_RADIUS))
        .overflow_hidden()
        .bg(cx.theme().background)
        .child(content)
}
