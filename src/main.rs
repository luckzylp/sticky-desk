mod app;
mod i18n;
mod model;
mod store;
mod theme;
mod util;

use app::model::AppModel;
use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;
use std::borrow::Cow;

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .with_quit_mode(gpui_kit::QuitMode::Explicit)
        .run(|cx| {
            gpui_kit::init(cx);
            app::chrome::install_transparent_root(cx);
            cx.bind_keys([
                gpui_kit::KeyBinding::new("alt-n", app::actions::NewNote, None),
                gpui_kit::KeyBinding::new("alt-v", app::actions::NewNoteFromClipboard, None),
                gpui_kit::KeyBinding::new("alt-s", app::actions::ShowAllNotes, None),
                gpui_kit::KeyBinding::new("alt-h", app::actions::HideAllNotes, None),
                gpui_kit::KeyBinding::new("alt-g", app::actions::GoNextNote, None),
                gpui_kit::KeyBinding::new("alt-x", app::actions::OpenExplorer, None),
            ]);
            let (tray_tx, tray_rx) = std::sync::mpsc::channel();
            if !app::single_instance::start(tray_tx.clone()) {
                std::process::exit(0);
            }
            let fonts: Vec<Cow<'static, [u8]>> =
                vec![Cow::Borrowed(include_bytes!("../assets/fonts/fluent.ttf"))];
            let _ = cx.text_system().add_fonts(fonts);

            let model = cx.new(|_| AppModel::new());

            // The app keeps running without windows when "close to tray" is on.
            // Otherwise closing the last window quits, matching the previous
            // LastWindowClosed behaviour.
            let close_model = model.clone();
            let close_sub = cx.on_window_closed(move |cx, _| {
                if cx.windows().is_empty() && !close_model.read(cx).settings.close_to_tray {
                    cx.quit();
                }
            });
            std::mem::forget(close_sub);

            let dark = {
                let m = model.read(cx);
                match m.settings.app_mode {
                    crate::model::AppMode::Dark => true,
                    crate::model::AppMode::Light => false,
                    crate::model::AppMode::System => system_prefers_dark(),
                }
            };
            Theme::change(
                if dark {
                    ThemeMode::Dark
                } else {
                    ThemeMode::Light
                },
                None,
                cx,
            );

            let auto_start = model.read(cx).settings.auto_start;
            crate::util::set_autostart(auto_start);
            let show_notes = model.read(cx).settings.show_all_notes_on_startup;
            let first_run = model.read(cx).first_run;
            let ids: Vec<i64> = model
                .read(cx)
                .notes
                .iter()
                .filter(|n| !n.is_trashed())
                .map(|n| n.id)
                .collect();
            if show_notes || first_run {
                for id in ids {
                    app::windows::open_note_window(cx, model.clone(), id);
                }
            }
            app::windows::open_explorer_window(cx, model.clone());

            // Linux tray icon / StatusNotifierItem.
            let notebooks = model.read(cx).notebooks_with_default();
            app::tray::start(tray_tx, notebooks);

            // Background dispatcher: tray commands and alarm checks.
            let loop_model = model.clone();
            let task = cx.spawn(async move |cx| {
                loop {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(400))
                        .await;
                    while let Ok(cmd) = tray_rx.try_recv() {
                        cx.update(|cx| handle_tray_cmd(cx, loop_model.clone(), cmd));
                    }
                    cx.update(|cx| {
                        let due = loop_model.read(cx).alarms_due();
                        for id in due {
                            app::windows::open_alarm_window(cx, loop_model.clone(), id);
                        }
                    });
                }
            });
            std::mem::forget(task);
            let _ = model;
        });
}

fn handle_tray_cmd(cx: &mut App, model: gpui_kit::Entity<AppModel>, cmd: app::tray::TrayCommand) {
    use app::tray::TrayCommand;
    match cmd {
        TrayCommand::NewNote => {
            app::windows::new_note(cx, model, None, None);
        }
        TrayCommand::NewNoteFromClipboard => {
            let text = cx.read_from_clipboard().and_then(|item| {
                item.entries.iter().find_map(|e| match e {
                    gpui_kit::ClipboardEntry::String(s) => Some(s.text.clone()),
                    _ => None,
                })
            });
            app::windows::new_note(cx, model, text, None);
        }
        TrayCommand::ShowAllNotes => app::windows::show_all_notes(cx, model),
        TrayCommand::HideAllNotes => app::windows::hide_all_notes(cx, model),
        TrayCommand::Notebook(name) => {
            model.update(cx, |m, _| {
                m.active_notebook = name.clone();
                m.settings.active_notebook = name.clone();
                m.save_settings();
            });
            app::windows::new_note(cx, model, None, None);
        }
        TrayCommand::NewNotebook => {
            app::windows::open_explorer_window(cx, model);
        }
        TrayCommand::Explorer => app::windows::open_explorer_window(cx, model),
        TrayCommand::Settings => app::windows::open_settings_window(cx, model),
        TrayCommand::About => app::windows::open_about_window(cx, model),
        TrayCommand::Exit => cx.quit(),
    }
}

pub fn system_prefers_dark() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(out) = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
        {
            if String::from_utf8_lossy(&out.stdout).contains("dark") {
                return true;
            }
        }
    }
    false
}
