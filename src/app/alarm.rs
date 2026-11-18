use super::model::AppModel;
use crate::model::*;
use chrono::Timelike;
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};
use gpui_kit::{
    Context, Entity, IntoElement, ParentElement as _, Render, Styled as _, Subscription, Window,
    div, px,
};

pub struct AlarmView {
    model: Entity<AppModel>,
    note_id: i64,
    _sub: Subscription,
}

impl AlarmView {
    pub fn new(
        note_id: i64,
        model: Entity<AppModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let sound = model
            .read(cx)
            .note(note_id)
            .map(|n| n.alarm_sound.clone())
            .unwrap_or_default();
        if !sound.is_empty() {
            crate::util::play_sound(&sound);
        }
        window.set_client_corner_radii(gpui_kit::Corners::all(px(12.)));
        let sub = cx.observe(&model, |_, _, cx| cx.notify());
        Self {
            model,
            note_id,
            _sub: sub,
        }
    }

    fn snooze(&mut self, minutes: i64, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.note_id;
        self.model.update(cx, |m, cx| {
            m.mutate_note(id, |n| {
                let target =
                    chrono::Local::now().naive_local() + chrono::Duration::minutes(minutes);
                n.alarm = datetime_to_ole(target);
                n.alarm_current = 0.0;
                n.alarm_snooze = minutes as i32;
            });
            cx.notify();
        });
        window.remove_window();
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.note_id;
        self.model.update(cx, |m, cx| {
            m.mutate_note(id, |n| {
                n.alarm_current = 0.0;
                n.alarm_snooze = 0;
                if n.alarm_period > 0 {
                    n.alarm = next_occurrence(
                        n.alarm_period,
                        n.alarm_day,
                        chrono::Local::now().naive_local(),
                    );
                } else {
                    n.alarm = 0.0;
                }
            });
            cx.notify();
        });
        window.remove_window();
    }
}

impl Render for AlarmView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (title, text) = {
            let m = self.model.read(cx);
            m.note(self.note_id)
                .map(|n| (n.title.clone(), truncate(&n.doc.text, 260)))
                .unwrap_or_else(|| ("Alarm".into(), String::new()))
        };
        super::chrome::content_frame(
            v_flex()
                .size_full()
                .bg(cx.theme().background)
                .text_color(cx.theme().foreground)
                .child(super::chrome::title_bar("Alarm – StickyDesk", window, cx))
                .child(
                    v_flex()
                        .flex_1()
                        .p_6()
                        .gap_3()
                        .items_center()
                        .justify_center()
                        .child(
                            Icon::new(IconName::AlarmClock)
                                .large()
                                .text_color(cx.theme().primary),
                        )
                        .child(
                            div()
                                .text_size(px(22.))
                                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(text),
                        )
                        .child(div().text_sm().child("Alarm"))
                        .child(
                            h_flex()
                                .gap_2()
                                .child(
                                    Button::new("snooze-5")
                                        .small()
                                        .label("Snooze 5 min")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.snooze(5, window, cx)
                                        })),
                                )
                                .child(
                                    Button::new("snooze-10")
                                        .small()
                                        .label("Snooze 10 min")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.snooze(10, window, cx)
                                        })),
                                )
                                .child(
                                    Button::new("snooze-30")
                                        .small()
                                        .label("Snooze 30 min")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.snooze(30, window, cx)
                                        })),
                                )
                                .child(Button::new("dismiss").primary().label("Dismiss").on_click(
                                    cx.listener(|this, _, window, cx| this.dismiss(window, cx)),
                                )),
                        ),
                ),
            cx,
        )
    }
}

fn truncate(s: &str, n: usize) -> String {
    let clean = s.replace('\n', " ");
    let mut out: String = clean.chars().take(n).collect();
    if clean.chars().count() > n {
        out.push('…');
    }
    out
}

pub fn next_occurrence(period: i32, day: i32, from: chrono::NaiveDateTime) -> f64 {
    use chrono::{Datelike, Duration};
    let next = match period {
        1 => from + Duration::days(1),
        2 => from + Duration::days(7),
        3 => {
            let mut y = from.year();
            let mut m = from.month();
            m += 1;
            if m > 12 {
                m = 1;
                y += 1;
            }
            let d = day.clamp(1, 28) as u32;
            chrono::NaiveDate::from_ymd_opt(y, m, d)
                .and_then(|d| d.and_hms_opt(from.time().hour(), from.time().minute(), 0))
                .unwrap_or(from + Duration::days(30))
        }
        4 => {
            let y = from.year() + 1;
            let d = day.clamp(1, 28) as u32;
            chrono::NaiveDate::from_ymd_opt(y, from.month(), d)
                .and_then(|d| d.and_hms_opt(from.time().hour(), from.time().minute(), 0))
                .unwrap_or(from + Duration::days(365))
        }
        _ => from,
    };
    datetime_to_ole(next)
}
