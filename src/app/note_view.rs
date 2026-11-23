use super::model::AppModel;
use super::windows;
use crate::model::*;
use crate::theme;
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants},
    dialog::{DialogClose, DialogFooter},
    h_flex,
    input::{Input, InputEvent, InputState, Textarea, TextareaState},
    menu::{DropdownMenu as _, PopupMenu, PopupMenuItem},
    text::TextView,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, ClickEvent, Context, Corners, Entity, FocusHandle,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, px,
};

pub struct NoteView {
    model: Entity<AppModel>,
    note_id: i64,
    editor: Entity<TextareaState>,
    focus_handle: FocusHandle,
    title_input: Entity<InputState>,
    find_input: Entity<InputState>,
    replace_input: Entity<InputState>,
    match_case: bool,
    collapsed: bool,
    preview: bool,
    hovered: bool,
    expanded_height: i32,
    last_text: String,
    _subscriptions: Vec<Subscription>,
}

impl NoteView {
    pub fn new(
        note_id: i64,
        model: Entity<AppModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let doc = model
            .read(cx)
            .note(note_id)
            .map(|n| n.doc.clone())
            .unwrap_or_default();
        let text = doc.text.clone();
        let editor = cx.new(|cx| {
            TextareaState::new(window, cx)
                .soft_wrap(true)
                .placeholder("Write in Markdown… **bold**, *italic*, # heading, - list, - [ ] task")
        });
        editor.update(cx, |state, cx| {
            state.set_value(text.clone(), window, cx);
        });
        let focus_handle = editor.read(cx).presentation().focus_handle().clone();
        let title_input = cx.new(|cx| InputState::new(window, cx).placeholder("Title"));
        let find_input = cx.new(|cx| InputState::new(window, cx).placeholder("Find what"));
        let replace_input = cx.new(|cx| InputState::new(window, cx).placeholder("Replace with"));
        window.set_client_corner_radii(Corners::all(px(8.)));
        let sub = cx.subscribe_in(
            &editor,
            window,
            |this, state, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    let text = state.read(cx).value().to_string();
                    if text != this.last_text {
                        this.last_text = text.clone();
                        let id = this.note_id;
                        this.model.update(cx, |m, _| m.update_doc_text(id, text));
                    }
                }
            },
        );
        let this = Self {
            model,
            note_id,
            editor,
            focus_handle,
            title_input,
            find_input,
            replace_input,
            match_case: false,
            collapsed: false,
            preview: std::env::var_os("SSN_PREVIEW").is_some(),
            hovered: false,
            expanded_height: 0,
            last_text: text,
            _subscriptions: vec![sub],
        };
        this
    }

    fn note(&self, cx: &App) -> Option<Note> {
        self.model.read(cx).note(self.note_id).cloned()
    }

    fn edit_title(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = self.note(cx).map(|n| n.title).unwrap_or_default();
        if self.note(cx).map(|n| n.locked).unwrap_or(false) {
            return;
        }
        self.title_input
            .update(cx, |state, cx| state.set_value(title, window, cx));
        let model = self.model.clone();
        let input = self.title_input.clone();
        let id = self.note_id;
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let model = model.clone();
            let input = input.clone();
            dialog
                .title("Edit title")
                .w(px(380.))
                .child(Input::new(&input))
                .footer(
                    DialogFooter::new()
                        .gap_2()
                        .child(
                            DialogClose::new().child(Button::new("cancel-title").label("Cancel")),
                        )
                        .child(DialogClose::new().child(
                            Button::new("save-title").primary().label("Save").on_click(
                                move |_, window: &mut Window, cx: &mut App| {
                                    let value = input.read(cx).value().trim().to_string();
                                    if !value.is_empty() {
                                        model.update(cx, |m, cx| {
                                            m.mutate_note(id, |n| n.title = value);
                                            cx.notify();
                                        });
                                    }
                                    window.close_dialog(cx);
                                },
                            ),
                        )),
                )
        });
    }

    fn format_bool(&mut self, which: u8, window: &mut Window, cx: &mut Context<Self>) {
        let (p, s) = match which {
            0 => ("**", "**"),
            1 => ("*", "*"),
            2 => ("<u>", "</u>"),
            _ => ("~~", "~~"),
        };
        self.markdown_wrap(p, s, window, cx);
    }

    fn clear_formatting(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.markdown_clear(window, cx);
    }

    fn toggle_line_prefix(&mut self, kind: u8, window: &mut Window, cx: &mut Context<Self>) {
        let prefix = match kind {
            0 => "- ",
            1 => "1. ",
            _ => "- [ ] ",
        };
        self.markdown_line_prefix(prefix, window, cx);
    }

    fn set_note_align(&mut self, align: Align, window: &mut Window, cx: &mut Context<Self>) {
        let value = match align {
            Align::Left => "left",
            Align::Center => "center",
            Align::Right => "right",
        };
        self.markdown_wrap(&format!("<div align=\"{value}\">"), "</div>", window, cx);
    }

    fn persist_doc(&mut self, doc: NoteDoc, cx: &mut Context<Self>) {
        let id = self.note_id;
        self.model.update(cx, |m, cx| {
            m.mutate_note(id, |note| {
                note.doc = doc;
                note.updated = now_ole();
            });
            cx.notify();
        });
        cx.notify();
    }

    fn font_size(&self, cx: &App) -> u32 {
        let m = self.model.read(cx);
        m.note(self.note_id)
            .map(|n| {
                n.doc
                    .effective_font(
                        &m.settings.default_note.font_name,
                        m.settings.default_note.font_size,
                    )
                    .1
            })
            .unwrap_or(15)
    }

    fn font_name(&self, cx: &App) -> String {
        let m = self.model.read(cx);
        m.note(self.note_id)
            .map(|n| {
                n.doc
                    .effective_font(
                        &m.settings.default_note.font_name,
                        m.settings.default_note.font_size,
                    )
                    .0
            })
            .unwrap_or_else(|| "Segoe UI".into())
    }

    fn replace_source(
        &mut self,
        new_text: String,
        sel: (usize, usize),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.last_text = new_text.clone();
        self.editor.update(cx, |state, cx| {
            state.set_value(new_text.clone(), window, cx);
            state.set_selected_range(sel.0..sel.1, cx);
        });
        let id = self.note_id;
        let mut doc = self
            .model
            .read(cx)
            .note(id)
            .map(|n| n.doc.clone())
            .unwrap_or_default();
        doc.text = new_text;
        self.persist_doc(doc, cx);
        window.focus(&self.focus_handle, cx);
    }

    fn expand(&mut self, window: &mut Window) {
        if !self.collapsed {
            return;
        }
        let height = if self.expanded_height > 80 {
            self.expanded_height
        } else {
            240
        };
        self.collapsed = false;
        window.resize(gpui_kit::Size {
            width: window.bounds().size.width,
            height: px(height as f32),
        });
    }

    fn markdown_wrap(
        &mut self,
        prefix: &str,
        suffix: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = self.editor.read(cx).value().to_string();
        let sel = self.editor.read(cx).selected_range();
        let start = sel.start.min(text.len());
        let end = sel.end.min(text.len());
        if start != end && text[..start].ends_with(prefix) && text[end..].starts_with(suffix) {
            let new = format!(
                "{}{}{}",
                &text[..start - prefix.len()],
                &text[start..end],
                &text[end + suffix.len()..]
            );
            self.replace_source(new, (start - prefix.len(), end - prefix.len()), window, cx);
            return;
        }
        let selected = &text[start..end];
        let new = format!(
            "{}{}{}{}{}",
            &text[..start],
            prefix,
            selected,
            suffix,
            &text[end..]
        );
        let inner_start = start + prefix.len();
        let inner_end = inner_start + selected.len();
        self.replace_source(new, (inner_start, inner_end), window, cx);
    }

    fn markdown_line_prefix(&mut self, prefix: &str, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.editor.read(cx).value().to_string();
        let sel = self.editor.read(cx).selected_range();
        let line_start = text[..sel.start.min(text.len())]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let line_end = text[line_start..]
            .find('\n')
            .map(|i| line_start + i)
            .unwrap_or(text.len());
        let line = &text[line_start..line_end];
        let new_line = if line.starts_with(prefix) {
            line[prefix.len()..].to_string()
        } else {
            format!("{prefix}{line}")
        };
        let new = format!("{}{}{}", &text[..line_start], new_line, &text[line_end..]);
        let delta = new_line.len() as isize - line.len() as isize;
        let caret = (sel.start as isize + delta).max(0) as usize;
        self.replace_source(new, (caret, caret), window, cx);
    }

    fn markdown_clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.editor.read(cx).value().to_string();
        let sel = self.editor.read(cx).selected_range();
        let (start, end) = if sel.start != sel.end {
            (sel.start, sel.end)
        } else {
            let line_start = text[..sel.start.min(text.len())]
                .rfind('\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            let line_end = text[line_start..]
                .find('\n')
                .map(|i| line_start + i)
                .unwrap_or(text.len());
            (line_start, line_end)
        };
        let mut selected = text[start..end].to_string();
        for marker in [
            "**", "__", "~~", "*", "_", "<u>", "</u>", "<mark>", "</mark>", "`",
        ] {
            selected = selected.replace(marker, "");
        }
        let new = format!("{}{}{}", &text[..start], selected, &text[end..]);
        self.replace_source(new, (start, start + selected.len()), window, cx);
    }

    fn markdown_color_dialog(
        &mut self,
        highlight: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = self.note_id;
        let model = self.model.clone();
        let editor = self.editor.clone();
        let text = self.editor.read(cx).value().to_string();
        let sel = self.editor.read(cx).selected_range();
        let (start, end) = if sel.start != sel.end {
            (sel.start.min(text.len()), sel.end.min(text.len()))
        } else {
            let line_start = text[..sel.start.min(text.len())]
                .rfind('\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            let line_end = text[line_start..]
                .find('\n')
                .map(|i| line_start + i)
                .unwrap_or(text.len());
            (line_start, line_end)
        };
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let mut choices = if highlight {
                vec![
                    0xFFF3A0, 0xFFD6A5, 0xFFB3BA, 0xBAE1FF, 0xBAFFC9, 0xE0BBE4, 0xFFDFBA, 0xD4F0F0,
                ]
            } else {
                vec![
                    0x1A1A1A, 0xEEEEEE, 0xC0392B, 0xE67E22, 0xF1C40F, 0x27AE60, 0x2980B9, 0x8E44AD,
                ]
            };
            let mut row = h_flex().gap_1();
            for (i, c) in choices.drain(..).enumerate() {
                let model = model.clone();
                let editor = editor.clone();
                let text = text.clone();
                row = row.child(
                    Button::new(format!("md-color-{highlight}-{i}"))
                        .w(px(26.))
                        .h(px(26.))
                        .p_0()
                        .bg(theme::rgba(c))
                        .on_click(move |_, window: &mut Window, cx: &mut App| {
                            let (open, close) = if highlight {
                                ("<mark>".to_string(), "</mark>".to_string())
                            } else {
                                (
                                    format!("<span style=\"color:#{c:06X}\">"),
                                    "</span>".to_string(),
                                )
                            };
                            let selected = &text[start..end];
                            let new = format!(
                                "{}{}{}{}{}",
                                &text[..start],
                                open,
                                selected,
                                close,
                                &text[end..]
                            );
                            let inner_start = start + open.len();
                            let inner_end = inner_start + selected.len();
                            editor.update(cx, |state, cx| {
                                state.set_value(new.clone(), window, cx);
                                state.set_selected_range(inner_start..inner_end, cx);
                            });
                            model.update(cx, |m, cx| {
                                let mut doc = m.note(id).map(|n| n.doc.clone()).unwrap_or_default();
                                doc.text = new.clone();
                                m.set_doc(id, doc, None);
                                cx.notify();
                            });
                            window.close_dialog(cx);
                        }),
                );
            }
            dialog
                .title(if highlight { "Highlight" } else { "Font Color" })
                .child(row)
        });
    }

    fn color_dialog(&mut self, highlight: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.markdown_color_dialog(highlight, window, cx);
    }

    fn find_replace_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let model = self.model.clone();
        let editor = self.editor.clone();
        let focus = self.focus_handle.clone();
        let find_input = self.find_input.clone();
        let replace_input = self.replace_input.clone();
        let id = self.note_id;
        let match_case = self.match_case;
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let m_find = model.clone();
            let e_find = editor.clone();
            let f_find = focus.clone();
            let fi = find_input.clone();
            let find_next = Button::new("find-next")
                .small()
                .label("Find Next")
                .on_click(move |_, window: &mut Window, cx: &mut App| {
                    let needle = fi.read(cx).value().to_string();
                    if needle.is_empty() {
                        return;
                    }
                    let hay = e_find.read(cx).value().to_string();
                    let pos = e_find.read(cx).selected_range().end.min(hay.len());
                    let hay_cmp = if match_case {
                        hay.clone()
                    } else {
                        hay.to_lowercase()
                    };
                    let needle_cmp = if match_case {
                        needle.clone()
                    } else {
                        needle.to_lowercase()
                    };
                    let start = if let Some(rel) = hay_cmp[pos..].find(&needle_cmp) {
                        Some(pos + rel)
                    } else {
                        hay_cmp.find(&needle_cmp)
                    };
                    if let Some(start) = start {
                        e_find.update(cx, |state, cx| {
                            state.set_selected_range(start..start + needle.len(), cx)
                        });
                        window.focus(&f_find, cx);
                    }
                });
            let m_replace_state = m_find.clone();
            let e_replace = editor.clone();
            let f_replace = focus.clone();
            let fi2 = find_input.clone();
            let ri = replace_input.clone();
            let replace = Button::new("replace-one")
                .small()
                .label("Replace")
                .on_click(move |_, window: &mut Window, cx: &mut App| {
                    let needle = fi2.read(cx).value().to_string();
                    let repl = ri.read(cx).value().to_string();
                    let sel = e_replace.read(cx).selected_range();
                    let hay = e_replace.read(cx).value().to_string();
                    let ok = if match_case {
                        hay.get(sel.clone()).map(|s| s == needle)
                    } else {
                        hay.get(sel.clone())
                            .map(|s| s.eq_ignore_ascii_case(&needle))
                    };
                    if ok == Some(true) {
                        e_replace.update(cx, |state, cx| state.replace(repl, window, cx));
                        let new_text = e_replace.read(cx).value().to_string();
                        m_replace_state.update(cx, |m, cx| {
                            let mut doc = m.note(id).map(|n| n.doc.clone()).unwrap_or_default();
                            doc.text = new_text.clone();
                            m.set_doc(id, doc, None);
                            cx.notify();
                        });
                        window.focus(&f_replace, cx);
                    }
                });
            let m_all_state = m_find.clone();
            let e_all = editor.clone();
            let f_all = focus.clone();
            let fi3 = find_input.clone();
            let ri3 = replace_input.clone();
            let replace_all = Button::new("replace-all")
                .small()
                .label("Replace All")
                .on_click(move |_, window: &mut Window, cx: &mut App| {
                    let needle = fi3.read(cx).value().to_string();
                    let repl = ri3.read(cx).value().to_string();
                    if needle.is_empty() {
                        return;
                    }
                    let hay = e_all.read(cx).value().to_string();
                    let result = if match_case {
                        hay.replace(&needle, &repl)
                    } else {
                        let mut out = String::new();
                        let lower = hay.to_lowercase();
                        let n = needle.to_lowercase();
                        let mut last = 0;
                        while let Some(p) = lower[last..].find(&n) {
                            out.push_str(&hay[last..last + p]);
                            out.push_str(&repl);
                            last += p + n.len();
                        }
                        out.push_str(&hay[last..]);
                        out
                    };
                    e_all.update(cx, |state, cx| state.replace_all(result, window, cx));
                    let new_text = e_all.read(cx).value().to_string();
                    m_all_state.update(cx, |m, cx| {
                        let mut doc = m.note(id).map(|n| n.doc.clone()).unwrap_or_default();
                        doc.text = new_text.clone();
                        m.set_doc(id, doc, None);
                        cx.notify();
                    });
                    window.focus(&f_all, cx);
                });
            let fi4 = find_input.clone();
            dialog
                .title("Find & Replace")
                .w(px(460.))
                .child(
                    v_flex()
                        .gap_2()
                        .child(Input::new(&fi4))
                        .child(Input::new(&replace_input)),
                )
                .child(
                    h_flex()
                        .gap_1()
                        .child(find_next)
                        .child(replace)
                        .child(replace_all),
                )
        });
        self.match_case = match_case;
    }
}

impl Render for NoteView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let note = self.note(cx);
        let (title, stored_color, opacity, _starred, locked, _alarm, _aot) = note
            .as_ref()
            .map(|n| {
                (
                    n.title.clone(),
                    n.color,
                    n.opacity,
                    n.starred,
                    n.locked,
                    n.alarm,
                    n.aot,
                )
            })
            .unwrap_or(("Note".into(), 0xFFF3AB, 100, false, false, 0.0, false));
        let dark_notes = {
            let m = self.model.read(cx);
            matches!(m.settings.note_mode, NoteMode::Dark)
                || (matches!(m.settings.note_mode, NoteMode::Auto)
                    && (matches!(m.settings.app_mode, AppMode::Dark)
                        || (matches!(m.settings.app_mode, AppMode::System)
                            && crate::system_prefers_dark())))
        };
        let color = if dark_notes {
            theme::note_color_dark(stored_color)
        } else {
            stored_color
        };
        let bg = theme::hsla(color, opacity as f32 / 100.0);
        let fg = theme::hsla(theme::auto_font_color(color), 0.92);
        let divider = fg.opacity(0.08);
        let id = self.note_id;

        let title_area = div()
            .id("note-title-drag")
            .flex()
            .flex_1()
            .min_w(px(20.))
            .h_full()
            .items_center()
            .justify_center()
            .text_size(px(13.))
            .text_color(fg.opacity(0.85))
            .on_mouse_down(MouseButton::Left, |event, window, _| {
                if event.click_count == 1 {
                    window.start_window_move();
                }
            })
            .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                if event.click_count() >= 2 {
                    this.edit_title(window, cx);
                }
            }))
            .child(SharedString::from(title.clone()));

        let title_panel = h_flex()
            .h(px(32.))
            .px(px(2.))
            .items_center()
            .rounded_tl(px(8.))
            .rounded_tr(px(8.))
            .opacity(if self.hovered || self.collapsed {
                1.0
            } else {
                0.0
            })
            .when(crate::app::chrome::controls_on_left(), |b| {
                b.child(note_controls(id, fg, cx))
            })
            .child(title_area)
            .when(self.collapsed, |b| {
                b.child(
                    Button::new(("note-expand", id as u64))
                        .ghost()
                        .xsmall()
                        .text_color(fg)
                        .icon(IconName::Maximize)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.expand(window);
                            cx.notify();
                        })),
                )
            })
            .child(
                Button::new(("note-preview", id as u64))
                    .ghost()
                    .small()
                    .w(px(28.))
                    .h(px(28.))
                    .p_0()
                    .text_color(fg)
                    .icon(IconName::Eye)
                    .when(self.preview, |b| b.text_color(cx.theme().primary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.preview = !this.preview;
                        cx.notify();
                    })),
            )
            .child({
                let menu_model = self.model.clone();
                let weak_view = cx.entity().downgrade();
                let self_preview = self.preview;
                let self_collapsed = self.collapsed;
                Button::new(("menu", id as u64))
                    .ghost()
                    .small()
                    .w(px(28.))
                    .h(px(28.))
                    .p_0()
                    .text_color(fg)
                    .icon(IconName::Ellipsis)
                    .dropdown_menu(move |menu, window, cx| {
                        build_note_menu(
                            menu,
                            id,
                            menu_model.clone(),
                            weak_view.clone(),
                            self_preview,
                            self_collapsed,
                            window,
                            cx,
                        )
                    })
            })
            .when(!crate::app::chrome::controls_on_left(), |b| {
                b.child(note_controls(id, fg, cx))
            });

        let font_size = self.font_size(cx);
        let font_name = self.font_name(cx);
        let doc_align = self
            .model
            .read(cx)
            .note(id)
            .map(|n| n.doc.align)
            .unwrap_or(Align::Left);
        let window_width = window.bounds().size.width.as_f32();
        let core_tools = h_flex()
            .gap_0p5()
            .items_center()
            .child(fmt_button(
                "bold",
                IconName::Bold,
                fg,
                cx.listener(|this, _, window, cx| this.format_bool(0, window, cx)),
            ))
            .child(fmt_button(
                "italic",
                IconName::Italic,
                fg,
                cx.listener(|this, _, window, cx| this.format_bool(1, window, cx)),
            ))
            .child(fmt_button(
                "underline",
                IconName::Underline,
                fg,
                cx.listener(|this, _, window, cx| this.format_bool(2, window, cx)),
            ))
            .child(fmt_button(
                "strike",
                IconName::Strikethrough,
                fg,
                cx.listener(|this, _, window, cx| this.format_bool(3, window, cx)),
            ))
            .child(fmt_button(
                "highlight",
                IconName::Highlighter,
                fg,
                cx.listener(|this, _, window, cx| this.color_dialog(true, window, cx)),
            ))
            .child(fmt_button(
                "bullet",
                IconName::List,
                fg,
                cx.listener(|this, _, window, cx| this.toggle_line_prefix(0, window, cx)),
            ))
            .child(fmt_button(
                "checkbox",
                IconName::ListChecks,
                fg,
                cx.listener(|this, _, window, cx| this.toggle_line_prefix(2, window, cx)),
            ));

        let extra_tools = h_flex()
            .gap_0p5()
            .items_center()
            .when(window_width > 340.0, |d| {
                d.child(fmt_button(
                    "numbered",
                    IconName::ListOrdered,
                    fg,
                    cx.listener(|this, _, window, cx| this.toggle_line_prefix(1, window, cx)),
                ))
                .child(fmt_button(
                    "align-left",
                    IconName::TextAlignStart,
                    fg,
                    cx.listener(|this, _, window, cx| this.set_note_align(Align::Left, window, cx)),
                ))
                .child(fmt_button(
                    "align-center",
                    IconName::TextAlignCenter,
                    fg,
                    cx.listener(|this, _, window, cx| {
                        this.set_note_align(Align::Center, window, cx)
                    }),
                ))
                .child(fmt_button(
                    "align-right",
                    IconName::TextAlignEnd,
                    fg,
                    cx.listener(|this, _, window, cx| {
                        this.set_note_align(Align::Right, window, cx)
                    }),
                ))
                .child(fmt_button(
                    "clear",
                    IconName::Eraser,
                    fg,
                    cx.listener(|this, _, window, cx| this.clear_formatting(window, cx)),
                ))
            });

        let format_row = h_flex()
            .w_full()
            .h(px(30.))
            .pl(px(6.))
            .pr(px(18.))
            .items_center()
            .child(core_tools)
            .child(extra_tools)
            .child(div().flex_1())
            .child(
                Button::new("font-size")
                    .ghost()
                    .xsmall()
                    .text_color(fg)
                    .label(format!("{font_size}"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        let id = this.note_id;
                        let mut doc = this
                            .model
                            .read(cx)
                            .note(id)
                            .map(|n| n.doc.clone())
                            .unwrap_or_default();
                        let (_, current) = doc.effective_font("Segoe UI", 15);
                        let sizes = [
                            8u32, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 36, 48, 72,
                        ];
                        let idx = sizes.iter().position(|s| *s == current).unwrap_or(4);
                        doc.font_size = sizes[(idx + 1) % sizes.len()];
                        if doc.font_name.is_empty() {
                            doc.font_name = this.font_name(cx);
                        }
                        this.persist_doc(doc, cx);
                    })),
            );

        let toolbar = v_flex()
            .w_full()
            .border_t_1()
            .border_color(divider)
            .child(format_row);

        let editor = div()
            .flex()
            .flex_1()
            .min_h(px(0.))
            .px(px(11.))
            .pt(px(6.))
            .pb(px(2.))
            .child(if self.preview || locked {
                div()
                    .id("md-preview-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .px(px(4.))
                    .py_1()
                    .child(
                        TextView::markdown(
                            ("md-preview", id as u64),
                            self.model
                                .read(cx)
                                .note(id)
                                .map(|n| n.doc.text.clone())
                                .unwrap_or_default(),
                        )
                        .text_color(fg)
                        .text_size(px(font_size as f32))
                        .selectable(false),
                    )
                    .into_any_element()
            } else {
                Textarea::new(&self.editor)
                    .appearance(false)
                    .bordered(false)
                    .with_size(gpui_kit::component::Size::XSmall)
                    .readonly(locked)
                    .text_size(px(font_size as f32))
                    .text_color(fg)
                    .font_family(SharedString::from(font_name))
                    .text_align(match doc_align {
                        Align::Left => gpui_kit::TextAlign::Left,
                        Align::Center => gpui_kit::TextAlign::Center,
                        Align::Right => gpui_kit::TextAlign::Right,
                    })
                    .size_full()
                    .into_any_element()
            });

        v_flex()
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
                cx.listener(|this, _: &crate::app::actions::OpenExplorer, _, cx| {
                    windows::open_explorer_window(cx, this.model.clone());
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    if this.collapsed {
                        return;
                    }
                    let bounds = window.bounds();
                    let (left, top) = (
                        bounds.origin.x.as_f32() as i32,
                        bounds.origin.y.as_f32() as i32,
                    );
                    let (width, height) = (
                        bounds.size.width.as_f32() as i32,
                        bounds.size.height.as_f32() as i32,
                    );
                    let id = this.note_id;
                    this.model.update(cx, |m, cx| {
                        m.mutate_note(id, |n| {
                            n.left = left;
                            n.top = top;
                            n.width = width;
                            n.height = height;
                        });
                        cx.notify();
                    });
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
                    if let Some(pos) = ids.iter().position(|x| *x == this.note_id) {
                        if let Some(next) = ids.get((pos + 1) % ids.len()) {
                            windows::open_note_window(cx, this.model.clone(), *next);
                        }
                    }
                }),
            )
            .child(
                v_flex()
                    .id(("note-card", self.note_id as u64))
                    .size_full()
                    .rounded(px(8.))
                    .overflow_hidden()
                    .bg(bg)
                    .shadow_md()
                    .on_hover(cx.listener(|this, hovered, _, cx| {
                        if this.hovered != *hovered {
                            this.hovered = *hovered;
                            cx.notify();
                        }
                    }))
                    .child(title_panel)
                    .when(!self.collapsed, |d| {
                        d.child(editor)
                            .when(!locked && !self.preview, |d| d.child(toolbar))
                    })
                    .child(
                        div()
                            .absolute()
                            .right_0()
                            .bottom_0()
                            .w(px(16.))
                            .h(px(16.))
                            .cursor(gpui_kit::CursorStyle::ResizeUpLeftDownRight)
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_resize(gpui_kit::ResizeEdge::BottomRight);
                            }),
                    ),
            )
    }
}

fn build_note_menu(
    menu: PopupMenu,
    id: i64,
    model: Entity<AppModel>,
    view: gpui_kit::WeakEntity<NoteView>,
    preview: bool,
    collapsed: bool,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let Some(note) = model.read(cx).note(id).cloned() else {
        return menu;
    };
    let notebooks = model.read(cx).notebooks_with_default();
    let current_size = (1..=6)
        .min_by_key(|size| (note_size_dims(*size).0 - note.width).abs())
        .unwrap_or(3);

    let m = model.clone();
    let new_note = PopupMenuItem::new("New note")
        .icon(IconName::Plus)
        .on_click(move |_, _, cx| {
            let note_id = m.update(cx, |m, _| m.create_note(None, None));
            windows::open_note_window(cx, m.clone(), note_id);
        });

    let m = model.clone();
    let duplicate = PopupMenuItem::new("Duplicate")
        .icon(IconName::Copy)
        .on_click(move |_, _, cx| {
            if let Some(note_id) = m.update(cx, |m, _| m.duplicate_note(id)) {
                windows::open_note_window(cx, m.clone(), note_id);
            }
        });

    let v = view.clone();
    let preview_item = PopupMenuItem::new("Preview")
        .icon(IconName::Eye)
        .checked(preview)
        .on_click(move |_, _, cx| {
            let _ = v.update(cx, |this, cx| {
                this.preview = !this.preview;
                cx.notify();
            });
        });

    let v = view.clone();
    let find = PopupMenuItem::new("Find…")
        .icon(IconName::Search)
        .on_click(move |_, window, cx| {
            let v = v.clone();
            window.defer(cx, move |window, cx| {
                let _ = v.update(cx, |this, cx| this.find_replace_dialog(window, cx));
            });
        });

    let v = view.clone();
    let collapse = PopupMenuItem::new(if collapsed {
        "Expand note"
    } else {
        "Collapse note"
    })
    .icon(if collapsed {
        IconName::Maximize
    } else {
        IconName::Minus
    })
    .on_click(move |_, window, cx| {
        let v = v.clone();
        let restored = v
            .read_with(cx, |this, _| {
                if this.expanded_height > 80 {
                    this.expanded_height
                } else {
                    240
                }
            })
            .unwrap_or(240);
        let target = if collapsed {
            let _ = v.update(cx, |this, cx| {
                this.collapsed = false;
                cx.notify();
            });
            restored
        } else {
            let current = window.bounds().size.height.as_f32() as i32;
            let _ = v.update(cx, |this, cx| {
                this.collapsed = true;
                this.expanded_height = current;
                cx.notify();
            });
            32
        };
        window.resize(gpui_kit::Size {
            width: window.bounds().size.width,
            height: px(target as f32),
        });
    });

    let m = model.clone();
    let v = view.clone();
    let star_label = if note.starred {
        "Remove star"
    } else {
        "Add star"
    };
    let star = PopupMenuItem::new(star_label)
        .icon(IconName::Star)
        .checked(note.starred)
        .on_click(move |_, _, cx| {
            m.update(cx, |m, _| m.mutate_note(id, |n| n.starred = !n.starred));
            let _ = v.update(cx, |_, cx| cx.notify());
        });

    let m = model.clone();
    let v = view.clone();
    let lock_label = if note.locked {
        "Unlock note"
    } else {
        "Lock note"
    };
    let lock = PopupMenuItem::new(lock_label)
        .icon(IconName::Lock)
        .checked(note.locked)
        .on_click(move |_, _, cx| {
            m.update(cx, |m, _| m.mutate_note(id, |n| n.locked = !n.locked));
            let _ = v.update(cx, |_, cx| cx.notify());
        });

    let m = model.clone();
    let v = view.clone();
    let always_on_top = PopupMenuItem::new("Always on top")
        .icon(IconName::Pin)
        .checked(note.aot)
        .on_click(move |_, _, cx| {
            m.update(cx, |m, _| m.mutate_note(id, |n| n.aot = !n.aot));
            let _ = v.update(cx, |_, cx| cx.notify());
        });

    let m = model.clone();
    let v = view.clone();
    let initial_color = note.color;
    let color = PopupMenuItem::new("Color…")
        .icon(IconName::Palette)
        .on_click(move |_, window, cx| {
            let m = m.clone();
            let v = v.clone();
            window.defer(cx, move |window, cx| {
                super::color_wheel::open_color_wheel(
                    window,
                    cx,
                    initial_color,
                    move |value, cx| {
                        m.update(cx, |m, _| m.update_note_color(id, value));
                        let _ = v.update(cx, |_, cx| cx.notify());
                    },
                );
            });
        });

    let v = view.clone();
    let font_color = PopupMenuItem::new("Font color…")
        .icon(IconName::ALargeSmall)
        .on_click(move |_, window, cx| {
            let v = v.clone();
            window.defer(cx, move |window, cx| {
                let _ = v.update(cx, |this, cx| this.markdown_color_dialog(false, window, cx));
            });
        });

    let opacity = note.opacity;
    let m = model.clone();
    let opacity_menu = PopupMenu::build(window, cx, move |mut sub, _window, _cx| {
        for pct in [100u8, 90, 80, 70, 60, 50, 40, 30, 20, 10] {
            let m = m.clone();
            sub = sub.item(
                PopupMenuItem::new(format!("{pct}%"))
                    .checked(pct == opacity)
                    .on_click(move |_, _, cx| {
                        m.update(cx, |m, cx| {
                            m.mutate_note(id, |n| n.opacity = pct);
                            cx.notify();
                        });
                    }),
            );
        }
        sub
    });
    let opacity_item = PopupMenuItem::submenu("Opacity", opacity_menu).icon(IconName::Circle);

    let m = model.clone();
    let size_menu = PopupMenu::build(window, cx, move |mut sub, _window, _cx| {
        for (index, label) in ["Tiny", "Small", "Medium", "Large", "X-Large", "Huge"]
            .into_iter()
            .enumerate()
        {
            let size = index as i32 + 1;
            let m = m.clone();
            sub = sub.item(
                PopupMenuItem::new(label)
                    .checked(size == current_size)
                    .on_click(move |_, _, cx| {
                        let (width, height) = note_size_dims(size);
                        m.update(cx, |m, cx| {
                            m.mutate_note(id, |n| {
                                n.width = width;
                                n.height = height;
                            });
                            cx.notify();
                        });
                        windows::resize_note_window(cx, m.clone(), id);
                    }),
            );
        }
        sub
    });
    let size_item = PopupMenuItem::submenu("Size", size_menu).icon(IconName::Scaling);

    let m = model.clone();
    let current_notebook = note.notebook.clone();
    let notebook_menu =
        PopupMenu::build(window, cx, move |mut sub, _window, _cx| {
            for name in notebooks {
                let m = m.clone();
                let selected = name == current_notebook;
                let target = name.clone();
                sub = sub.item(PopupMenuItem::new(name).checked(selected).on_click(
                    move |_, _, cx| {
                        let target = target.clone();
                        m.update(cx, |m, cx| {
                            m.move_note(id, &target);
                            cx.notify();
                        });
                    },
                ));
            }
            sub
        });
    let notebook_item = PopupMenuItem::submenu("Notebook", notebook_menu).icon(IconName::Folder);

    let m = model.clone();
    let fit = PopupMenuItem::new("Fit to content")
        .icon(IconName::Move)
        .on_click(move |_, _, cx| {
            m.update(cx, |m, cx| {
                m.mutate_note(id, |n| {
                    let lines = n.doc.text.split('\n').count().max(1) as i32;
                    n.height = (64 + lines * 24).clamp(240, 1000);
                });
                cx.notify();
            });
            windows::resize_note_window(cx, m.clone(), id);
        });

    let m = model.clone();
    let v = view.clone();
    let hide_others = PopupMenuItem::new("Hide others")
        .icon(IconName::EyeOff)
        .on_click(move |_, _, cx| {
            let handles: Vec<_> = m
                .read(cx)
                .note_windows
                .iter()
                .filter(|(nid, _)| **nid != id)
                .map(|(_, handle)| *handle)
                .collect();
            for handle in handles {
                let _ = handle.update(cx, |_, window, _| window.remove_window());
            }
            m.update(cx, |m, _| {
                for nid in m.note_windows.keys().copied().collect::<Vec<_>>() {
                    if nid != id {
                        if let Some(n) = m.notes.iter_mut().find(|n| n.id == nid) {
                            n.visible = false;
                        }
                    }
                }
                m.note_windows.retain(|nid, _| *nid == id);
            });
            let _ = v.update(cx, |_, cx| cx.notify());
        });

    let m = model.clone();
    let export = PopupMenuItem::new("Export as Markdown…")
        .icon(IconName::Download)
        .on_click(move |_, _, cx| {
            let (text, title) = m
                .read(cx)
                .note(id)
                .map(|n| (n.doc.text.clone(), n.title.clone()))
                .unwrap_or_default();
            let dir = dirs::document_dir().unwrap_or_else(|| std::env::temp_dir());
            let path = dir.join(format!("{}.md", crate::util::safe_filename(&title)));
            let _ = std::fs::write(&path, text);
            m.update(cx, |m, _| {
                m.toast_error(format!("Exported to {}", path.display()))
            });
        });

    let m = model.clone();
    let print = PopupMenuItem::new("Print…")
        .icon(IconName::Printer)
        .on_click(move |_, _, cx| {
            let (title, text) = m
                .read(cx)
                .note(id)
                .map(|n| (n.title.clone(), n.doc.text.clone()))
                .unwrap_or_default();
            if !crate::util::print_text(&title, &text) {
                m.update(cx, |m, _| {
                    m.toast_error("Printing is not available (lp command missing)")
                });
            }
        });

    let m = model.clone();
    let locate = PopupMenuItem::new("Locate in Explorer")
        .icon(IconName::Search)
        .on_click(move |_, _, cx| {
            m.update(cx, |m, _| {
                m.selection = vec![id];
                m.explorer_page = super::model::ExplorerPage::AllNotes;
            });
            windows::open_explorer_window(cx, m.clone());
        });

    let m = model.clone();
    let hide = PopupMenuItem::new("Hide note")
        .icon(IconName::EyeOff)
        .on_click(move |_, window, cx| {
            m.update(cx, |m, _| {
                m.mutate_note(id, |n| n.visible = false);
            });
            window.remove_window();
        });

    let m = model.clone();
    let delete = PopupMenuItem::new("Delete note")
        .icon(IconName::Trash)
        .on_click(move |_, window, cx| {
            m.update(cx, |m, cx| {
                m.trash_note(id);
                cx.notify();
            });
            window.remove_window();
        });

    // Keep the menu inside the note window; when the note is short the items
    // scroll instead of being clipped by the window bounds.
    let max_h = (window.bounds().size.height.as_f32() - 44.).max(120.);

    menu.min_w(px(230.))
        .max_h(px(max_h))
        .scrollable(true)
        .item(new_note)
        .item(duplicate)
        .item(preview_item)
        .item(find)
        .item(collapse)
        .separator()
        .item(star)
        .item(lock)
        .item(always_on_top)
        .separator()
        .item(color)
        .item(font_color)
        .item(opacity_item)
        .item(size_item)
        .item(notebook_item)
        .separator()
        .item(fit)
        .item(hide_others)
        .separator()
        .item(export)
        .item(print)
        .item(locate)
        .separator()
        .item(hide)
        .item(delete)
}

fn note_controls(id: i64, fg: gpui_kit::Hsla, cx: &mut Context<NoteView>) -> impl IntoElement {
    h_flex().gap_0p5().items_center().child(
        Button::new(("note-close", id as u64))
            .ghost()
            .xsmall()
            .w(px(28.))
            .h(px(28.))
            .p_0()
            .label("✕")
            .text_color(fg)
            .on_click(cx.listener(|this, _, window, cx| {
                let id = this.note_id;
                this.model.update(cx, |m, _| {
                    m.mutate_note(id, |n| n.visible = false);
                });
                window.remove_window();
                cx.notify();
            })),
    )
}

fn fmt_button(
    id: &str,
    icon: IconName,
    fg: gpui_kit::Hsla,
    handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Button {
    Button::new(format!("fmt-{id}"))
        .ghost()
        .xsmall()
        .flex_shrink_0()
        .w(px(28.))
        .h(px(28.))
        .p_0()
        .text_color(fg)
        .icon(icon)
        .on_click(handler)
}
