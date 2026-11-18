use super::model::AppModel;
use super::windows;
use crate::model::*;
use crate::theme;
use gpui_kit::assets::IconName;
use gpui_kit::base::input::{TextDecoration, TextDecorationCollection};
use gpui_kit::component::{
    Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants},
    dialog::{DialogClose, DialogFooter},
    h_flex,
    input::{Editor, EditorState, Input, InputEvent, InputState},
    text::TextView,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, ClickEvent, Context, Corners, Entity, FocusHandle, FontStyle, FontWeight,
    HighlightStyle, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Render,
    SharedString, StatefulInteractiveElement as _, StrikethroughStyle, Styled as _, Subscription,
    UnderlineStyle, Window, div, prelude::FluentBuilder as _, px,
};

pub struct NoteView {
    model: Entity<AppModel>,
    note_id: i64,
    editor: Entity<EditorState>,
    decorations: TextDecorationCollection,
    focus_handle: FocusHandle,
    title_input: Entity<InputState>,
    find_input: Entity<InputState>,
    replace_input: Entity<InputState>,
    match_case: bool,
    collapsed: bool,
    preview: bool,
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
            EditorState::new(window, cx)
                .soft_wrap(true)
                .line_number(false)
                .placeholder("Write in Markdown… **bold**, *italic*, # heading, - list, - [ ] task")
        });
        editor.update(cx, |state, cx| {
            state.set_value(text.clone(), window, cx);
        });
        let decorations = editor.update(cx, |state, cx| {
            state.create_decorations_collection(Vec::new(), cx)
        });
        let focus_handle = editor.read(cx).presentation().focus_handle().clone();
        let title_input = cx.new(|cx| InputState::new(window, cx).placeholder("Title"));
        let find_input = cx.new(|cx| InputState::new(window, cx).placeholder("Find what"));
        let replace_input = cx.new(|cx| InputState::new(window, cx).placeholder("Replace with"));
        window.set_client_corner_radii(Corners::all(px(12.)));
        let sub = cx.subscribe_in(
            &editor,
            window,
            |this, state, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    let text = state.read(cx).value().to_string();
                    if text != this.last_text {
                        let old = this.last_text.clone();
                        this.last_text = text.clone();
                        let id = this.note_id;
                        let mut doc = this
                            .model
                            .read(cx)
                            .note(id)
                            .map(|n| n.doc.clone())
                            .unwrap_or_default();
                        doc.adjust_spans(&old, &text);
                        doc.ensure_paras();
                        this.apply_decorations(&doc, cx);
                        this.model.update(cx, |m, cx| {
                            m.mutate_note(id, |note| {
                                note.doc = doc;
                                note.updated = now_ole();
                            });
                            cx.notify();
                        });
                    }
                }
            },
        );
        let mut this = Self {
            model,
            note_id,
            editor,
            decorations,
            focus_handle,
            title_input,
            find_input,
            replace_input,
            match_case: false,
            collapsed: false,
            preview: std::env::var_os("SSN_PREVIEW").is_some(),
            expanded_height: 0,
            last_text: text,
            _subscriptions: vec![sub],
        };
        this.apply_decorations(&doc, cx);
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

    fn apply_decorations(&mut self, doc: &NoteDoc, cx: &mut Context<Self>) {
        if doc.markdown {
            self.decorations.set(Vec::new(), cx);
        } else {
            self.decorations.set(build_decorations(doc), cx);
        }
    }

    fn selection_or_line(&self, doc: &NoteDoc, cx: &App) -> (usize, usize) {
        let range = self.editor.read(cx).selected_range();
        if range.start != range.end {
            let len = doc.text.len();
            (range.start.min(len), range.end.min(len))
        } else {
            let len = doc.text.len();
            doc.line_range(range.start.min(len))
        }
    }

    fn format_bool(&mut self, which: u8, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.note_id;
        let mut doc = self
            .model
            .read(cx)
            .note(id)
            .map(|n| n.doc.clone())
            .unwrap_or_default();
        if doc.markdown {
            let (p, s) = match which {
                0 => ("**", "**"),
                1 => ("*", "*"),
                2 => ("<u>", "</u>"),
                _ => ("~~", "~~"),
            };
            self.markdown_wrap(p, s, window, cx);
            return;
        }
        let (start, end) = self.selection_or_line(&doc, cx);
        if start >= end {
            return;
        }
        let style = doc.style_at(start);
        let value = match which {
            0 => !style.bold,
            1 => !style.italic,
            2 => !style.underline,
            _ => !style.strike,
        };
        doc.apply_style(start, end, move |s| match which {
            0 => s.bold = value,
            1 => s.italic = value,
            2 => s.underline = value,
            _ => s.strike = value,
        });
        self.persist_doc(doc, cx);
        let _ = window;
        window.focus(&self.focus_handle, cx);
    }

    fn clear_formatting(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.note_id;
        let mut doc = self
            .model
            .read(cx)
            .note(id)
            .map(|n| n.doc.clone())
            .unwrap_or_default();
        if doc.markdown {
            self.markdown_clear(window, cx);
            return;
        }
        let (start, end) = self.selection_or_line(&doc, cx);
        doc.clear_style(start, end);
        self.persist_doc(doc, cx);
    }

    fn toggle_line_prefix(&mut self, kind: u8, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.note_id;
        let mut doc = self
            .model
            .read(cx)
            .note(id)
            .map(|n| n.doc.clone())
            .unwrap_or_default();
        if doc.markdown {
            let prefix = match kind {
                0 => "- ",
                1 => "1. ",
                _ => "- [ ] ",
            };
            self.markdown_line_prefix(prefix, window, cx);
            return;
        }
        let (start, _) = self.selection_or_line(&doc, cx);
        let line_start = doc.line_start(doc.line_index(start));
        let line_end = doc.line_end(doc.line_index(start));
        let line = doc.text[line_start..line_end].to_string();
        let prefixes = ["• ", "1. ", "☐ ", "☑ "];
        let already = prefixes.iter().any(|p| line.starts_with(p));
        let mut new_line = line.clone();
        for p in prefixes {
            if let Some(rest) = line.strip_prefix(p) {
                new_line = rest.to_string();
            }
        }
        if !already {
            new_line = match kind {
                0 => format!("• {new_line}"),
                1 => format!("1. {new_line}"),
                _ => format!("☐ {new_line}"),
            };
        }
        let old = doc.text.clone();
        doc.text.replace_range(line_start..line_end, &new_line);
        let new_text = doc.text.clone();
        doc.adjust_spans(&old, &new_text);
        self.persist_doc(doc, cx);
    }

    fn set_note_align(&mut self, align: Align, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.note_id;
        let mut doc = self
            .model
            .read(cx)
            .note(id)
            .map(|n| n.doc.clone())
            .unwrap_or_default();
        if doc.markdown {
            let value = match align {
                Align::Left => "left",
                Align::Center => "center",
                Align::Right => "right",
            };
            self.markdown_wrap(&format!("<div align=\"{value}\">"), "</div>", window, cx);
            return;
        }
        doc.align = align;
        self.persist_doc(doc, cx);
    }

    fn persist_doc(&mut self, doc: NoteDoc, cx: &mut Context<Self>) {
        let id = self.note_id;
        self.apply_decorations(&doc, cx);
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
        let old = self.editor.read(cx).value().to_string();
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
        doc.adjust_spans(&old, &new_text);
        doc.markdown = true;
        self.persist_doc(doc, cx);
        window.focus(&self.focus_handle, cx);
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
        let id = self.note_id;
        let model = self.model.clone();
        let decorations = self.decorations.clone();
        let doc = self
            .model
            .read(cx)
            .note(id)
            .map(|n| n.doc.clone())
            .unwrap_or_default();
        if doc.markdown {
            self.markdown_color_dialog(highlight, window, cx);
            return;
        }
        let (start, end) = self.selection_or_line(&doc, cx);
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let decorations = decorations.clone();
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
            for (i, c) in choices.drain(..).chain(std::iter::once(0)).enumerate() {
                let model = model.clone();
                let dec = decorations.clone();
                row = row.child(
                    Button::new(format!("color-choice-{highlight}-{i}"))
                        .w(px(26.))
                        .h(px(26.))
                        .p_0()
                        .when(c != 0, |b| b.bg(theme::rgba(c)))
                        .when(c == 0, |b| b.label("×"))
                        .on_click(move |_, window: &mut Window, cx: &mut App| {
                            let value = if c == 0 { None } else { Some(c) };
                            let decorations = dec.clone();
                            model.update(cx, |m, cx| {
                                let mut doc = m.note(id).map(|n| n.doc.clone()).unwrap_or_default();
                                doc.apply_style(start, end, move |s| {
                                    if highlight {
                                        s.highlight = value;
                                    } else {
                                        s.color = value;
                                    }
                                });
                                decorations.set(build_decorations(&doc), cx);
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

    fn find_replace_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let model = self.model.clone();
        let editor = self.editor.clone();
        let focus = self.focus_handle.clone();
        let find_input = self.find_input.clone();
        let replace_input = self.replace_input.clone();
        let decorations = self.decorations.clone();
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
            let dec_replace = decorations.clone();
            let dec_all = decorations.clone();
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
                        let decorations = dec_replace.clone();
                        m_replace_state.update(cx, |m, cx| {
                            let mut doc = m.note(id).map(|n| n.doc.clone()).unwrap_or_default();
                            doc.adjust_spans(&hay, &new_text);
                            decorations.set(build_decorations(&doc), cx);
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
                    let decorations = dec_all.clone();
                    m_all_state.update(cx, |m, cx| {
                        let mut doc = m.note(id).map(|n| n.doc.clone()).unwrap_or_default();
                        doc.adjust_spans(&hay, &new_text);
                        decorations.set(build_decorations(&doc), cx);
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

    fn show_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(note) = self.note(cx) else { return };
        let model = self.model.clone();
        let id = self.note_id;
        let starred = note.starred;
        let locked = note.locked;
        let aot = note.aot;
        let _color = note.color;
        let opacity = note.opacity;
        let title = note.title.clone();
        let notebook = note.notebook.clone();
        let trashed = note.is_trashed();
        let palette = self.model.read(cx).settings.note_color_pal.clone();
        let notebooks = self.model.read(cx).notebooks_with_default();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let m_new = model.clone();
            let new_note = Button::new("mn-new")
                .small()
                .icon(IconName::Plus)
                .label("New Note")
                .on_click(move |_, _, cx: &mut App| {
                    let id = m_new.update(cx, |m, _| m.create_note(None, None));
                    windows::open_note_window(cx, m_new.clone(), id);
                });
            let m_dup = model.clone();
            let duplicate = Button::new("mn-dup")
                .small()
                .icon(IconName::Copy)
                .label("Duplicate")
                .on_click(move |_, _, cx: &mut App| {
                    if let Some(new_id) = m_dup.update(cx, |m, _| m.duplicate_note(id)) {
                        windows::open_note_window(cx, m_dup.clone(), new_id);
                    }
                });
            let m_star = model.clone();
            let star = Button::new("mn-star")
                .small()
                .icon(IconName::Star)
                .label(if starred { "Remove star" } else { "Add star" })
                .on_click(move |_, _, cx: &mut App| {
                    m_star.update(cx, |m, cx| {
                        m.mutate_note(id, |n| n.starred = !n.starred);
                        cx.notify();
                    });
                });
            let m_lock = model.clone();
            let lock = Button::new("mn-lock")
                .small()
                .icon(IconName::Lock)
                .label(if locked { "Unlock note" } else { "Lock note" })
                .on_click(move |_, _, cx: &mut App| {
                    m_lock.update(cx, |m, cx| {
                        m.mutate_note(id, |n| n.locked = !n.locked);
                        cx.notify();
                    });
                });
            let m_aot = model.clone();
            let aot_button = Button::new("mn-aot")
                .small()
                .icon(IconName::Pin)
                .label(if aot {
                    "Always on top: On"
                } else {
                    "Always on top: Off"
                })
                .on_click(move |_, _, cx: &mut App| {
                    m_aot.update(cx, |m, cx| {
                        m.mutate_note(id, |n| n.aot = !n.aot);
                        cx.notify();
                    });
                });
            let m_fit = model.clone();
            let fit = Button::new("mn-fit")
                .small()
                .icon(IconName::Move)
                .label("Fit to content")
                .on_click(move |_, _, cx: &mut App| {
                    m_fit.update(cx, |m, cx| {
                        m.mutate_note(id, |n| {
                            let lines = n.doc.text.split('\n').count().max(1) as i32;
                            n.height = (64 + lines * 24).clamp(240, 1000);
                        });
                        cx.notify();
                    });
                    windows::resize_note_window(cx, m_fit.clone(), id);
                });
            let m_alarm = model.clone();
            let _alarm_button = Button::new("mn-alarm")
                .small()
                .icon(IconName::AlarmClock)
                .label("Alarm...")
                .on_click(move |_, window: &mut Window, cx: &mut App| {
                    let m = m_alarm.clone();
                    window.open_dialog(cx, move |dialog, _window, _cx| {
                        let mut col = v_flex().gap_1();
                        for (label, minutes) in [
                            ("5 minutes", 5i64),
                            ("10 minutes", 10),
                            ("30 minutes", 30),
                            ("1 hour", 60),
                            ("1 day", 1440),
                        ] {
                            let m = m.clone();
                            col = col.child(
                                Button::new(format!("menu-alarm-{minutes}"))
                                    .small()
                                    .label(format!("Remind in {label}"))
                                    .on_click(move |_, window: &mut Window, cx: &mut App| {
                                        m.update(cx, |m, cx| {
                                            m.mutate_note(id, |n| {
                                                let target = chrono::Local::now().naive_local()
                                                    + chrono::Duration::minutes(minutes);
                                                n.alarm = datetime_to_ole(target);
                                                n.alarm_current = 0.0;
                                            });
                                            cx.notify();
                                        });
                                        window.close_dialog(cx);
                                    }),
                            );
                        }
                        let m_clear = m.clone();
                        col = col.child(
                            Button::new("menu-alarm-clear")
                                .small()
                                .label("Clear alarm")
                                .on_click(move |_, window: &mut Window, cx: &mut App| {
                                    m_clear.update(cx, |m, cx| {
                                        m.mutate_note(id, |n| {
                                            n.alarm = 0.0;
                                            n.alarm_current = 0.0;
                                        });
                                        cx.notify();
                                    });
                                    window.close_dialog(cx);
                                }),
                        );
                        dialog.title("Alarm").w(px(320.)).child(col)
                    });
                });
            let maximize = Button::new("mn-max")
                .small()
                .icon(IconName::Maximize)
                .label("Maximize")
                .on_click(|_, window: &mut Window, _| window.zoom_window());
            let minimize = Button::new("mn-min")
                .small()
                .icon(IconName::Minus)
                .label("Minimize")
                .on_click(|_, window: &mut Window, _| window.minimize_window());
            let m_others = model.clone();
            let hide_others = Button::new("mn-hide-others")
                .small()
                .icon(IconName::EyeOff)
                .label("Hide Others")
                .on_click(move |_, window: &mut Window, cx: &mut App| {
                    let handles: Vec<_> = m_others
                        .read(cx)
                        .note_windows
                        .iter()
                        .filter(|(nid, _)| **nid != id)
                        .map(|(_, h)| *h)
                        .collect();
                    for h in handles {
                        let _ = h.update(cx, |_, window, _| window.remove_window());
                    }
                    m_others.update(cx, |m, _| {
                        for (nid, _) in m.note_windows.iter() {
                            if *nid != id {
                                if let Some(n) = m.notes.iter_mut().find(|n| n.id == *nid) {
                                    n.visible = false;
                                }
                            }
                        }
                        m.note_windows.retain(|nid, _| *nid == id);
                    });
                    let _ = window;
                });
            let m_export = model.clone();
            let export = Button::new("mn-export")
                .small()
                .icon(IconName::Download)
                .label("Export as text...")
                .on_click(move |_, _, cx: &mut App| {
                    let (text, title, md) = m_export
                        .read(cx)
                        .note(id)
                        .map(|n| (n.doc.text.clone(), n.title.clone(), n.doc.markdown))
                        .unwrap_or_default();
                    let dir = dirs::document_dir().unwrap_or_else(|| std::env::temp_dir());
                    let path = dir.join(format!(
                        "{}.{}",
                        crate::util::safe_filename(&title),
                        if md { "md" } else { "txt" }
                    ));
                    let _ = std::fs::write(&path, text);
                    m_export.update(cx, |m, _| {
                        m.toast_error(format!("Exported to {}", path.display()))
                    });
                });
            let m_print = model.clone();
            let print = Button::new("mn-print")
                .small()
                .icon(IconName::Printer)
                .label("Print...")
                .on_click(move |_, _, cx: &mut App| {
                    let (title, text) = m_print
                        .read(cx)
                        .note(id)
                        .map(|n| (n.title.clone(), n.doc.text.clone()))
                        .unwrap_or_default();
                    if !crate::util::print_text(&title, &text) {
                        m_print.update(cx, |m, _| {
                            m.toast_error("Printing is not available (lp command missing)")
                        });
                    }
                });
            let m_locate = model.clone();
            let locate = Button::new("mn-locate")
                .small()
                .icon(IconName::Search)
                .label("Locate in Explorer")
                .on_click(move |_, _, cx: &mut App| {
                    m_locate.update(cx, |m, _| {
                        m.selection = vec![id];
                        m.explorer_page = super::model::ExplorerPage::AllNotes;
                    });
                    windows::open_explorer_window(cx, m_locate.clone());
                });
            let m_hide = model.clone();
            let hide = Button::new("mn-hide")
                .small()
                .icon(IconName::Minus)
                .label("Hide")
                .on_click(move |_, window: &mut Window, cx: &mut App| {
                    m_hide.update(cx, |m, _| {
                        m.mutate_note(id, |n| n.visible = false);
                    });
                    window.remove_window();
                });
            let m_trash = model.clone();
            let trash = Button::new("mn-trash")
                .small()
                .icon(IconName::Trash)
                .label("Delete note")
                .on_click(move |_, window: &mut Window, cx: &mut App| {
                    m_trash.update(cx, |m, cx| {
                        m.trash_note(id);
                        cx.notify();
                    });
                    window.remove_window();
                });

            let mut colors = v_flex().gap_1();
            let mut color_row = h_flex().gap_1();
            for (i, c) in super::model::color_choices_for(&palette)
                .into_iter()
                .enumerate()
            {
                if i > 0 && i % 9 == 0 {
                    colors = colors.child(color_row);
                    color_row = h_flex().gap_1();
                }
                let m_color = model.clone();
                color_row = color_row.child(
                    Button::new(("mn-color", i))
                        .w(px(24.))
                        .h(px(24.))
                        .p_0()
                        .bg(theme::rgba(c))
                        .on_click(move |_, _, cx: &mut App| {
                            m_color.update(cx, |m, cx| {
                                m.mutate_note(id, |n| n.color = c);
                                cx.notify();
                            });
                        }),
                );
            }
            let colors = colors.child(color_row);

            let mut opacity_row = h_flex().gap_1();
            for pct in [100u8, 90, 80, 70, 60, 50, 40, 30, 20, 10] {
                let m_op = model.clone();
                opacity_row = opacity_row.child(
                    Button::new(("mn-opacity", pct as usize))
                        .small()
                        .when(pct == opacity, |b| b.primary())
                        .label(format!("{pct}%"))
                        .on_click(move |_, _, cx: &mut App| {
                            m_op.update(cx, |m, cx| {
                                m.mutate_note(id, |n| n.opacity = pct);
                                cx.notify();
                            });
                        }),
                );
            }
            let mut size_row = h_flex().gap_1();
            for s in 1..=6 {
                let m_size = model.clone();
                size_row = size_row.child(
                    Button::new(("mn-size", s as usize))
                        .small()
                        .label(format!("Size {s}"))
                        .on_click(move |_, _, cx: &mut App| {
                            let (w, h) = note_size_dims(s);
                            m_size.update(cx, |m, cx| {
                                m.mutate_note(id, |n| {
                                    n.width = w;
                                    n.height = h;
                                });
                                cx.notify();
                            });
                            windows::resize_note_window(cx, m_size.clone(), id);
                        }),
                );
            }
            let mut nb_col = v_flex().gap_1();
            for (i, name) in notebooks.iter().enumerate() {
                let m_nb = model.clone();
                let name = name.clone();
                nb_col = nb_col.child(
                    Button::new(("mn-nb", i))
                        .small()
                        .label(name.clone())
                        .on_click(move |_, _, cx: &mut App| {
                            let name = name.clone();
                            m_nb.update(cx, |m, cx| {
                                m.move_note(id, &name);
                                cx.notify();
                            });
                        }),
                );
            }

            let close_title = Button::new("mn-close-title")
                .ghost()
                .small()
                .label("✕")
                .w(px(28.))
                .h(px(24.))
                .p_0()
                .on_click(|_, window, cx| window.close_dialog(cx));
            let title_row = if crate::app::chrome::controls_on_left() {
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(close_title)
                    .child(div().flex_1().child(title.clone()))
            } else {
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().child(title.clone()))
                    .child(close_title)
            };
            dialog
                .title(title_row)
                .w(px(420.))
                .close_button(false)
                .child(
                    v_flex()
                        .gap_1()
                        .child(new_note)
                        .child(duplicate)
                        .child(star)
                        .child(lock)
                        .child(aot_button)
                        .child(fit)
                        .child(maximize)
                        .child(minimize)
                        .child(hide_others)
                        .child(
                            div()
                                .text_xs()
                                .pt_2()
                                .child(format!("Notebook: {notebook}")),
                        )
                        .child(nb_col)
                        .child(div().text_xs().pt_2().child("Color"))
                        .child(colors)
                        .child(div().text_xs().pt_2().child("Opacity"))
                        .child(opacity_row)
                        .child(div().text_xs().pt_2().child("Size"))
                        .child(size_row)
                        .child(export)
                        .child(print)
                        .child(locate)
                        .child(hide)
                        .child(trash),
                )
        });
        let _ = trashed;
    }
}

impl Render for NoteView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let note = self.note(cx);
        let (title, stored_color, opacity, starred, locked, _alarm, _aot) = note
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
            .unwrap_or(("Note".into(), 0xFFFF99, 100, false, false, 0.0, false));
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
        let title_bg = theme::hsla(
            theme::brightness(color, -8),
            (opacity as f32 / 100.0).min(0.98),
        );
        let fg = theme::hsla(theme::auto_font_color(color), 0.92);
        let border = theme::hsla(theme::brightness(color, -22), 0.5);
        let id = self.note_id;

        let title_area = div()
            .id("note-title-drag")
            .flex()
            .flex_1()
            .min_w(px(20.))
            .h_full()
            .items_center()
            .justify_center()
            .text_size(px(13.5))
            .text_color(fg)
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
            .h(px(31.))
            .px_1()
            .items_center()
            .bg(title_bg)
            .rounded_tl(px(12.))
            .rounded_tr(px(12.))
            .border_b_1()
            .border_color(border)
            .when(crate::app::chrome::controls_on_left(), |b| {
                b.child(note_controls(id, fg, cx))
            })
            .child(title_area)
            .child(
                Button::new(("star", id as u64))
                    .ghost()
                    .small()
                    .icon(IconName::Star)
                    .when(starred, |b| b.text_color(gpui_kit::rgb(0xFFB800)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        let id = this.note_id;
                        this.model.update(cx, |m, cx| {
                            m.mutate_note(id, |n| n.starred = !n.starred);
                            cx.notify();
                        });
                    })),
            )
            .child(
                Button::new(("menu", id as u64))
                    .ghost()
                    .small()
                    .icon(IconName::Ellipsis)
                    .on_click(cx.listener(|this, _, window, cx| this.show_menu(window, cx))),
            );

        let font_size = self.font_size(cx);
        let font_name = self.font_name(cx);
        let doc_align = self
            .model
            .read(cx)
            .note(id)
            .map(|n| n.doc.align)
            .unwrap_or(Align::Left);
        // Keep font colour / find / MD / preview exactly centred while the note
        // resizes. The remaining formatting controls live on a second row so
        // the centred group never drifts when the note becomes narrow.
        let window_width = window.bounds().size.width.as_f32();
        let center_row = h_flex()
            .w_full()
            .h(px(28.))
            .items_center()
            .justify_center()
            .gap_0p5()
            .child(fmt_button(
                "font-color",
                IconName::Palette,
                cx.listener(|this, _, window, cx| this.color_dialog(false, window, cx)),
            ))
            .child(fmt_button(
                "find",
                IconName::Search,
                cx.listener(|this, _, window, cx| this.find_replace_dialog(window, cx)),
            ))
            .child(
                Button::new("md-toggle")
                    .ghost()
                    .xsmall()
                    .label(
                        if self
                            .model
                            .read(cx)
                            .note(id)
                            .map(|n| n.doc.markdown)
                            .unwrap_or(true)
                        {
                            "MD"
                        } else {
                            "RTF"
                        },
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        let id = this.note_id;
                        this.model.update(cx, |m, cx| {
                            m.mutate_note(id, |n| n.doc.markdown = !n.doc.markdown);
                            cx.notify();
                        });
                    })),
            )
            .child(
                Button::new("preview")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Eye)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.preview = !this.preview;
                        cx.notify();
                    })),
            );

        let core_tools = h_flex()
            .gap_0p5()
            .items_center()
            .child(fmt_button(
                "bold",
                IconName::Bold,
                cx.listener(|this, _, window, cx| this.format_bool(0, window, cx)),
            ))
            .child(fmt_button(
                "italic",
                IconName::Italic,
                cx.listener(|this, _, window, cx| this.format_bool(1, window, cx)),
            ))
            .child(fmt_button(
                "underline",
                IconName::Underline,
                cx.listener(|this, _, window, cx| this.format_bool(2, window, cx)),
            ))
            .child(fmt_button(
                "strike",
                IconName::Strikethrough,
                cx.listener(|this, _, window, cx| this.format_bool(3, window, cx)),
            ))
            .child(fmt_button(
                "highlight",
                IconName::Highlighter,
                cx.listener(|this, _, window, cx| this.color_dialog(true, window, cx)),
            ))
            .child(fmt_button(
                "bullet",
                IconName::List,
                cx.listener(|this, _, window, cx| this.toggle_line_prefix(0, window, cx)),
            ))
            .child(fmt_button(
                "checkbox",
                IconName::ListChecks,
                cx.listener(|this, _, window, cx| this.toggle_line_prefix(2, window, cx)),
            ));

        let extra_tools = h_flex()
            .gap_0p5()
            .items_center()
            .when(window_width > 340.0, |d| {
                d.child(fmt_button(
                    "numbered",
                    IconName::ListOrdered,
                    cx.listener(|this, _, window, cx| this.toggle_line_prefix(1, window, cx)),
                ))
                .child(fmt_button(
                    "align-left",
                    IconName::TextAlignStart,
                    cx.listener(|this, _, window, cx| this.set_note_align(Align::Left, window, cx)),
                ))
                .child(fmt_button(
                    "align-center",
                    IconName::TextAlignCenter,
                    cx.listener(|this, _, window, cx| {
                        this.set_note_align(Align::Center, window, cx)
                    }),
                ))
                .child(fmt_button(
                    "align-right",
                    IconName::TextAlignEnd,
                    cx.listener(|this, _, window, cx| {
                        this.set_note_align(Align::Right, window, cx)
                    }),
                ))
                .child(fmt_button(
                    "clear",
                    IconName::Eraser,
                    cx.listener(|this, _, window, cx| this.clear_formatting(window, cx)),
                ))
            });

        let format_row = h_flex()
            .w_full()
            .h(px(28.))
            .px_1()
            .items_center()
            .child(core_tools)
            .child(extra_tools)
            .child(div().flex_1())
            .child(
                Button::new("font-size")
                    .ghost()
                    .xsmall()
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
            .bg(theme::hsla(
                theme::brightness(color, -4),
                (opacity as f32 / 100.0).min(0.98),
            ))
            .border_b_1()
            .border_color(border)
            .child(center_row)
            .child(format_row);

        let editor = div()
            .flex()
            .flex_1()
            .min_h(px(0.))
            .p_3()
            .child(if self.preview || locked {
                div()
                    .id("md-preview-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .p_3()
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
                        .text_size(px(font_size as f32)),
                    )
                    .into_any_element()
            } else {
                Editor::new(&self.editor)
                    .appearance(false)
                    .bordered(false)
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
                    .size_full()
                    .rounded(px(10.))
                    .overflow_hidden()
                    .bg(bg)
                    .border_1()
                    .border_color(border)
                    .shadow_lg()
                    .child(title_panel)
                    .when(!self.collapsed, |d| {
                        d.when(!locked, |d| d.child(toolbar)).child(editor)
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

fn note_controls(id: i64, fg: gpui_kit::Hsla, cx: &mut Context<NoteView>) -> impl IntoElement {
    let mut items = Vec::new();
    for kind in crate::app::chrome::control_order() {
        let button = match kind {
            0 => Button::new(("note-close", id as u64))
                .ghost()
                .xsmall()
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
            1 => Button::new(("note-min", id as u64))
                .ghost()
                .xsmall()
                .icon(IconName::Minus)
                .on_click(cx.listener(|this, _, window, cx| {
                    if !this.collapsed {
                        this.expanded_height = window.bounds().size.height.as_f32() as i32;
                        window.resize(gpui_kit::Size {
                            width: window.bounds().size.width,
                            height: px(32.),
                        });
                        this.collapsed = true;
                        cx.notify();
                    }
                })),
            _ => Button::new(("note-max", id as u64))
                .ghost()
                .xsmall()
                .icon(IconName::Maximize)
                .on_click(|_, window, _| window.zoom_window()),
        };
        items.push(button);
    }
    h_flex().gap_0p5().items_center().children(items)
}

fn build_decorations(doc: &NoteDoc) -> Vec<TextDecoration> {
    let mut decorations = Vec::new();
    if doc.markdown {
        return decorations;
    }
    for span in &doc.spans {
        if span.style.is_default() {
            continue;
        }
        let mut style = HighlightStyle::default();
        if let Some(c) = span.style.color {
            style.color = Some(theme::hsla(c, 1.0));
        }
        if span.style.bold {
            style.font_weight = Some(FontWeight::BOLD);
        }
        if span.style.italic {
            style.font_style = Some(FontStyle::Italic);
        }
        if span.style.underline {
            style.underline = Some(UnderlineStyle {
                thickness: px(1.),
                color: Some(theme::hsla(span.style.color.unwrap_or(0x8A6A00), 0.9)),
                wavy: false,
            });
        }
        if span.style.strike {
            style.strikethrough = Some(StrikethroughStyle {
                thickness: px(1.),
                color: Some(theme::hsla(span.style.color.unwrap_or(0x666666), 0.9)),
            });
        }
        if let Some(hl) = span.style.highlight {
            style.background_color = Some(theme::hsla(hl, 0.62));
        }
        decorations.push(TextDecoration::new(span.start..span.end, style));
    }
    decorations
}

fn fmt_button(
    id: &str,
    icon: IconName,
    handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Button {
    Button::new(format!("fmt-{id}"))
        .ghost()
        .xsmall()
        .flex_shrink_0()
        .w(px(22.))
        .h(px(22.))
        .p_0()
        .icon(icon)
        .on_click(handler)
}
