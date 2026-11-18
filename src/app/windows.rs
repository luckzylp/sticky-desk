use super::model::AppModel;
use super::note_view::NoteView;
use crate::model::now_ole;
use gpui_kit::{
    App, AppContext as _, Bounds, Entity, Size, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowKind, WindowOptions, point, px, size,
};
use std::sync::Arc;

fn app_icon() -> Option<Arc<image::RgbaImage>> {
    let bytes = include_bytes!("../../assets/linux/sticky-desk-64.png");
    image::load_from_memory(bytes)
        .ok()
        .map(|img| Arc::new(img.to_rgba8()))
}

fn note_options(
    width: i32,
    height: i32,
    left: i32,
    top: i32,
    title: &str,
    aot: bool,
) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(left as f32), px(top as f32)),
            size: size(px(width as f32), px(height as f32)),
        })),
        titlebar: Some(gpui_kit::TitlebarOptions {
            title: Some(title.into()),
            appears_transparent: true,
            traffic_light_position: None,
        }),
        kind: if aot {
            WindowKind::Normal
        } else {
            WindowKind::Normal
        },
        is_movable: true,
        app_owns_titlebar_drag: true,
        is_resizable: true,
        is_minimizable: true,
        window_background: WindowBackgroundAppearance::Transparent,
        app_id: Some("sticky-desk".into()),
        icon: app_icon(),
        window_min_size: Some(size(px(240.), px(240.))),
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    }
}

pub fn open_note_window(cx: &mut App, model: Entity<AppModel>, id: i64) {
    let data = {
        let m = model.read(cx);
        m.note(id)
            .map(|n| (n.width, n.height, n.left, n.top, n.title.clone(), n.aot))
    };
    let Some((width, height, left, top, title, aot)) = data else {
        return;
    };
    if let Some(handle) = model.read(cx).note_windows.get(&id).copied() {
        if handle
            .update(cx, |_, window, _| {
                window.activate_window();
            })
            .is_ok()
        {
            return;
        }
    }
    let options = note_options(width, height, left, top, &title, aot);
    let model2 = model.clone();
    if let Ok((handle, _)) = gpui_kit::open_window(options, cx, move |window, cx| {
        cx.new(|cx| NoteView::new(id, model2.clone(), window, cx))
    }) {
        model.update(cx, |m, _| {
            m.note_windows.insert(id, handle);
            if let Some(n) = m.note_mut(id) {
                n.visible = true;
            }
        });
    }
}

pub fn resize_note_window(cx: &mut App, model: Entity<AppModel>, id: i64) {
    let data = model.read(cx).note(id).map(|n| (n.width, n.height));
    let Some((w, h)) = data else { return };
    let handle = model.read(cx).note_windows.get(&id).copied();
    if let Some(handle) = handle {
        let _ = handle.update(cx, |_, window, _| {
            window.resize(Size {
                width: px(w as f32),
                height: px(h as f32),
            });
        });
    }
}

pub fn open_explorer_window(cx: &mut App, model: Entity<AppModel>) {
    if let Some(handle) = model.read(cx).explorer_window {
        if handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
        {
            return;
        }
    }
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(240.), px(160.)),
            size: size(px(980.), px(680.)),
        })),
        titlebar: Some(gpui_kit::TitlebarOptions {
            title: Some("StickyDesk – Explorer".into()),
            appears_transparent: true,
            traffic_light_position: None,
        }),
        app_owns_titlebar_drag: true,
        window_min_size: Some(size(px(520.), px(420.))),
        app_id: Some("sticky-desk".into()),
        icon: app_icon(),
        window_background: WindowBackgroundAppearance::Transparent,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    };
    let model2 = model.clone();
    if let Ok((handle, _)) = gpui_kit::open_window(options, cx, move |window, cx| {
        cx.new(|cx| super::explorer::ExplorerView::new(model2.clone(), window, cx))
    }) {
        model.update(cx, |m, _| m.explorer_window = Some(handle));
    }
}

pub fn open_settings_window(cx: &mut App, model: Entity<AppModel>) {
    if let Some(handle) = model.read(cx).settings_window {
        if handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
        {
            return;
        }
    }
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(320.), px(180.)),
            size: size(px(760.), px(760.)),
        })),
        titlebar: Some(gpui_kit::TitlebarOptions {
            title: Some("StickyDesk – Settings".into()),
            appears_transparent: true,
            traffic_light_position: None,
        }),
        app_owns_titlebar_drag: true,
        window_min_size: Some(size(px(520.), px(520.))),
        app_id: Some("sticky-desk".into()),
        icon: app_icon(),
        window_background: WindowBackgroundAppearance::Transparent,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    };
    let model2 = model.clone();
    if let Ok((handle, _)) = gpui_kit::open_window(options, cx, move |window, cx| {
        cx.new(|cx| super::settings_view::SettingsView::new(model2.clone(), window, cx))
    }) {
        model.update(cx, |m, _| m.settings_window = Some(handle));
    }
}

pub fn open_about_window(cx: &mut App, model: Entity<AppModel>) {
    if let Some(handle) = model.read(cx).about_window {
        if handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
        {
            return;
        }
    }
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::centered(size(px(420.), px(320.)), cx)),
        titlebar: Some(gpui_kit::TitlebarOptions {
            title: Some("About StickyDesk".into()),
            appears_transparent: true,
            traffic_light_position: None,
        }),
        is_resizable: false,
        app_owns_titlebar_drag: true,
        app_id: Some("sticky-desk".into()),
        icon: app_icon(),
        window_background: WindowBackgroundAppearance::Transparent,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    };
    let model2 = model.clone();
    if let Ok((handle, _)) = gpui_kit::open_window(options, cx, move |window, cx| {
        cx.new(|cx| super::about::AboutView::new(model2.clone(), window, cx))
    }) {
        model.update(cx, |m, _| m.about_window = Some(handle));
    }
}

pub fn open_alarm_window(cx: &mut App, model: Entity<AppModel>, note_id: i64) {
    if let Some(handle) = model.read(cx).alarm_window {
        if handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
        {
            return;
        }
    }
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::centered(size(px(460.), px(300.)), cx)),
        titlebar: Some(gpui_kit::TitlebarOptions {
            title: Some("Alarm – StickyDesk".into()),
            appears_transparent: true,
            traffic_light_position: None,
        }),
        is_resizable: false,
        app_owns_titlebar_drag: true,
        app_id: Some("sticky-desk".into()),
        icon: app_icon(),
        window_background: WindowBackgroundAppearance::Transparent,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    };
    let model2 = model.clone();
    if let Ok((handle, _)) = gpui_kit::open_window(options, cx, move |window, cx| {
        cx.new(|cx| super::alarm::AlarmView::new(note_id, model2.clone(), window, cx))
    }) {
        model.update(cx, |m, _| {
            m.alarm_window = Some(handle);
            m.mutate_note(note_id, |n| {
                n.alarm_current = now_ole();
                n.alarm = if n.alarm_period > 0 {
                    super::alarm::next_occurrence(
                        n.alarm_period,
                        n.alarm_day,
                        chrono::Local::now().naive_local(),
                    )
                } else {
                    0.0
                };
            });
        });
    }
}

pub fn new_note(
    cx: &mut App,
    model: Entity<AppModel>,
    text: Option<String>,
    notebook: Option<String>,
) -> i64 {
    let id = model.update(cx, |m, _| m.create_note(text, notebook));
    open_note_window(cx, model, id);
    id
}

pub fn show_all_notes(cx: &mut App, model: Entity<AppModel>) {
    let ids: Vec<i64> = {
        let m = model.read(cx);
        m.notes
            .iter()
            .filter(|n| !n.is_trashed())
            .map(|n| n.id)
            .collect()
    };
    for id in ids {
        open_note_window(cx, model.clone(), id);
    }
}

pub fn hide_all_notes(cx: &mut App, model: Entity<AppModel>) {
    let handles: Vec<_> = model.read(cx).note_windows.values().copied().collect();
    for handle in handles {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
    model.update(cx, |m, cx| {
        for n in &mut m.notes {
            n.visible = false;
        }
        m.note_windows.clear();
        cx.notify();
    });
}
