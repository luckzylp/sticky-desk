use super::model::AppModel;
use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, v_flex};
use gpui_kit::{
    Context, Entity, IntoElement, ParentElement as _, Render, Styled as _, Window, div, px,
};

pub struct AboutView {
    model: Entity<AppModel>,
}

impl AboutView {
    pub fn new(model: Entity<AppModel>, window: &mut Window, _cx: &mut Context<Self>) -> Self {
        window.set_client_corner_radii(gpui_kit::Corners::all(px(12.)));
        Self { model }
    }
}

impl Render for AboutView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = &self.model.read(cx).translator;
        super::chrome::content_frame(
            v_flex()
                .size_full()
                .text_color(cx.theme().foreground)
                .child(super::chrome::title_bar("About StickyDesk", window, cx))
                .child(
                    v_flex()
                        .p_6()
                        .gap_3()
                        .items_center()
                        .child(Icon::new(IconName::NotebookPen).large())
                        .child(
                            div()
                                .text_size(px(22.))
                                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                .child("StickyDesk"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .child(format!("Version {}", env!("CARGO_PKG_VERSION"))),
                        )
                        .child(
                            div()
                                .text_sm()
                                .child("Sticky notes for your Linux desktop · Rust + GPUI Kit"),
                        )
                        .child(div().text_xs().child(t.t("General", "AllRightsReserved"))),
                ),
            cx,
        )
    }
}
