use super::model::{AppModel, ExplorerPage};
use super::windows;
use crate::model::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants},
    dialog::{DialogClose, DialogFooter},
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, ClickEvent, Context, Entity, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, Window, div, prelude::FluentBuilder as _, px,
};

pub struct ExplorerView {
    model: Entity<AppModel>,
    search: Entity<InputState>,
    query: String,
    sort_desc: bool,
    sort_by: SortBy,
    _subs: Vec<Subscription>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortBy {
    Updated,
    Created,
    Title,
}

impl ExplorerView {
    pub fn new(model: Entity<AppModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.set_client_corner_radii(gpui_kit::Corners::all(px(12.)));
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));
        let mut subs = Vec::new();
        subs.push(
            cx.subscribe_in(&search, window, |this, state, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.query = state.read(cx).value().to_string();
                    cx.notify();
                }
            }),
        );
        subs.push(cx.observe(&model, |_, _, cx| cx.notify()));
        Self {
            model,
            search,
            query: String::new(),
            sort_desc: true,
            sort_by: SortBy::Updated,
            _subs: subs,
        }
    }

    fn open_note_menu(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        let model = self.model.clone();
        let note = self.model.read(cx).note(id).cloned();
        let Some(note) = note else { return };
        let trashed = note.is_trashed();
        let title = note.title.clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let m_open = model.clone();
            let open =
                Button::new("row-open")
                    .label("Open Note")
                    .on_click(move |_, _, cx: &mut App| {
                        windows::open_note_window(cx, m_open.clone(), id);
                    });
            let m_info = model.clone();
            let info =
                Button::new("row-info").label("Note Info").on_click(
                    move |_, window: &mut Window, cx: &mut App| {
                        let note = m_info.read(cx).note(id).cloned();
                        if let Some(note) = note {
                            window.open_dialog(cx, move |dialog, _window, _cx| {
                                dialog
                                    .title(note.title.clone())
                                    .w(px(460.))
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .child(info_row("Notebook", &note.notebook))
                                            .child(info_row("Created", &fmt_dt(note.created_dt())))
                                            .child(info_row("Updated", &fmt_dt(note.updated_dt())))
                                            .child(info_row("Deleted", &fmt_dt(note.deleted_dt())))
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(gpui_kit::rgb(0x808080))
                                                    .child(truncate(&note.doc.text, 220)),
                                            ),
                                    )
                                    .footer(DialogFooter::new().child(
                                        DialogClose::new().child(
                                            Button::new("info-close").label("Close").outline(),
                                        ),
                                    ))
                            });
                        }
                    },
                );
            let m_rename = model.clone();
            let rename = Button::new("row-edit-title").label("Edit Title").on_click(
                move |_, window: &mut Window, cx: &mut App| {
                    let m = m_rename.clone();
                    let title = m
                        .read(cx)
                        .note(id)
                        .map(|n| n.title.clone())
                        .unwrap_or_default();
                    let input = cx.new(|cx| InputState::new(window, cx).placeholder("Title"));
                    input.update(cx, |state, cx| state.set_value(title, window, cx));
                    window.open_dialog(cx, move |dialog, _window, _cx| {
                        let m = m.clone();
                        let input = input.clone();
                        dialog
                            .title("Edit title")
                            .w(px(360.))
                            .child(Input::new(&input))
                            .footer(
                                DialogFooter::new()
                                    .gap_2()
                                    .child(
                                        DialogClose::new()
                                            .child(Button::new("title-cancel").label("Cancel")),
                                    )
                                    .child(DialogClose::new().child(
                                        Button::new("title-save").primary().label("Save").on_click(
                                            move |_, window: &mut Window, cx: &mut App| {
                                                let value =
                                                    input.read(cx).value().trim().to_string();
                                                if !value.is_empty() {
                                                    m.update(cx, |m, cx| {
                                                        m.mutate_note(id, |n| {
                                                            n.title = value.clone()
                                                        });
                                                        cx.notify();
                                                    });
                                                }
                                                window.close_dialog(cx);
                                            },
                                        ),
                                    )),
                            )
                    });
                },
            );
            let m_star = model.clone();
            let star = Button::new("row-star").label("Toggle Star").on_click(
                move |_, window: &mut Window, cx: &mut App| {
                    m_star.update(cx, |m, cx| {
                        m.mutate_note(id, |n| n.starred = !n.starred);
                        cx.notify();
                    });
                    window.close_dialog(cx);
                },
            );
            let m_trash = model.clone();
            let trash = Button::new("row-trash").label("Move to Trash").on_click(
                move |_, window: &mut Window, cx: &mut App| {
                    m_trash.update(cx, |m, cx| {
                        m.trash_note(id);
                        cx.notify();
                    });
                    window.close_dialog(cx);
                },
            );
            let m_restore = model.clone();
            let restore = Button::new("row-restore").label("Restore").on_click(
                move |_, window: &mut Window, cx: &mut App| {
                    m_restore.update(cx, |m, cx| {
                        m.restore_note(id);
                        cx.notify();
                    });
                    window.close_dialog(cx);
                },
            );
            let m_delete = model.clone();
            let delete = Button::new("row-delete").label("Delete forever").on_click(
                move |_, window: &mut Window, cx: &mut App| {
                    m_delete.update(cx, |m, cx| {
                        m.delete_forever(id);
                        cx.notify();
                    });
                    window.close_dialog(cx);
                },
            );
            dialog.title(title.clone()).w(px(320.)).child(
                v_flex()
                    .gap_1()
                    .when(!trashed, |d| {
                        d.child(open)
                            .child(rename)
                            .child(star)
                            .child(info)
                            .child(trash)
                    })
                    .when(trashed, |d| d.child(restore).child(delete))
                    .child(DialogFooter::new().mt_2().child(
                        DialogClose::new().child(Button::new("row-close").label("Close").outline()),
                    )),
            )
        });
    }

    fn bulk_action_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let model = self.model.clone();
        let selection = self.model.read(cx).selection.clone();
        let page = self.model.read(cx).explorer_page;
        let trash_page = page == ExplorerPage::Trash;
        let mut bar = h_flex().gap_2().items_center();
        if trash_page {
            bar = bar
                .child(
                    Button::new("bulk-restore")
                        .small()
                        .label("Restore")
                        .on_click({
                            let model = model.clone();
                            move |_, _, cx: &mut App| {
                                let sel = model.read(cx).selection.clone();
                                model.update(cx, |m, cx| {
                                    for id in sel {
                                        m.restore_note(id);
                                    }
                                    m.selection.clear();
                                    cx.notify();
                                });
                            }
                        }),
                )
                .child(
                    Button::new("bulk-delete")
                        .small()
                        .danger()
                        .label("Delete forever")
                        .on_click({
                            let model = model.clone();
                            move |_, _, cx: &mut App| {
                                let sel = model.read(cx).selection.clone();
                                model.update(cx, |m, cx| {
                                    for id in sel {
                                        m.delete_forever(id);
                                    }
                                    m.selection.clear();
                                    cx.notify();
                                });
                            }
                        }),
                )
                .child(
                    Button::new("bulk-empty")
                        .small()
                        .label("Empty Trash...")
                        .on_click({
                            let model = model.clone();
                            move |_, _, cx: &mut App| {
                                model.update(cx, |m, cx| {
                                    m.empty_trash();
                                    m.selection.clear();
                                    cx.notify();
                                });
                            }
                        }),
                );
        } else {
            bar = bar
                .child(
                    Button::new("bulk-move")
                        .small()
                        .label("Move To...")
                        .on_click({
                            let model = model.clone();
                            move |_, window: &mut Window, cx: &mut App| {
                                let names = model.read(cx).notebooks_with_default();
                                let model = model.clone();
                                window.open_dialog(cx, move |dialog, _window, _cx| {
                                    let mut col = v_flex().gap_1();
                                    for (i, name) in names.iter().enumerate() {
                                        let model = model.clone();
                                        let name = name.clone();
                                        col = col.child(
                                            Button::new(("move-to", i))
                                                .label(name.clone())
                                                .on_click(
                                                    move |_, window: &mut Window, cx: &mut App| {
                                                        let name = name.clone();
                                                        model.update(cx, |m, cx| {
                                                            let sel = m.selection.clone();
                                                            for id in sel {
                                                                m.move_note(id, &name);
                                                            }
                                                            m.selection.clear();
                                                            cx.notify();
                                                        });
                                                        window.close_dialog(cx);
                                                    },
                                                ),
                                        );
                                    }
                                    dialog.title("Move To").w(px(320.)).child(col)
                                });
                            }
                        }),
                )
                .child(
                    Button::new("bulk-merge")
                        .small()
                        .label("Merge Notes")
                        .on_click({
                            let model = model.clone();
                            move |_, _, cx: &mut App| {
                                let sel = model.read(cx).selection.clone();
                                if let Some(id) = model.update(cx, |m, _| m.merge_notes(&sel)) {
                                    windows::open_note_window(cx, model.clone(), id);
                                }
                            }
                        }),
                )
                .child(
                    Button::new("bulk-trash")
                        .small()
                        .label("Move to Trash")
                        .on_click({
                            let model = model.clone();
                            move |_, _, cx: &mut App| {
                                let sel = model.read(cx).selection.clone();
                                model.update(cx, |m, cx| {
                                    for id in sel {
                                        m.trash_note(id);
                                    }
                                    m.selection.clear();
                                    cx.notify();
                                });
                            }
                        }),
                );
        }
        bar.child(div().flex_1()).child(
            Button::new("bulk-clear")
                .small()
                .ghost()
                .label(format!("{} selected", selection.len()))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.model.update(cx, |m, cx| {
                        m.selection.clear();
                        cx.notify();
                    });
                })),
        )
    }
}

impl Render for ExplorerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (_settings, notes, notebooks, page, active_nb, selection) = {
            let m = self.model.read(cx);
            (
                m.settings.clone(),
                m.notes.clone(),
                m.notebooks_with_default(),
                m.explorer_page,
                m.active_notebook.clone(),
                m.selection.clone(),
            )
        };
        let query = self.query.to_lowercase();
        let notebook_counts: std::collections::HashMap<String, usize> = notes
            .iter()
            .filter(|n| !n.is_trashed())
            .fold(std::collections::HashMap::new(), |mut acc, n| {
                *acc.entry(n.notebook.clone()).or_insert(0) += 1;
                acc
            });
        let mut list: Vec<Note> = notes
            .into_iter()
            .filter(|n| match page {
                ExplorerPage::Trash => n.is_trashed(),
                ExplorerPage::Starred => !n.is_trashed() && n.starred,
                ExplorerPage::Notebook => !n.is_trashed() && n.notebook == active_nb,
                ExplorerPage::AllNotes => !n.is_trashed(),
            })
            .filter(|n| {
                query.is_empty()
                    || n.title.to_lowercase().contains(&query)
                    || n.doc.text.to_lowercase().contains(&query)
            })
            .collect();
        match self.sort_by {
            SortBy::Updated => list.sort_by(|a, b| {
                a.updated
                    .partial_cmp(&b.updated)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortBy::Created => list.sort_by(|a, b| {
                a.created
                    .partial_cmp(&b.created)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortBy::Title => {
                list.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
            }
        }
        if self.sort_desc {
            list.reverse();
        }
        let total = list.len();
        let page_title = match page {
            ExplorerPage::AllNotes => "All Notes".to_string(),
            ExplorerPage::Starred => "Starred".to_string(),
            ExplorerPage::Trash => "Trash".to_string(),
            ExplorerPage::Notebook => active_nb.clone(),
        };
        let model = self.model.clone();

        let sidebar = v_flex()
            .w(px(240.))
            .h_full()
            .p_2()
            .gap_1()
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .h(px(44.))
                    .items_center()
                    .gap_2()
                    .px_1()
                    .child(Icon::new(IconName::NotebookPen))
                    .child(
                        div()
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child("StickyDesk"),
                    ),
            )
            .child(Input::new(&self.search))
            .child(
                h_flex()
                    .h(px(28.))
                    .px_1()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child("Notebooks"),
                    )
                    .child(
                        Button::new("nb-new-icon")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Plus)
                            .tooltip("New Notebook")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_new_notebook(window, cx);
                            })),
                    ),
            )
            .children(notebooks.iter().enumerate().map(|(i, name)| {
                let name = name.clone();
                let selected = page == ExplorerPage::Notebook && active_nb == name;
                let model = model.clone();
                let _ = i;
                let count = *notebook_counts.get(&name).unwrap_or(&0);
                let select_name = name.clone();
                let rename_name = name.clone();
                let delete_name = name.clone();
                notebook_item(
                    cx,
                    name.clone(),
                    count,
                    selected,
                    move |_, _, cx: &mut App| {
                        let n = select_name.clone();
                        model.update(cx, |m, cx| {
                            m.explorer_page = ExplorerPage::Notebook;
                            m.active_notebook = n.clone();
                            m.settings.active_notebook = n;
                            m.save_settings();
                            m.selection.clear();
                            cx.notify();
                        });
                    },
                    cx.listener(move |this, _, window, cx| {
                        this.rename_notebook_dialog(rename_name.clone(), window, cx);
                    }),
                    cx.listener(move |this, _, window, cx| {
                        this.delete_notebook_dialog(delete_name.clone(), window, cx);
                    }),
                )
            }))
            .child(section_label("Notes"))
            .child(nav_item(
                cx,
                "All Notes",
                Icon::new(IconName::FileText),
                page == ExplorerPage::AllNotes,
                None,
                {
                    let model = model.clone();
                    move |_, _, cx| {
                        model.update(cx, |m, cx| {
                            m.explorer_page = ExplorerPage::AllNotes;
                            m.active_notebook = "Default".into();
                            m.selection.clear();
                            cx.notify();
                        });
                    }
                },
            ))
            .child(nav_item(
                cx,
                "Starred",
                Icon::new(IconName::Star),
                page == ExplorerPage::Starred,
                None,
                {
                    let model = model.clone();
                    move |_, _, cx| {
                        model.update(cx, |m, cx| {
                            m.explorer_page = ExplorerPage::Starred;
                            m.selection.clear();
                            cx.notify();
                        });
                    }
                },
            ))
            .child(nav_item(
                cx,
                "Trash",
                Icon::new(IconName::Trash).data(TRASH_SVG.as_bytes()),
                page == ExplorerPage::Trash,
                None,
                {
                    let model = model.clone();
                    move |_, _, cx| {
                        model.update(cx, |m, cx| {
                            m.explorer_page = ExplorerPage::Trash;
                            m.selection.clear();
                            cx.notify();
                        });
                    }
                },
            ))
            .child(div().flex_1())
            .child(nav_item(
                cx,
                "Settings",
                Icon::new(IconName::Settings),
                false,
                None,
                {
                    let model = model.clone();
                    move |_, _, cx| {
                        windows::open_settings_window(cx, model.clone());
                    }
                },
            ))
            .child(nav_item(
                cx,
                "About",
                Icon::new(IconName::Info),
                false,
                None,
                {
                    let model = model.clone();
                    move |_, _, cx| {
                        windows::open_about_window(cx, model.clone());
                    }
                },
            ));

        let header = h_flex()
            .h(px(56.))
            .items_center()
            .gap_3()
            .px_6()
            .child(
                div()
                    .text_size(px(24.))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(page_title),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("{total} notes")),
            )
            .child(div().flex_1())
            .child(
                Button::new("sort")
                    .small()
                    .ghost()
                    .icon(IconName::ArrowDownWideNarrow)
                    .label(match self.sort_by {
                        SortBy::Updated => "Updated",
                        SortBy::Created => "Created",
                        SortBy::Title => "Title",
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.sort_by = match this.sort_by {
                            SortBy::Updated => SortBy::Created,
                            SortBy::Created => SortBy::Title,
                            SortBy::Title => SortBy::Updated,
                        };
                        this.sort_desc = !this.sort_desc;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("show-all")
                    .small()
                    .icon(IconName::Eye)
                    .label("Show All")
                    .on_click({
                        let model = model.clone();
                        move |_, _, cx: &mut App| windows::show_all_notes(cx, model.clone())
                    }),
            )
            .child(
                Button::new("hide-all")
                    .small()
                    .icon(IconName::EyeOff)
                    .label("Hide All")
                    .on_click({
                        let model = model.clone();
                        move |_, _, cx: &mut App| windows::hide_all_notes(cx, model.clone())
                    }),
            )
            .child(
                Button::new("from-clipboard")
                    .small()
                    .icon(IconName::Clipboard)
                    .label("From Clipboard")
                    .on_click(cx.listener(|this, _, _, cx| {
                        let text = cx.read_from_clipboard().and_then(|item| {
                            item.entries.iter().find_map(|e| match e {
                                gpui_kit::ClipboardEntry::String(s) => Some(s.text.clone()),
                                _ => None,
                            })
                        });
                        windows::new_note(cx, this.model.clone(), text, None);
                    })),
            )
            .child(
                Button::new("new-note")
                    .primary()
                    .icon(IconName::Plus)
                    .label("New Note")
                    .on_click(cx.listener(|this, _, _, cx| {
                        let id = this.model.update(cx, |m, _| m.create_note(None, None));
                        windows::open_note_window(cx, this.model.clone(), id);
                    })),
            );

        let rows = v_flex()
            .id("note-rows")
            .flex_1()
            .min_h(px(0.))
            .gap_1()
            .p_4()
            .overflow_y_scroll()
            .children(list.iter().enumerate().map(|(_i, note)| {
                let id = note.id;
                let selected = selection.contains(&id);
                let title = note.title.clone();
                let snippet = truncate(&note.doc.text.replace('\n', " "), 100);
                let updated = fmt_dt(note.updated_dt());
                let starred = note.starred;
                let color = note.color;
                let locked = note.locked;
                let notebook = note.notebook.clone();
                let model = model.clone();
                div()
                    .id(("note-row", id as u64))
                    .flex()
                    .items_center()
                    .gap_3()
                    .h(px(54.))
                    .px_3()
                    .rounded(px(6.))
                    .cursor(gpui_kit::CursorStyle::PointingHand)
                    .when(selected, |d| d.bg(cx.theme().accent.opacity(0.14)))
                    .hover(|d| d.bg(cx.theme().muted.opacity(0.25)))
                    .on_click(move |event: &ClickEvent, _, cx: &mut App| {
                        let additive = event.modifiers().control || event.modifiers().shift;
                        model.update(cx, |m, cx| {
                            if additive {
                                if let Some(pos) = m.selection.iter().position(|x| *x == id) {
                                    m.selection.remove(pos);
                                } else {
                                    m.selection.push(id);
                                }
                            } else {
                                m.selection = vec![id];
                            }
                            cx.notify();
                        });
                        if event.click_count() >= 2 {
                            windows::open_note_window(cx, model.clone(), id);
                        }
                    })
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, _, window, cx| {
                            this.open_note_menu(id, window, cx);
                        }),
                    )
                    .child(
                        div()
                            .w(px(6.))
                            .h(px(34.))
                            .rounded_full()
                            .bg(crate::theme::rgba(color)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w(px(0.))
                            .gap_0p5()
                            .child(
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                            .child(title),
                                    )
                                    .when(starred, |d| {
                                        d.child(
                                            Icon::new(IconName::Star)
                                                .xsmall()
                                                .text_color(cx.theme().warning),
                                        )
                                    })
                                    .when(locked, |d| {
                                        d.child(
                                            Icon::new(IconName::Lock)
                                                .xsmall()
                                                .text_color(cx.theme().muted_foreground),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(snippet),
                            ),
                    )
                    .child(
                        v_flex()
                            .items_end()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(updated),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(notebook),
                            ),
                    )
            }));

        let empty = total == 0;
        let body = if empty {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_3()
                .child(
                    Icon::new(IconName::NotebookPen)
                        .large()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .child("No notes to display"),
                )
                .into_any_element()
        } else {
            rows.into_any_element()
        };

        let inner = h_flex()
            .size_full()
            .on_action(
                cx.listener(|this, _: &crate::app::actions::NewNote, _, cx| {
                    let id = this.model.update(cx, |m, _| m.create_note(None, None));
                    windows::open_note_window(cx, this.model.clone(), id);
                }),
            )
            .on_action(cx.listener(
                |this, _: &crate::app::actions::NewNoteFromClipboard, _, cx| {
                    let text = cx.read_from_clipboard().and_then(|item| {
                        item.entries.iter().find_map(|e| match e {
                            gpui_kit::ClipboardEntry::String(s) => Some(s.text.clone()),
                            _ => None,
                        })
                    });
                    windows::new_note(cx, this.model.clone(), text, None);
                },
            ))
            .on_action(
                cx.listener(|this, _: &crate::app::actions::ShowAllNotes, _, cx| {
                    windows::show_all_notes(cx, this.model.clone());
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::app::actions::HideAllNotes, _, cx| {
                    windows::hide_all_notes(cx, this.model.clone());
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::app::actions::GoNextNote, _, cx| {
                    let ids: Vec<i64> = this
                        .model
                        .read(cx)
                        .notes
                        .iter()
                        .filter(|n| !n.is_trashed())
                        .map(|n| n.id)
                        .collect();
                    if let Some(id) = ids.first() {
                        windows::open_note_window(cx, this.model.clone(), *id);
                    }
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::app::actions::OpenSettings, _, cx| {
                    windows::open_settings_window(cx, this.model.clone());
                }),
            )
            .text_color(cx.theme().foreground)
            .child(sidebar)
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .child(header)
                    .when(!selection.is_empty(), |d| {
                        d.child(div().px_4().pb_2().child(self.bulk_action_bar(cx)))
                    })
                    .child(body),
            );
        super::chrome::content_frame(
            v_flex()
                .size_full()
                .child(super::chrome::title_bar(
                    "StickyDesk – Explorer",
                    window,
                    cx,
                ))
                .child(inner),
            cx,
        )
    }
}

impl ExplorerView {
    fn rename_notebook_dialog(&mut self, old: String, window: &mut Window, cx: &mut Context<Self>) {
        if old == "Default" {
            return;
        }
        let model = self.model.clone();
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Notebook name"));
        input.update(cx, |state, cx| state.set_value(old.clone(), window, cx));
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let model = model.clone();
            let input = input.clone();
            let old = old.clone();
            dialog
                .title("Rename Notebook")
                .w(px(380.))
                .child(Input::new(&input))
                .footer(
                    DialogFooter::new()
                        .gap_2()
                        .child(
                            DialogClose::new()
                                .child(Button::new("nb-rename-cancel").label("Cancel")),
                        )
                        .child(
                            Button::new("nb-rename-save")
                                .primary()
                                .label("Rename")
                                .on_click({
                                    let model = model.clone();
                                    let input = input.clone();
                                    move |_, window: &mut Window, cx: &mut App| {
                                        let new = input.read(cx).value().trim().to_string();
                                        if !new.is_empty() {
                                            model.update(cx, |m, cx| {
                                                m.rename_notebook(&old, &new);
                                                m.save_settings();
                                                cx.notify();
                                            });
                                        }
                                        window.close_dialog(cx);
                                    }
                                }),
                        ),
                )
        });
    }

    fn delete_notebook_dialog(
        &mut self,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if name == "Default" {
            return;
        }
        let model = self.model.clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let model = model.clone();
            let name = name.clone();
            dialog
                .title(format!("Delete notebook \"{name}\"?"))
                .w(px(420.))
                .child(
                    div()
                        .text_sm()
                        .child("Notes in this notebook will be deleted."),
                )
                .footer(
                    DialogFooter::new()
                        .gap_2()
                        .child(
                            DialogClose::new()
                                .child(Button::new("nb-delete-cancel").label("Cancel")),
                        )
                        .child(
                            Button::new("nb-delete-confirm")
                                .danger()
                                .label("Delete")
                                .on_click({
                                    let model = model.clone();
                                    let name = name.clone();
                                    move |_, window: &mut Window, cx: &mut App| {
                                        model.update(cx, |m, cx| {
                                            m.delete_notebook(&name);
                                            cx.notify();
                                        });
                                        window.close_dialog(cx);
                                    }
                                }),
                        ),
                )
        });
    }

    fn open_new_notebook(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let model = self.model.clone();
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Notebook name"));
        window.open_dialog(cx, move |dialog, _window, _cx| {
            dialog
                .title("New Notebook")
                .w(px(380.))
                .child(Input::new(&input))
                .footer(
                    DialogFooter::new()
                        .gap_2()
                        .child(DialogClose::new().child(Button::new("nb-cancel").label("Cancel")))
                        .child(DialogClose::new().child(
                            Button::new("nb-save").primary().label("Create").on_click({
                                let model = model.clone();
                                let input = input.clone();
                                move |_, window: &mut Window, cx: &mut App| {
                                    let name = input.read(cx).value().trim().to_string();
                                    if !name.is_empty() {
                                        model.update(cx, |m, cx| {
                                            m.create_notebook(&name);
                                            cx.notify();
                                        });
                                    }
                                    window.close_dialog(cx);
                                }
                            }),
                        )),
                )
        });
    }
}

fn section_label(text: &str) -> impl IntoElement {
    div()
        .px_1()
        .pt_3()
        .pb_1()
        .text_xs()
        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
        .child(text.to_string())
}

const TRASH_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M6 6l1 15h10l1-15"/><path d="M10 11v6"/><path d="M14 11v6"/></svg>"#;

fn notebook_item(
    cx: &gpui_kit::App,
    name: String,
    count: usize,
    selected: bool,
    on_select: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_rename: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_delete: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let hover_bg = cx.theme().muted.opacity(0.28);
    let selected_bg = cx.theme().accent.opacity(0.16);
    h_flex()
        .id(format!("notebook-{name}"))
        .w_full()
        .h(px(30.))
        .px_2()
        .gap_1()
        .items_center()
        .justify_start()
        .rounded(px(4.))
        .text_sm()
        .text_color(cx.theme().foreground)
        .when(selected, |d| d.bg(selected_bg))
        .hover(move |d| d.bg(hover_bg))
        .cursor(gpui_kit::CursorStyle::PointingHand)
        .on_click(on_select)
        .child(Icon::new(IconName::Folder).small())
        .child(div().flex_1().min_w(px(0.)).child(name.clone()))
        .when(count > 0, |d| {
            d.child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(count.to_string()),
            )
        })
        .when(name != "Default", |d| {
            d.child(
                Button::new(format!("nb-row-rename-{name}"))
                    .ghost()
                    .xsmall()
                    .icon(IconName::Pencil)
                    .tooltip("Rename")
                    .on_click(on_rename),
            )
            .child(
                Button::new(format!("nb-row-delete-{name}"))
                    .ghost()
                    .xsmall()
                    .icon(IconName::TrashOff)
                    .tooltip("Delete")
                    .on_click(on_delete),
            )
        })
}

fn nav_item(
    cx: &gpui_kit::App,
    label: impl Into<SharedString>,
    icon: Icon,
    selected: bool,
    trailing: Option<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label: SharedString = label.into();
    let hover_bg = cx.theme().muted.opacity(0.28);
    let selected_bg = cx.theme().accent.opacity(0.16);
    h_flex()
        .id(format!("nav-{label}"))
        .w_full()
        .h(px(30.))
        .px_2()
        .gap_2()
        .items_center()
        .justify_start()
        .rounded(px(4.))
        .text_sm()
        .text_color(cx.theme().foreground)
        .when(selected, |d| d.bg(selected_bg))
        .hover(move |d| d.bg(hover_bg))
        .cursor(gpui_kit::CursorStyle::PointingHand)
        .on_click(on_click)
        .child(icon.small())
        .child(div().flex_1().child(label.clone()))
        .when_some(trailing, |d, trailing| {
            d.child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(trailing),
            )
        })
}

fn info_row(label: &str, value: &str) -> impl IntoElement {
    h_flex()
        .gap_2()
        .child(
            div()
                .w(px(80.))
                .text_sm()
                .text_color(gpui_kit::rgb(0x808080))
                .child(label.to_string()),
        )
        .child(div().text_sm().child(value.to_string()))
}

fn fmt_dt(dt: Option<chrono::NaiveDateTime>) -> String {
    dt.map(|d| d.format("%m/%d/%Y %H:%M").to_string())
        .unwrap_or_else(|| "-".into())
}

fn truncate(s: &str, n: usize) -> String {
    let mut out: String = s.chars().take(n).collect();
    if s.chars().count() > n {
        out.push('…');
    }
    out
}
