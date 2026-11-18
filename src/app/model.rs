use crate::i18n::Translator;
use crate::model::*;
use crate::store::{DB_FILE, Store, default_db_path, load_settings, save_settings, settings_file};
use gpui_kit::AnyWindowHandle;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct AppModel {
    pub settings: Settings,
    pub translator: Translator,
    pub notes: Vec<Note>,
    pub notebooks: Vec<Notebook>,
    pub store: Option<Store>,
    pub db_path: PathBuf,
    pub note_windows: HashMap<i64, AnyWindowHandle>,
    pub explorer_window: Option<AnyWindowHandle>,
    pub settings_window: Option<AnyWindowHandle>,
    pub about_window: Option<AnyWindowHandle>,
    pub alarm_window: Option<AnyWindowHandle>,
    pub active_notebook: String,
    pub first_run: bool,
    pub last_error: Option<String>,
    pub selection: Vec<i64>,
    pub explorer_page: ExplorerPage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplorerPage {
    AllNotes,
    Starred,
    Trash,
    Notebook,
}

impl AppModel {
    pub fn new() -> Self {
        let mut settings = load_settings();
        if settings.db_location.is_empty() {
            let p = default_db_path(&settings_file());
            settings.db_location = p.parent().unwrap_or(&p).to_string_lossy().to_string();
        }
        let db_path = PathBuf::from(&settings.db_location).join(DB_FILE);
        let first_run = !db_path.exists();
        let mut model = Self {
            translator: Translator::new(&settings.language),
            settings,
            notes: Vec::new(),
            notebooks: Vec::new(),
            store: None,
            db_path,
            note_windows: HashMap::new(),
            explorer_window: None,
            settings_window: None,
            about_window: None,
            alarm_window: None,
            active_notebook: "Default".into(),
            first_run,
            last_error: None,
            selection: Vec::new(),
            explorer_page: ExplorerPage::AllNotes,
        };
        model.open_store();
        model
    }

    fn open_store(&mut self) {
        match Store::open(&self.db_path) {
            Ok(store) => {
                self.notes = store.load_notes().unwrap_or_default();
                self.notebooks = store.load_notebooks().unwrap_or_default();
                self.store = Some(store);
                if self.notebooks.is_empty() {
                    if let Some(store) = &self.store {
                        let _ = store.insert_notebook("Default");
                    }
                    self.notebooks.push(Notebook {
                        id: 1,
                        name: "Default".into(),
                    });
                }
                if self.notes.is_empty() {
                    self.create_note(None, None);
                }
                if !self
                    .notebooks
                    .iter()
                    .any(|n| n.name == self.settings.active_notebook)
                {
                    self.settings.active_notebook = self
                        .notebooks
                        .first()
                        .map(|n| n.name.clone())
                        .unwrap_or_else(|| "Default".into());
                }
                self.active_notebook = self.settings.active_notebook.clone();
            }
            Err(e) => {
                self.last_error = Some(format!("{e}"));
            }
        }
    }

    pub fn restore_from(&mut self, source: &std::path::Path) -> anyhow::Result<()> {
        if let Some(store) = self.store.take() {
            drop(store);
        }
        let wal = std::path::PathBuf::from(format!("{}-wal", self.db_path.display()));
        let shm = std::path::PathBuf::from(format!("{}-shm", self.db_path.display()));
        let _ = std::fs::remove_file(&wal);
        let _ = std::fs::remove_file(&shm);
        std::fs::copy(source, &self.db_path)?;
        self.notes.clear();
        self.open_store();
        Ok(())
    }

    pub fn reload_db(&mut self, path: PathBuf) {
        self.db_path = path;
        self.notes.clear();
        self.open_store();
    }

    pub fn note(&self, id: i64) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    pub fn note_mut(&mut self, id: i64) -> Option<&mut Note> {
        self.notes.iter_mut().find(|n| n.id == id)
    }

    pub fn notebooks_with_default(&self) -> Vec<String> {
        let mut v: Vec<String> = self.notebooks.iter().map(|n| n.name.clone()).collect();
        if !v.iter().any(|n| n == "Default") {
            v.insert(0, "Default".into());
        }
        v
    }

    pub fn create_note(&mut self, text: Option<String>, notebook: Option<String>) -> i64 {
        let d = &self.settings.default_note;
        let (w, h) = note_size_dims(d.size);
        let count = self.notes.iter().filter(|n| !n.is_trashed()).count() as i32;
        let existing_titles: Vec<String> = self.notes.iter().map(|n| n.title.clone()).collect();
        let title = title_for_format(d.title, &|s| existing_titles.iter().any(|t| t == s));
        let mut doc = NoteDoc::from_text(text.unwrap_or_default());
        doc.font_name = d.font_name.clone();
        doc.font_size = d.font_size.max(8) as u32;
        let now = now_ole();
        let mut note = Note {
            id: 0,
            state: 1,
            created: now,
            updated: now,
            deleted: 0.0,
            starred: false,
            aot: d.always_on_top,
            minimize: false,
            color: d.color,
            opacity: d.opacity,
            left: 80 + (count % 12) * 26,
            top: 80 + (count % 12) * 24,
            width: w,
            height: h,
            locked: false,
            zorder: self.store.as_ref().map(|s| s.next_zorder()).unwrap_or(1),
            alarm: 0.0,
            alarm_current: 0.0,
            alarm_period: 0,
            alarm_day: 0,
            alarm_snooze: 0,
            alarm_sound: String::new(),
            notebook: notebook
                .clone()
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| self.active_notebook.clone()),
            title,
            note_type: 1,
            doc,
            visible: true,
        };
        let id = if let Some(store) = &self.store {
            match store.insert_note(&note) {
                Ok(id) => id,
                Err(e) => {
                    self.last_error = Some(format!("{e}"));
                    0
                }
            }
        } else {
            0
        };
        note.id = id;
        self.notes.push(note);
        self.save_settings();
        id
    }

    /// Mutate one note and persist it. Uses disjoint field borrows so callers can
    /// touch a note through the model without holding an `&mut self` conflict.
    pub fn mutate_note(&mut self, id: i64, f: impl FnOnce(&mut Note)) -> bool {
        let store = self.store.as_ref();
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == id) {
            f(note);
            if let Some(store) = store {
                let _ = store.save_note(note);
            }
            true
        } else {
            false
        }
    }

    pub fn save_settings(&self) {
        save_settings(&self.settings);
    }

    pub fn set_doc(&mut self, id: i64, doc: NoteDoc, title: Option<String>) {
        self.mutate_note(id, |note| {
            note.doc = doc;
            if let Some(t) = title {
                note.title = t;
            }
            note.updated = now_ole();
        });
    }

    pub fn trash_note(&mut self, id: i64) {
        self.mutate_note(id, |note| {
            note.state = 0;
            note.deleted = now_ole();
            note.visible = false;
        });
    }

    pub fn restore_note(&mut self, id: i64) {
        self.mutate_note(id, |note| {
            note.state = 1;
            note.deleted = 0.0;
            note.visible = true;
        });
    }

    pub fn delete_forever(&mut self, id: i64) {
        if let Some(store) = &self.store {
            let _ = store.delete_forever(id);
        }
        self.notes.retain(|n| n.id != id);
        self.selection.retain(|x| *x != id);
        self.note_windows.remove(&id);
    }

    pub fn empty_trash(&mut self) {
        if let Some(store) = &self.store {
            let _ = store.empty_trash();
        }
        self.notes.retain(|n| !n.is_trashed());
    }

    pub fn duplicate_note(&mut self, id: i64) -> Option<i64> {
        let src = self.note(id)?.clone();
        let new_title = {
            let t = format!("Copy of {}", src.title);
            let existing: Vec<String> = self.notes.iter().map(|n| n.title.clone()).collect();
            let mut name = t.clone();
            let mut i = 1;
            while existing.iter().any(|e| e == &name) {
                name = format!("{t} ({i})");
                i += 1;
            }
            name
        };
        let now = now_ole();
        let mut note = src.clone();
        note.id = 0;
        note.title = new_title;
        note.created = now;
        note.updated = now;
        note.deleted = 0.0;
        note.state = 1;
        note.visible = true;
        note.starred = false;
        note.locked = false;
        note.left += 28;
        note.top += 28;
        note.zorder = self.store.as_ref().map(|s| s.next_zorder()).unwrap_or(1);
        let new_id = if let Some(store) = &self.store {
            store.insert_note(&note).unwrap_or(0)
        } else {
            0
        };
        note.id = new_id;
        if new_id > 0 {
            self.notes.push(note);
            Some(new_id)
        } else {
            None
        }
    }

    pub fn move_note(&mut self, id: i64, notebook: &str) {
        self.mutate_note(id, |note| note.notebook = notebook.to_string());
    }

    pub fn merge_notes(&mut self, ids: &[i64]) -> Option<i64> {
        if ids.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        for id in ids {
            if let Some(n) = self.note(*id) {
                parts.push(n.doc.text.clone());
            }
        }
        let text = parts.join("\n\n");
        let target = ids[0];
        self.mutate_note(target, |note| {
            note.doc = NoteDoc::from_text(text);
            note.updated = now_ole();
        });
        for id in ids.iter().skip(1) {
            self.trash_note(*id);
        }
        Some(target)
    }

    pub fn create_notebook(&mut self, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty()
            || self
                .notebooks
                .iter()
                .any(|n| n.name.eq_ignore_ascii_case(name))
        {
            return false;
        }
        if let Some(store) = &self.store {
            let _ = store.insert_notebook(name);
        }
        self.notebooks.push(Notebook {
            id: 0,
            name: name.to_string(),
        });
        self.notebooks.sort_by(|a, b| a.name.cmp(&b.name));
        true
    }

    pub fn rename_notebook(&mut self, old: &str, new: &str) -> bool {
        let new = new.trim();
        if new.is_empty()
            || self
                .notebooks
                .iter()
                .any(|n| n.name.eq_ignore_ascii_case(new))
        {
            return false;
        }
        if let Some(store) = &self.store {
            let _ = store.rename_notebook(old, new);
        }
        if let Some(n) = self.notebooks.iter_mut().find(|n| n.name == old) {
            n.name = new.to_string();
        }
        for n in &mut self.notes {
            if n.notebook == old {
                n.notebook = new.to_string();
            }
        }
        if self.active_notebook == old {
            self.active_notebook = new.to_string();
            self.settings.active_notebook = new.to_string();
        }
        true
    }

    pub fn delete_notebook(&mut self, name: &str) {
        if name == "Default" {
            return;
        }
        if let Some(store) = &self.store {
            let _ = store.delete_notebook(name);
        }
        self.notes.retain(|n| n.notebook != name);
        self.notebooks.retain(|n| n.name != name);
        if self.active_notebook == name {
            self.active_notebook = "Default".into();
            self.settings.active_notebook = "Default".into();
        }
    }

    pub fn toast_error(&mut self, msg: impl Into<String>) {
        self.last_error = Some(msg.into());
    }

    pub fn alarms_due(&self) -> Vec<i64> {
        let now = now_ole();
        self.notes
            .iter()
            .filter(|n| !n.is_trashed())
            .filter(|n| n.alarm > 0.0 && n.alarm <= now)
            .map(|n| n.id)
            .collect()
    }
}

pub fn color_choices_for(palette: &str) -> Vec<u32> {
    match palette {
        "Classic" => crate::theme::classic_colors(),
        "Pastel" => crate::theme::pastel_palette(),
        _ => crate::theme::standard_palette()
            .into_iter()
            .map(|c| c.base)
            .collect(),
    }
}
