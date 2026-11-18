use serde_json::Value;
use std::collections::HashMap;

pub struct Translator {
    lang: String,
    table: Value,
    fallback: Value,
}

impl Translator {
    pub fn new(lang: &str) -> Self {
        let fallback =
            serde_json::from_str(include_str!("../assets/i18n/en.json")).unwrap_or(Value::Null);
        let lang = normalize_lang(lang);
        let table = load_lang(&lang).unwrap_or_else(|| fallback.clone());
        Self {
            lang,
            table,
            fallback,
        }
    }

    pub fn set_lang(&mut self, lang: &str) {
        let lang = normalize_lang(lang);
        if let Some(table) = load_lang(&lang) {
            self.table = table;
            self.lang = lang;
        }
    }

    pub fn t(&self, section: &str, key: &str) -> String {
        self.lookup(&self.table, section, key)
            .or_else(|| self.lookup(&self.fallback, section, key))
            .unwrap_or_else(|| key.to_string())
    }

    fn lookup(&self, table: &Value, section: &str, key: &str) -> Option<String> {
        table
            .get(section)
            .and_then(|s| s.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub fn languages(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("zh", "Chinese, Simplified (简体中文)"),
            ("da", "Danish (Dansk)"),
            ("en", "English"),
            ("fr", "French (Français)"),
            ("de", "German (Deutsch)"),
            ("it", "Italian (Italiano)"),
            ("ja", "Japanese (日本語)"),
            ("nl", "Nederlands (Dutch)"),
            ("pt", "Portuguese (Português)"),
            ("ru", "Russian (Русский)"),
            ("es", "Spanish (Español)"),
            ("sv", "Swedish (Svenska)"),
            ("tr", "Turkish (Türkçe)"),
        ]
    }
}

fn normalize_lang(lang: &str) -> String {
    let l = lang.to_ascii_lowercase();
    if l.starts_with("zh") {
        "zh".into()
    } else {
        l.split(['-', '_']).next().unwrap_or("en").to_string()
    }
}

fn load_lang(lang: &str) -> Option<Value> {
    let data = match lang {
        "da" => include_str!("../assets/i18n/da.json"),
        "de" => include_str!("../assets/i18n/de.json"),
        "en" => include_str!("../assets/i18n/en.json"),
        "es" => include_str!("../assets/i18n/es.json"),
        "fr" => include_str!("../assets/i18n/fr.json"),
        "it" => include_str!("../assets/i18n/it.json"),
        "ja" => include_str!("../assets/i18n/ja.json"),
        "nl" => include_str!("../assets/i18n/nl.json"),
        "pt" => include_str!("../assets/i18n/pt.json"),
        "ru" => include_str!("../assets/i18n/ru.json"),
        "sv" => include_str!("../assets/i18n/sv.json"),
        "tr" => include_str!("../assets/i18n/tr.json"),
        "zh" => include_str!("../assets/i18n/zh.json"),
        _ => return None,
    };
    let data = data.strip_prefix('\u{feff}').unwrap_or(data);
    serde_json::from_str(data).ok()
}

pub fn language_name(lang: &str) -> String {
    let map: HashMap<&str, &str> = [
        ("zh", "Chinese, Simplified (简体中文)"),
        ("da", "Danish (Dansk)"),
        ("en", "English"),
        ("fr", "French (Français)"),
        ("de", "German (Deutsch)"),
        ("it", "Italian (Italiano)"),
        ("ja", "Japanese (日本語)"),
        ("nl", "Nederlands (Dutch)"),
        ("pt", "Portuguese (Português)"),
        ("ru", "Russian (Русский)"),
        ("es", "Spanish (Español)"),
        ("sv", "Swedish (Svenska)"),
        ("tr", "Turkish (Türkçe)"),
    ]
    .into_iter()
    .collect();
    map.get(lang).unwrap_or(&"English").to_string()
}
