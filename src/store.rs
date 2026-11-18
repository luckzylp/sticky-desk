use crate::model::*;
use rusqlite::{Connection, params};
use std::fs;
use std::path::{Path, PathBuf};

pub const DB_FILE: &str = "Notes.db";

pub struct Store {
    pub conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        let store = Self { conn };
        store.ensure_schema()?;
        Ok(store)
    }

    fn ensure_schema(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS NOTEBOOKS (
                ID INTEGER PRIMARY KEY,
                NAME TEXT
            );
            CREATE TABLE IF NOT EXISTS NOTES (
                ID INTEGER PRIMARY KEY,
                STATE INTEGER DEFAULT 1,
                CREATED REAL DEFAULT 0,
                UPDATED REAL DEFAULT 0,
                DELETED REAL DEFAULT 0,
                STARRED INTEGER DEFAULT 0,
                AOT INTEGER DEFAULT 0,
                MINIMIZE INTEGER DEFAULT 0,
                COLOR INTEGER DEFAULT 65535,
                OPACITY INTEGER DEFAULT 100,
                LEFT INTEGER DEFAULT 100,
                TOP INTEGER DEFAULT 100,
                WIDTH INTEGER DEFAULT 480,
                HEIGHT INTEGER DEFAULT 480,
                LOCKED INTEGER DEFAULT 0,
                ZORDER INTEGER DEFAULT 0,
                ALARM REAL DEFAULT 0,
                ALARM_CURRENT REAL DEFAULT 0,
                ALARM_PERIOD INTEGER DEFAULT 0,
                ALARM_DAY INTEGER DEFAULT 0,
                ALARM_SNOOZE INTEGER DEFAULT 0,
                ALARM_SOUND TEXT DEFAULT '',
                NOTEBOOK TEXT DEFAULT 'Default',
                TITLE TEXT DEFAULT 'New Note',
                TYPE INTEGER DEFAULT 0,
                DATA BLOB,
                TEXT TEXT DEFAULT ''
            );
            "#,
        )?;
        // Windows databases from older releases may lack newer columns.
        let cols: Vec<String> = self
            .conn
            .prepare("PRAGMA table_info(NOTES)")?
            .query_map([], |r| r.get::<_, String>(1))?
            .filter_map(Result::ok)
            .collect();
        let add = |name: &str, ty: &str| {
            if !cols.iter().any(|c| c.eq_ignore_ascii_case(name)) {
                self.conn
                    .execute(&format!("ALTER TABLE NOTES ADD COLUMN {name} {ty}"), [])
                    .ok();
            }
        };
        add("STATE", "INTEGER DEFAULT 1");
        add("DELETED", "REAL DEFAULT 0");
        add("STARRED", "INTEGER DEFAULT 0");
        add("AOT", "INTEGER DEFAULT 0");
        add("MINIMIZE", "INTEGER DEFAULT 0");
        add("COLOR", "INTEGER DEFAULT 65535");
        add("OPACITY", "INTEGER DEFAULT 100");
        add("LEFT", "INTEGER DEFAULT 100");
        add("TOP", "INTEGER DEFAULT 100");
        add("WIDTH", "INTEGER DEFAULT 480");
        add("HEIGHT", "INTEGER DEFAULT 480");
        add("LOCKED", "INTEGER DEFAULT 0");
        add("ZORDER", "INTEGER DEFAULT 0");
        add("ALARM", "REAL DEFAULT 0");
        add("ALARM_CURRENT", "REAL DEFAULT 0");
        add("ALARM_PERIOD", "INTEGER DEFAULT 0");
        add("ALARM_DAY", "INTEGER DEFAULT 0");
        add("ALARM_SNOOZE", "INTEGER DEFAULT 0");
        add("ALARM_SOUND", "TEXT DEFAULT ''");
        add("NOTEBOOK", "TEXT DEFAULT 'Default'");
        add("TITLE", "TEXT DEFAULT 'New Note'");
        add("TYPE", "INTEGER DEFAULT 0");
        add("DATA", "BLOB");
        add("TEXT", "TEXT DEFAULT ''");

        if self.load_notebooks()?.is_empty() {
            self.conn.execute(
                "INSERT INTO NOTEBOOKS (NAME) VALUES (?1)",
                params!["Default"],
            )?;
        }
        Ok(())
    }

    pub fn load_notebooks(&self) -> anyhow::Result<Vec<Notebook>> {
        let mut stmt = self
            .conn
            .prepare("SELECT ID, NAME FROM NOTEBOOKS ORDER BY NAME ASC")?;
        let rows = stmt.query_map([], |r| {
            Ok(Notebook {
                id: r.get(0)?,
                name: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn load_notes(&self) -> anyhow::Result<Vec<Note>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM NOTES ORDER BY ZORDER ASC")?;
        let names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let geti = |n: &str, d: i64| -> i64 {
                names
                    .iter()
                    .position(|x| x.eq_ignore_ascii_case(n))
                    .and_then(|i| row.get::<_, Option<i64>>(i).ok().flatten())
                    .unwrap_or(d)
            };
            let getf = |n: &str, d: f64| -> f64 {
                names
                    .iter()
                    .position(|x| x.eq_ignore_ascii_case(n))
                    .and_then(|i| {
                        row.get::<_, Option<f64>>(i).ok().flatten().or_else(|| {
                            row.get::<_, Option<i64>>(i)
                                .ok()
                                .flatten()
                                .map(|v| v as f64)
                        })
                    })
                    .unwrap_or(d)
            };
            let gets = |n: &str| -> String {
                names
                    .iter()
                    .position(|x| x.eq_ignore_ascii_case(n))
                    .and_then(|i| row.get::<_, Option<String>>(i).ok().flatten())
                    .unwrap_or_default()
            };
            let text = gets("TEXT");
            let data: Option<Vec<u8>> = names
                .iter()
                .position(|x| x.eq_ignore_ascii_case("DATA"))
                .and_then(|i| row.get::<_, Option<Vec<u8>>>(i).ok().flatten());
            let doc = data
                .as_deref()
                .and_then(parse_doc)
                .unwrap_or_else(|| NoteDoc::from_text(text.clone()));
            let mut doc = doc;
            if doc.text.is_empty() && !text.is_empty() {
                doc.text = text;
            }
            doc.normalize();
            out.push(Note {
                id: geti("ID", 0),
                state: geti("STATE", 1) as i32,
                created: getf("CREATED", 0.0),
                updated: getf("UPDATED", 0.0),
                deleted: getf("DELETED", 0.0),
                starred: geti("STARRED", 0) != 0,
                aot: geti("AOT", 0) != 0,
                minimize: geti("MINIMIZE", 0) != 0,
                color: geti("COLOR", 0xFFFF99) as u32,
                opacity: geti("OPACITY", 100).clamp(10, 100) as u8,
                left: geti("LEFT", 100) as i32,
                top: geti("TOP", 100) as i32,
                width: geti("WIDTH", 480) as i32,
                height: geti("HEIGHT", 480) as i32,
                locked: geti("LOCKED", 0) != 0,
                zorder: geti("ZORDER", 0) as i32,
                alarm: getf("ALARM", 0.0),
                alarm_current: getf("ALARM_CURRENT", 0.0),
                alarm_period: geti("ALARM_PERIOD", 0) as i32,
                alarm_day: geti("ALARM_DAY", 0) as i32,
                alarm_snooze: geti("ALARM_SNOOZE", 0) as i32,
                alarm_sound: gets("ALARM_SOUND"),
                notebook: {
                    let n = gets("NOTEBOOK");
                    if n.is_empty() { "Default".into() } else { n }
                },
                title: {
                    let t = gets("TITLE");
                    if t.is_empty() { "New Note".into() } else { t }
                },
                note_type: geti("TYPE", 0) as i32,
                doc,
                visible: true,
            });
        }
        Ok(out)
    }

    pub fn next_zorder(&self) -> i32 {
        self.conn
            .query_row("SELECT COALESCE(MAX(ZORDER), 0) + 1 FROM NOTES", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap_or(1) as i32
    }

    pub fn insert_note(&self, n: &Note) -> anyhow::Result<i64> {
        let data = serde_json::to_vec(&n.doc)?;
        self.conn.execute(
            "INSERT INTO NOTES (STATE,CREATED,UPDATED,DELETED,STARRED,AOT,MINIMIZE,COLOR,OPACITY,LEFT,TOP,WIDTH,HEIGHT,LOCKED,ZORDER,ALARM,ALARM_CURRENT,ALARM_PERIOD,ALARM_DAY,ALARM_SNOOZE,ALARM_SOUND,NOTEBOOK,TITLE,TYPE,DATA,TEXT)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)",
            params![
                n.state, n.created, n.updated, n.deleted, n.starred as i64, n.aot as i64,
                n.minimize as i64, n.color as i64, n.opacity as i64, n.left, n.top,
                n.width, n.height, n.locked as i64, n.zorder, n.alarm, n.alarm_current,
                n.alarm_period, n.alarm_day, n.alarm_snooze, n.alarm_sound, n.notebook,
                n.title, n.note_type, data, n.doc.text
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn save_note(&self, n: &Note) -> anyhow::Result<()> {
        let data = serde_json::to_vec(&n.doc)?;
        self.conn.execute(
            "UPDATE NOTES SET STATE=?1,UPDATED=?2,DELETED=?3,STARRED=?4,AOT=?5,MINIMIZE=?6,COLOR=?7,OPACITY=?8,LEFT=?9,TOP=?10,WIDTH=?11,HEIGHT=?12,LOCKED=?13,ZORDER=?14,ALARM=?15,ALARM_CURRENT=?16,ALARM_PERIOD=?17,ALARM_DAY=?18,ALARM_SNOOZE=?19,ALARM_SOUND=?20,NOTEBOOK=?21,TITLE=?22,TYPE=?23,DATA=?24,TEXT=?25 WHERE ID=?26",
            params![
                n.state, n.updated, n.deleted, n.starred as i64, n.aot as i64,
                n.minimize as i64, n.color as i64, n.opacity as i64, n.left, n.top,
                n.width, n.height, n.locked as i64, n.zorder, n.alarm, n.alarm_current,
                n.alarm_period, n.alarm_day, n.alarm_snooze, n.alarm_sound, n.notebook,
                n.title, n.note_type, data, n.doc.text, n.id
            ],
        )?;
        Ok(())
    }

    pub fn delete_forever(&self, id: i64) -> anyhow::Result<()> {
        self.conn
            .execute("DELETE FROM NOTES WHERE ID=?1", params![id])?;
        Ok(())
    }

    pub fn empty_trash(&self) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM NOTES WHERE STATE=0", [])?;
        Ok(())
    }

    pub fn insert_notebook(&self, name: &str) -> anyhow::Result<i64> {
        self.conn
            .execute("INSERT INTO NOTEBOOKS (NAME) VALUES (?1)", params![name])?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn rename_notebook(&self, old: &str, new: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE NOTEBOOKS SET NAME=?1 WHERE NAME=?2",
            params![new, old],
        )?;
        self.conn.execute(
            "UPDATE NOTES SET NOTEBOOK=?1 WHERE NOTEBOOK=?2",
            params![new, old],
        )?;
        Ok(())
    }

    pub fn delete_notebook(&self, name: &str) -> anyhow::Result<()> {
        self.conn
            .execute("DELETE FROM NOTEBOOKS WHERE NAME=?1", params![name])?;
        self.conn
            .execute("DELETE FROM NOTES WHERE NOTEBOOK=?1", params![name])?;
        Ok(())
    }

    pub fn backup_to(&self, path: &Path) -> anyhow::Result<()> {
        if path.exists() {
            fs::remove_file(path)?;
        }
        let escaped = path.to_string_lossy().replace('\'', "''");
        self.conn
            .execute_batch(&format!("VACUUM INTO '{}'", escaped))?;
        Ok(())
    }
}

fn parse_doc(bytes: &[u8]) -> Option<NoteDoc> {
    let text = std::str::from_utf8(bytes).ok()?;
    let text = text.trim_start_matches('\u{feff}');
    if text.trim_start().starts_with('{') {
        if let Ok(doc) = serde_json::from_str::<NoteDoc>(text) {
            return Some(doc);
        }
    }
    if text.starts_with("{\\rtf") {
        return Some(NoteDoc::from_text(rtf_to_text(text)));
    }
    None
}

fn rtf_to_text(rtf: &str) -> String {
    let mut out = String::new();
    let mut chars = rtf.chars().peekable();
    let mut skip_group = 0i32;
    while let Some(c) = chars.next() {
        match c {
            '{' => skip_group += 1,
            '}' => skip_group = (skip_group - 1).max(0),
            '\\' => {
                let mut word = String::new();
                while let Some(&n) = chars.peek() {
                    if n.is_ascii_alphabetic() {
                        word.push(n);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if word == "par" || word == "line" {
                    out.push('\n');
                } else if word == "tab" {
                    out.push('\t');
                } else if word == "u" {
                    let mut num = String::new();
                    while let Some(&n) = chars.peek() {
                        if n.is_ascii_digit() || n == '-' {
                            num.push(n);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if let Ok(v) = num.parse::<i32>() {
                        if let Some(ch) = char::from_u32(v as u32) {
                            out.push(ch);
                        }
                    }
                } else if !word.is_empty() && skip_group == 1 {
                    // formatting control word: ignore
                }
                if chars.peek() == Some(&' ') {
                    chars.next();
                }
            }
            '\r' | '\n' => {}
            _ if skip_group == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

pub fn default_db_path(_settings_path: &Path) -> PathBuf {
    if let Some(docs) = dirs::document_dir() {
        docs.join("StickyDesk").join(DB_FILE)
    } else if let Some(data) = dirs::data_dir() {
        data.join("StickyDesk").join(DB_FILE)
    } else {
        PathBuf::from(DB_FILE)
    }
}

pub fn settings_file() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sticky-desk")
        .join("settings.json")
}

pub fn load_settings() -> Settings {
    let path = settings_file();
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<Settings>(s.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default()
}

pub fn save_settings(settings: &Settings) {
    let path = settings_file();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, s);
    }
}

pub fn import_into(conn: &Connection, path: &Path) -> anyhow::Result<()> {
    conn.execute(
        &format!(
            "ATTACH DATABASE '{}' AS src",
            path.to_string_lossy().replace('\'', "''")
        ),
        [],
    )?;
    conn.execute("INSERT OR REPLACE INTO NOTES SELECT * FROM src.NOTES", [])
        .ok();
    conn.execute(
        "INSERT OR IGNORE INTO NOTEBOOKS SELECT * FROM src.NOTEBOOKS",
        [],
    )
    .ok();
    conn.execute("DETACH DATABASE src", [])?;
    Ok(())
}
