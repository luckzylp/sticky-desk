use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoteSpan {
    pub start: usize,
    pub end: usize,
    #[serde(default)]
    pub style: InlineStyle,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InlineStyle {
    #[serde(default, skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub underline: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub strike: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
    /// 0xRRGGBB
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    /// 0xRRGGBB
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight: Option<u32>,
}

fn is_false(v: &bool) -> bool {
    !*v
}

impl InlineStyle {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ListKind {
    #[default]
    None,
    Bullet,
    Numbered,
    Checkbox,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ParaStyle {
    #[serde(default)]
    pub align: Align,
    #[serde(default)]
    pub list: ListKind,
    #[serde(default)]
    pub indent: u8,
    #[serde(default = "default_spacing")]
    pub line_spacing: f32,
    #[serde(default)]
    pub checked: bool,
}

fn default_spacing() -> f32 {
    1.0
}

impl Default for ParaStyle {
    fn default() -> Self {
        Self {
            align: Align::Left,
            list: ListKind::None,
            indent: 0,
            line_spacing: 1.0,
            checked: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NoteDoc {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub spans: Vec<NoteSpan>,
    #[serde(default)]
    pub paras: Vec<ParaStyle>,
    #[serde(default)]
    pub font_name: String,
    #[serde(default)]
    pub font_size: u32,
    #[serde(default)]
    pub align: Align,
    #[serde(default)]
    pub line_spacing: f32,
    #[serde(default = "default_true")]
    pub markdown: bool,
}

fn default_true() -> bool {
    true
}

impl Default for NoteDoc {
    fn default() -> Self {
        Self {
            text: String::new(),
            spans: Vec::new(),
            paras: vec![ParaStyle::default()],
            font_name: String::new(),
            font_size: 0,
            align: Align::Left,
            line_spacing: 1.0,
            markdown: true,
        }
    }
}

impl NoteDoc {
    pub fn from_text(text: impl Into<String>) -> Self {
        let text = text.into();
        let lines = text.split('\n').count().max(1);
        Self {
            text,
            spans: Vec::new(),
            paras: vec![ParaStyle::default(); lines],
            font_name: String::new(),
            font_size: 0,
            align: Align::Left,
            line_spacing: 1.0,
            markdown: true,
        }
    }

    pub fn ensure_paras(&mut self) {
        let lines = self.text.split('\n').count().max(1);
        self.paras.resize(lines, ParaStyle::default());
    }

    pub fn normalize(&mut self) {
        self.ensure_paras();
        let len = self.text.len();
        self.spans.retain(|s| s.start < s.end && s.start < len);
        for s in &mut self.spans {
            s.end = s.end.min(len);
            s.start = s.start.min(s.end);
        }
        self.spans.sort_by_key(|s| (s.start, s.end));
        self.spans.retain(|s| !s.style.is_default());
    }

    pub fn style_at(&self, index: usize) -> InlineStyle {
        for span in &self.spans {
            if span.start <= index && index < span.end {
                return span.style.clone();
            }
        }
        InlineStyle::default()
    }

    pub fn apply_style(&mut self, start: usize, end: usize, f: impl Fn(&mut InlineStyle) + Copy) {
        if start >= end {
            return;
        }
        let len = self.text.len();
        let start = start.min(len);
        let end = end.min(len);
        let mut spans = Vec::new();
        for span in &self.spans {
            if span.end <= start || span.start >= end {
                spans.push(span.clone());
                continue;
            }
            if span.start < start {
                spans.push(NoteSpan {
                    start: span.start,
                    end: start,
                    style: span.style.clone(),
                });
            }
            if span.end > end {
                spans.push(NoteSpan {
                    start: end,
                    end: span.end,
                    style: span.style.clone(),
                });
            }
        }
        let mut style = self.style_at(start);
        f(&mut style);
        spans.push(NoteSpan { start, end, style });
        self.spans = spans;
        self.normalize();
    }

    pub fn clear_style(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        self.apply_style(start, end, |s| *s = InlineStyle::default());
    }

    /// Convert a byte offset to a character offset.
    /// Convert a character offset to a byte offset.
    pub fn line_index(&self, byte: usize) -> usize {
        self.text[..byte.min(self.text.len())].matches('\n').count()
    }

    pub fn line_start(&self, line: usize) -> usize {
        if line == 0 {
            return 0;
        }
        let mut n = 0;
        for (i, c) in self.text.char_indices() {
            if c == '\n' {
                n += 1;
                if n == line {
                    return i + 1;
                }
            }
        }
        self.text.len()
    }

    pub fn line_end(&self, line: usize) -> usize {
        self.text[self.line_start(line)..]
            .find('\n')
            .map(|i| self.line_start(line) + i)
            .unwrap_or(self.text.len())
    }
}

impl NoteDoc {
    pub fn effective_font(&self, fallback_name: &str, fallback_size: i32) -> (String, u32) {
        let name = if self.font_name.is_empty() {
            fallback_name.to_string()
        } else {
            self.font_name.clone()
        };
        let size = if self.font_size == 0 {
            fallback_size.max(8) as u32
        } else {
            self.font_size
        };
        (name, size)
    }

    /// Shift stored style spans through a single contiguous text replacement.
    pub fn adjust_spans(&mut self, old: &str, new: &str) {
        let mut prefix = 0;
        for (a, b) in old.bytes().zip(new.bytes()) {
            if a != b {
                break;
            }
            prefix += 1;
        }
        while !old.is_char_boundary(prefix) || !new.is_char_boundary(prefix) {
            prefix -= 1;
        }
        let mut suffix = 0;
        while suffix + prefix < old.len() && suffix + prefix < new.len() {
            let ai = old.len() - 1 - suffix;
            let bi = new.len() - 1 - suffix;
            if old.as_bytes()[ai] != new.as_bytes()[bi] {
                break;
            }
            suffix += 1;
        }
        while !old.is_char_boundary(old.len().saturating_sub(suffix))
            || !new.is_char_boundary(new.len().saturating_sub(suffix))
        {
            suffix -= 1;
        }
        let old_end = old.len().saturating_sub(suffix);
        let new_end = new.len().saturating_sub(suffix);
        let delta = new.len() as isize - old.len() as isize;
        let map = |x: usize, is_end: bool| -> usize {
            if x <= prefix {
                x
            } else if x >= old_end {
                (x as isize + delta).max(0) as usize
            } else if is_end {
                new_end
            } else {
                prefix
            }
        };
        for span in &mut self.spans {
            span.start = map(span.start, false);
            span.end = map(span.end, true);
        }
        self.spans.retain(|s| s.start < s.end);
        self.text = new.to_string();
        self.normalize();
    }

    pub fn line_range(&self, index: usize) -> (usize, usize) {
        let line = self.line_index(index);
        (self.line_start(line), self.line_end(line))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Note {
    pub id: i64,
    pub state: i32,
    pub created: f64,
    pub updated: f64,
    pub deleted: f64,
    pub starred: bool,
    pub aot: bool,
    pub minimize: bool,
    pub color: u32,
    pub opacity: u8,
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub locked: bool,
    pub zorder: i32,
    pub alarm: f64,
    pub alarm_current: f64,
    pub alarm_period: i32,
    pub alarm_day: i32,
    pub alarm_snooze: i32,
    pub alarm_sound: String,
    pub notebook: String,
    pub title: String,
    pub note_type: i32,
    pub doc: NoteDoc,
    pub visible: bool,
}

impl Note {
    pub fn is_trashed(&self) -> bool {
        self.state == 0
    }

    pub fn created_dt(&self) -> Option<chrono::NaiveDateTime> {
        ole_to_datetime(self.created)
    }

    pub fn updated_dt(&self) -> Option<chrono::NaiveDateTime> {
        ole_to_datetime(self.updated)
    }

    pub fn deleted_dt(&self) -> Option<chrono::NaiveDateTime> {
        ole_to_datetime(self.deleted)
    }
}

pub fn ole_to_datetime(v: f64) -> Option<chrono::NaiveDateTime> {
    if !(1.0..3_000_000.0).contains(&v) {
        return None;
    }
    let _base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?.and_hms_opt(0, 0, 0)?;
    let secs = ((v - 25569.0) * 86400.0).round() as i64;
    chrono::DateTime::from_timestamp(secs, 0).map(|d| d.naive_utc())
}

pub fn datetime_to_ole(dt: chrono::NaiveDateTime) -> f64 {
    let utc = dt.and_utc().timestamp() as f64;
    utc / 86400.0 + 25569.0
}

pub fn now_ole() -> f64 {
    datetime_to_ole(chrono::Local::now().naive_local())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Notebook {
    pub id: i64,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppMode {
    Light,
    Dark,
    System,
}

impl Default for AppMode {
    fn default() -> Self {
        Self::Light
    }
}

impl AppMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::System => "System",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NoteMode {
    Light,
    Dark,
    Auto,
}

impl Default for NoteMode {
    fn default() -> Self {
        Self::Light
    }
}

impl NoteMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::Auto => "Auto",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefaultNote {
    pub size: i32,
    pub title: i32,
    pub font_name: String,
    pub font_size: i32,
    pub font_style: i32,
    /// 0xRRGGBB, 0 means automatic
    pub font_color: u32,
    /// 0xRRGGBB
    pub color: u32,
    pub opacity: u8,
    pub always_on_top: bool,
}

impl Default for DefaultNote {
    fn default() -> Self {
        Self {
            size: 1,
            title: 0,
            font_name: "Segoe UI".into(),
            font_size: 12,
            font_style: 0,
            font_color: 0,
            color: 0xFFFF99,
            opacity: 100,
            always_on_top: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HotKeys {
    pub new_note: String,
    pub new_note_from_cb: String,
    pub show_all_notes: String,
    pub hide_all_notes: String,
    pub go_next: String,
    pub explorer: String,
}

impl Default for HotKeys {
    fn default() -> Self {
        Self {
            new_note: "Alt+N".into(),
            new_note_from_cb: "Alt+V".into(),
            show_all_notes: "Alt+S".into(),
            hide_all_notes: "Alt+H".into(),
            go_next: "Alt+G".into(),
            explorer: "Alt+X".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub version: String,
    pub language: String,
    pub app_mode: AppMode,
    pub note_mode: NoteMode,
    /// 0xRRGGBB
    pub accent_color: u32,
    pub auto_start: bool,
    pub show_all_notes_on_startup: bool,
    pub check_updates: bool,
    pub live_preview: bool,
    pub always_show_title: bool,
    pub note_color_pal: String,
    pub default_note: DefaultNote,
    pub hotkeys: HotKeys,
    pub db_location: String,
    pub db_location_type: i32,
    pub active_notebook: String,
    #[serde(default)]
    pub default_alarm_sound: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: "0.2.0".into(),
            language: "en".into(),
            app_mode: AppMode::Light,
            note_mode: NoteMode::Light,
            accent_color: 0x0078D4,
            auto_start: false,
            show_all_notes_on_startup: true,
            check_updates: true,
            live_preview: true,
            always_show_title: false,
            note_color_pal: "Standard".into(),
            default_note: DefaultNote::default(),
            hotkeys: HotKeys::default(),
            db_location: String::new(),
            db_location_type: 0,
            active_notebook: "Default".into(),
            default_alarm_sound: String::new(),
        }
    }
}

pub fn note_size_dims(size: i32) -> (i32, i32) {
    let d = match size.clamp(1, 6) {
        1 => 120,
        2 => 135,
        3 => 150,
        4 => 170,
        5 => 190,
        _ => 210,
    };
    (d * 2, d * 2)
}

pub fn title_for_format(format: i32, existing: &dyn Fn(&str) -> bool) -> String {
    let now = chrono::Local::now();
    let base = match format {
        1 => now.format("%m/%d/%Y").to_string(),
        2 => now.format("%m/%d/%Y %H:%M").to_string(),
        3 => now.format("%H:%M").to_string(),
        4 => "To Do".to_string(),
        5 => format!("To Do ({})", now.format("%m/%d/%Y")),
        _ => "New Note".to_string(),
    };
    let mut name = base.clone();
    let mut n = 1;
    while existing(&name) {
        name = format!("{base} ({n})");
        n += 1;
    }
    name
}
