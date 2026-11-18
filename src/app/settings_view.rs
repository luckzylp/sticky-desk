use super::model::AppModel;
use super::windows;
use crate::i18n::language_name;
use crate::model::*;
use crate::theme;
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, Theme, ThemeMode, WindowExt as _,
    button::{Button, ButtonVariants},
    dialog::{DialogClose, DialogFooter},
    h_flex,
    input::{Input, InputState},
    switch::Switch,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, StatefulInteractiveElement as _, Styled as _, Subscription, Window,
    div, prelude::FluentBuilder as _, px,
};

pub struct SettingsView {
    model: Entity<AppModel>,
    path_input: Entity<InputState>,
    _subs: Vec<Subscription>,
}

impl SettingsView {
    pub fn new(model: Entity<AppModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.set_client_corner_radii(gpui_kit::Corners::all(px(12.)));
        let db_path = model.read(cx).settings.db_location.clone();
        let path_input = cx.new(|cx| InputState::new(window, cx).placeholder("Database folder"));
        path_input.update(cx, |state, cx| state.set_value(db_path, window, cx));
        let mut subs = Vec::new();
        subs.push(cx.observe(&model, |_, _, cx| cx.notify()));
        Self {
            model,
            path_input,
            _subs: subs,
        }
    }

    fn location_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let model = self.model.clone();
        let input = self.path_input.clone();
        let current = self.model.read(cx).settings.db_location.clone();
        input.update(cx, |state, cx| state.set_value(current, window, cx));
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let model = model.clone();
            let input = input.clone();
            dialog
                .title("Database location")
                .w(px(520.))
                .child(
                    div()
                        .text_sm()
                        .child("Choose the folder that contains Notes.db."),
                )
                .child(Input::new(&input))
                .footer(
                    DialogFooter::new()
                        .gap_2()
                        .child(DialogClose::new().child(Button::new("loc-cancel").label("Cancel")))
                        .child(DialogClose::new().child(
                            Button::new("loc-apply").primary().label("Apply").on_click({
                                let model = model.clone();
                                let input = input.clone();
                                move |_, window: &mut Window, cx: &mut App| {
                                    let folder = input.read(cx).value().trim().to_string();
                                    if folder.is_empty() {
                                        window.close_dialog(cx);
                                        return;
                                    }
                                    let path = std::path::PathBuf::from(&folder)
                                        .join(crate::store::DB_FILE);
                                    let _ = std::fs::create_dir_all(&folder);
                                    if !path.exists() {
                                        if let Some(store) = &model.read(cx).store {
                                            let _ = store.backup_to(&path);
                                        }
                                    }
                                    model.update(cx, |m, cx| {
                                        m.settings.db_location = folder;
                                        m.save_settings();
                                        m.reload_db(path);
                                        cx.notify();
                                    });
                                    window.close_dialog(cx);
                                }
                            }),
                        )),
                )
        });
    }

    fn add_sound_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let model = self.model.clone();
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("/path/to/sound.mp3"));
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let model = model.clone();
            let input = input.clone();
            dialog
                .title("Add Sound")
                .w(px(520.))
                .child(
                    div()
                        .text_sm()
                        .child("Choose an mp3, wav, ogg or flac file to add to the sound list."),
                )
                .child(Input::new(&input))
                .footer(
                    DialogFooter::new()
                        .gap_2()
                        .child(
                            DialogClose::new().child(Button::new("sound-cancel").label("Cancel")),
                        )
                        .child(DialogClose::new().child(
                            Button::new("sound-add").primary().label("Add").on_click(
                                move |_, window: &mut Window, cx: &mut App| {
                                    let path =
                                        std::path::PathBuf::from(input.read(cx).value().trim());
                                    match crate::util::install_sound(&path) {
                                        Ok(dest) => {
                                            model.update(cx, |m, cx| {
                                                m.settings.default_alarm_sound = dest;
                                                m.save_settings();
                                                cx.notify();
                                            });
                                        }
                                        Err(e) => {
                                            model.update(cx, |m, cx| {
                                                m.toast_error(format!("Could not add sound: {e}"));
                                                cx.notify();
                                            });
                                        }
                                    }
                                    window.close_dialog(cx);
                                },
                            ),
                        )),
                )
        });
    }

    fn backup_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let model = self.model.clone();
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Backup file path"));
        let default_path = dirs::document_dir()
            .unwrap_or_else(|| std::env::temp_dir())
            .join(format!(
                "Notes_{}.db",
                chrono::Local::now().format("%Y_%m_%d_%H_%M_%S")
            ))
            .to_string_lossy()
            .to_string();
        input.update(cx, |state, cx| state.set_value(default_path, window, cx));
        window.open_dialog(cx, move |dialog, _window, _cx| {
            dialog
                .title("Backup")
                .w(px(480.))
                .child(Input::new(&input))
                .footer(
                    DialogFooter::new()
                        .gap_2()
                        .child(
                            DialogClose::new().child(Button::new("backup-cancel").label("Cancel")),
                        )
                        .child(
                            DialogClose::new().child(
                                Button::new("backup-go")
                                    .primary()
                                    .label("Backup")
                                    .on_click({
                                        let model = model.clone();
                                        let input = input.clone();
                                        move |_, window: &mut Window, cx: &mut App| {
                                            let path = std::path::PathBuf::from(
                                                input.read(cx).value().to_string(),
                                            );
                                            let result = model
                                                .read(cx)
                                                .store
                                                .as_ref()
                                                .map(|s| s.backup_to(&path));
                                            if let Some(Err(e)) = result {
                                                model.update(cx, |m, cx| {
                                                    m.toast_error(format!("Backup failed: {e}"));
                                                    cx.notify();
                                                });
                                            }
                                            window.close_dialog(cx);
                                        }
                                    }),
                            ),
                        ),
                )
        });
    }

    fn restore_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let model = self.model.clone();
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Backup file path"));
        window.open_dialog(cx, move |dialog, _window, _cx| {
            dialog
                .title("Restore / Import")
                .w(px(480.))
                .child(div().text_sm().child("Existing notes will be replaced when restoring. Import merges the selected database."))
                .child(Input::new(&input))
                .footer(
                    DialogFooter::new()
                        .gap_2()
                        .child(DialogClose::new().child(Button::new("restore-cancel").label("Cancel")))
                        .child(DialogClose::new().child(Button::new("restore-go").danger().label("Restore").on_click({
                            let model = model.clone();
                            let input = input.clone();
                            move |_, window: &mut Window, cx: &mut App| {
                                let path = std::path::PathBuf::from(input.read(cx).value().to_string());
                                let result = model.update(cx, |m, _| m.restore_from(&path));
                                if let Err(e) = result {
                                    model.update(cx, |m, cx| {
                                        m.toast_error(format!("Restore failed: {e}"));
                                        cx.notify();
                                    });
                                }
                                window.close_dialog(cx);
                            }
                        })))
                        .child(DialogClose::new().child(Button::new("import-go").label("Import").on_click({
                            let model = model.clone();
                            let input = input.clone();
                            move |_, window: &mut Window, cx: &mut App| {
                                let path = std::path::PathBuf::from(input.read(cx).value().to_string());
                                if let Some(store) = &model.read(cx).store {
                                    if let Err(e) = crate::store::import_into(&store.conn, &path) {
                                        model.update(cx, |m, cx| {
                                            m.toast_error(format!("Import failed: {e}"));
                                            cx.notify();
                                        });
                                    } else {
                                        let path = model.read(cx).db_path.clone();
                                        model.update(cx, |m, cx| {
                                            m.reload_db(path);
                                            cx.notify();
                                        });
                                    }
                                }
                                window.close_dialog(cx);
                            }
                        }))),
                )
        });
    }
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (s, notes_count) = {
            let m = self.model.read(cx);
            (m.settings.clone(), m.notes.len())
        };
        let model = self.model.clone();
        let sounds = crate::util::list_sounds();
        let default_sound = s.default_alarm_sound.clone();
        let page = v_flex()
            .id("settings-scroll")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .p_6()
            .gap_4()
            .child(div().text_size(px(26.)).font_weight(gpui_kit::FontWeight::SEMIBOLD).child("Settings"))
            .child(section(cx, "General",
                v_flex()
                    .gap_1()
                    .child(toggle_row("Run on system start", s.auto_start, {
                        let model = model.clone();
                        move |value, cx| {
                            model.update(cx, |m, cx| {
                                m.settings.auto_start = value;
                                m.save_settings();
                                cx.notify();
                            });
                            crate::util::set_autostart(value);
                        }
                    }))
                    .child(toggle_row("Show notes when the program starts", s.show_all_notes_on_startup, {
                        let model = model.clone();
                        move |value, cx| {
                            model.update(cx, |m, cx| {
                                m.settings.show_all_notes_on_startup = value;
                                m.save_settings();
                                cx.notify();
                            });
                        }
                    }))
                    .child(toggle_row("Minimize to tray on close", s.close_to_tray, {
                        let model = model.clone();
                        move |value, cx| {
                            model.update(cx, |m, cx| {
                                m.settings.close_to_tray = value;
                                m.save_settings();
                                cx.notify();
                            });
                        }
                    }))
                    .child(toggle_row("Automatically check for updates", s.check_updates, {
                        let model = model.clone();
                        move |value, cx| {
                            model.update(cx, |m, cx| {
                                m.settings.check_updates = value;
                                m.save_settings();
                                cx.notify();
                            });
                        }
                    }))
                    .child(toggle_row("Enable Live Preview", s.live_preview, {
                        let model = model.clone();
                        move |value, cx| {
                            model.update(cx, |m, cx| {
                                m.settings.live_preview = value;
                                m.save_settings();
                                cx.notify();
                            });
                        }
                    }))
                    .child(toggle_row("Always show note title", s.always_show_title, {
                        let model = model.clone();
                        move |value, cx| {
                            model.update(cx, |m, cx| {
                                m.settings.always_show_title = value;
                                m.save_settings();
                                cx.notify();
                            });
                        }
                    }))
                    .child(h_flex().h(px(44.)).items_center().justify_between().child(div().text_sm().child("Application language")).child(
                        h_flex().gap_1().items_center().child(
                            Button::new("lang")
                                .outline()
                                .small()
                                .label(language_name(&s.language))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    let langs = this.model.read(cx).translator.languages();
                                    let model = this.model.clone();
                                    let current = model.read(cx).settings.language.clone();
                                    let idx = langs.iter().position(|(l, _)| *l == current).unwrap_or(0);
                                    let next = langs[(idx + 1) % langs.len()];
                                    let next = next.0.to_string();
                                    window.open_dialog(cx, {
                                        let model = model.clone();
                                        move |dialog, _window, _cx| {
                                            let mut col = v_flex().gap_1();
                                            for (i, (code, name)) in langs.iter().enumerate() {
                                                let model = model.clone();
                                                let code = code.to_string();
                                                let next = next.clone();
                                                col = col.child(Button::new(("lang-item", i)).label(*name).on_click(move |_, window: &mut Window, cx: &mut App| {
                                                    let code = code.clone();
                                                    model.update(cx, |m, cx| {
                                                        m.settings.language = code.clone();
                                                        m.translator.set_lang(&code);
                                                        m.save_settings();
                                                        cx.notify();
                                                    });
                                                    let _ = &next;
                                                    window.close_dialog(cx);
                                                }));
                                            }
                                            dialog.title("Language").w(px(360.)).child(col)
                                        }
                                    });
                                })),
                        ),
                    )),
            ))
            .child(section(cx, "Personalization",
                v_flex()
                    .gap_1()
                    .child(h_flex().h(px(44.)).items_center().justify_between().child(div().text_sm().child("Default note color")).child({
                        let model = model.clone();
                        let view = cx.entity().downgrade();
                        let initial = s.default_note.color;
                        h_flex().gap_2().items_center()
                            .child(div().w(px(18.)).h(px(18.)).rounded(px(4.)).bg(theme::rgba(initial)))
                            .child(Button::new("default-color").small().outline().label(super::color_wheel::hex_string(initial)).on_click(move |_, window, cx| {
                                let m = model.clone();
                                let v = view.clone();
                                super::color_wheel::open_color_wheel(window, cx, initial, move |value, cx| {
                                    m.update(cx, |m, cx| {
                                        m.settings.default_note.color = value;
                                        m.save_settings();
                                        cx.notify();
                                    });
                                    let _ = v.update(cx, |_, cx| cx.notify());
                                });
                            }))
                    }))
                    .child(h_flex().h(px(44.)).items_center().justify_between().child(div().text_sm().child("Default note opacity")).child(
                        Button::new("opacity-cycle").small().outline().label(format!("{}%", s.default_note.opacity)).on_click({
                            let model = model.clone();
                            move |_, _, cx: &mut App| {
                                model.update(cx, |m, cx| {
                                    let v = m.settings.default_note.opacity;
                                    m.settings.default_note.opacity = if v <= 20 { 100 } else { v - 10 };
                                    m.save_settings();
                                    cx.notify();
                                });
                            }
                        }),
                    ))
                    .child(h_flex().h(px(44.)).items_center().justify_between().child(div().text_sm().child("Default note size")).child(
                        Button::new("size-cycle").small().outline().label(format!("Size {}", s.default_note.size)).on_click({
                            let model = model.clone();
                            move |_, _, cx: &mut App| {
                                model.update(cx, |m, cx| {
                                    m.settings.default_note.size = if m.settings.default_note.size >= 6 { 1 } else { m.settings.default_note.size + 1 };
                                    m.save_settings();
                                    cx.notify();
                                });
                            }
                        }),
                    ))
                    .child(h_flex().h(px(44.)).items_center().justify_between().child(div().text_sm().child("Default note font")).child(
                        Button::new("font-cycle").small().outline().label(format!("{} {}", s.default_note.font_name, s.default_note.font_size)).on_click({
                            let model = model.clone();
                            move |_, _, cx: &mut App| {
                                model.update(cx, |m, cx| {
                                    let fonts = ["Segoe UI", "Tahoma", "Arial", "Verdana", "Georgia", "Times New Roman", "Courier New", "Consolas"];
                                    let idx = fonts.iter().position(|f| *f == m.settings.default_note.font_name).unwrap_or(0);
                                    m.settings.default_note.font_name = fonts[(idx + 1) % fonts.len()].to_string();
                                    m.save_settings();
                                    cx.notify();
                                });
                            }
                        }),
                    ))
                    .child(toggle_row("Always on top for new notes", s.default_note.always_on_top, {
                        let model = model.clone();
                        move |value, cx| {
                            model.update(cx, |m, cx| {
                                m.settings.default_note.always_on_top = value;
                                m.save_settings();
                                cx.notify();
                            });
                        }
                    })),
            ))
            .child(section(cx, "Appearance",
                v_flex()
                    .gap_1()
                    .child(choice_row("App mode", ["Light", "Dark", "System"], &s.app_mode.as_str(), {
                        let model = model.clone();
                        move |idx, cx| {
                            let mode = match idx {
                                0 => AppMode::Light,
                                1 => AppMode::Dark,
                                _ => AppMode::System,
                            };
                            model.update(cx, |m, cx| {
                                m.settings.app_mode = mode.clone();
                                m.save_settings();
                                cx.notify();
                            });
                            let dark = idx == 1 || (idx == 2 && crate::system_prefers_dark());
                            Theme::change(if dark { ThemeMode::Dark } else { ThemeMode::Light }, None, cx);
                            super::chrome::apply_window_theme(cx);
                        }
                    }))
                    .child(choice_row("Note mode", ["Light", "Dark", "Auto"], &s.note_mode.as_str(), {
                        let model = model.clone();
                        move |idx, cx| {
                            let mode = match idx {
                                0 => NoteMode::Light,
                                1 => NoteMode::Dark,
                                _ => NoteMode::Auto,
                            };
                            model.update(cx, |m, cx| {
                                m.settings.note_mode = mode;
                                m.save_settings();
                                cx.notify();
                            });
                        }
                    })),
            ))
            .child(section(
                cx,
                "Sounds",
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .justify_between()
                            .child(div().text_sm().child("Alarm sounds"))
                            .child(
                                h_flex().gap_1()
                                    .child(Button::new("sound-add-file").small().label("Add Sound...").on_click(cx.listener(|this, _, window, cx| this.add_sound_dialog(window, cx))))

                            ),
                    )
                    .when(sounds.is_empty(), |d| d.child(div().text_xs().text_color(cx.theme().muted_foreground).child("No sounds to display")))
                    .children(sounds.iter().enumerate().map(|(i, (name, path))| {
                        let path_play = path.clone();
                        let path_default = path.clone();
                        let model = model.clone();
                        let is_default = default_sound == *path;
                        h_flex()
                            .h(px(32.))
                            .items_center()
                            .gap_2()
                            .child(div().flex_1().text_sm().child(name.clone()))
                            .when(is_default, |d| d.child(div().text_xs().text_color(cx.theme().primary).child("Default")))
                            .child(Button::new(("play-sound", i)).xsmall().icon(IconName::Play).on_click(move |_, _, _| crate::util::play_sound(&path_play)))
                            .child(Button::new(("default-sound", i)).xsmall().label("Set default").on_click(move |_, _, cx: &mut App| {
                                model.update(cx, |m, cx| {
                                    m.settings.default_alarm_sound = path_default.clone();
                                    m.save_settings();
                                    cx.notify();
                                });
                            }))
                    })),
            ))
            .child(section(
                cx,
                "Hot Keys",
                v_flex()
                    .gap_1()
                    .child(hotkey_row("New Note", &s.hotkeys.new_note))
                    .child(hotkey_row("New Note from clipboard", &s.hotkeys.new_note_from_cb))
                    .child(hotkey_row("Show All Notes", &s.hotkeys.show_all_notes))
                    .child(hotkey_row("Hide All Notes", &s.hotkeys.hide_all_notes))
                    .child(hotkey_row("Go to next note", &s.hotkeys.go_next))
                    .child(hotkey_row("Explorer", &s.hotkeys.explorer)),
            ))
            .child(section(cx, "Database",
                v_flex()
                    .gap_2()
                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child(format!("{} notes · {}", notes_count, s.db_location)))
                    .child(h_flex().gap_2().child(Button::new("backup").small().label("Backup").on_click(cx.listener(|this, _, window, cx| this.backup_dialog(window, cx)))).child(Button::new("restore").small().label("Restore / Import").on_click(cx.listener(|this, _, window, cx| this.restore_dialog(window, cx)))))
                    .child(h_flex().gap_2().child(Button::new("change-db-loc").small().label("Change location...").on_click(cx.listener(|this, _, window, cx| this.location_dialog(window, cx)))).child(Button::new("open-db-folder").small().label("Open Folder").on_click({
                        let path = s.db_location.clone();
                        move |_, _, _| {
                            crate::util::open_path(std::path::Path::new(&path));
                        }
                    })).child(Button::new("compact").small().label("Compact").on_click({
                        let model = model.clone();
                        move |_, _, cx: &mut App| {
                            if let Some(store) = &model.read(cx).store {
                                let _ = store.conn.execute("VACUUM", []);
                            }
                        }
                    }))),
            ))
            .child(section(cx, "Links",
                h_flex().gap_2().child(Button::new("about").small().label("About").on_click({
                    let model = model.clone();
                    move |_, _, cx: &mut App| windows::open_about_window(cx, model.clone())
                })),
            ))
            .child(div().h(px(24.)));

        super::chrome::content_frame(
            v_flex()
                .size_full()
                .text_color(cx.theme().foreground)
                .child(super::chrome::title_bar(
                    "StickyDesk – Settings",
                    window,
                    cx,
                ))
                .child(page),
            cx,
        )
    }
}

fn section(cx: &gpui_kit::App, title: &str, content: impl IntoElement) -> impl IntoElement {
    v_flex()
        .gap_2()
        .p_4()
        .rounded(px(8.))
        .border_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .text_size(px(14.))
                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                .child(title.to_string()),
        )
        .child(content)
}

fn toggle_row(
    label: &str,
    checked: bool,
    handler: impl Fn(bool, &mut App) + 'static,
) -> impl IntoElement {
    let id = format!("toggle-{label}");
    h_flex()
        .h(px(40.))
        .items_center()
        .justify_between()
        .child(div().text_sm().child(label.to_string()))
        .child(
            Switch::new(id)
                .checked(checked)
                .on_change(move |value, _, cx| handler(*value, cx)),
        )
}

fn choice_row(
    label: &str,
    options: [&'static str; 3],
    current: &str,
    handler: impl Fn(usize, &mut App) + 'static,
) -> impl IntoElement {
    let handler: std::rc::Rc<dyn Fn(usize, &mut App)> = std::rc::Rc::new(handler);
    let mut row = h_flex().gap_1();
    for (i, opt) in options.iter().enumerate() {
        let handler = handler.clone();
        let selected = current.eq_ignore_ascii_case(opt);
        row = row.child(
            Button::new(format!("choice-{label}-{i}"))
                .small()
                .when(selected, |b| b.primary())
                .when(!selected, |b| b.outline())
                .label(*opt)
                .on_click(move |_, _, cx| handler(i, cx)),
        );
    }
    h_flex()
        .h(px(44.))
        .items_center()
        .justify_between()
        .child(div().text_sm().child(label.to_string()))
        .child(row)
}

fn hotkey_row(label: &str, keys: &str) -> impl IntoElement {
    h_flex()
        .h(px(28.))
        .items_center()
        .justify_between()
        .child(div().text_sm().child(label.to_string()))
        .child(
            div()
                .text_xs()
                .text_color(gpui_kit::rgb(0x808080))
                .child(keys.to_string()),
        )
}
