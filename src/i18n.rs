use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A static mapping of ISO language codes to their native display names.
const NATIVE_NAMES: &[(&str, &str)] = &[
    ("en_US", "English (US)"),
    ("zh_CN", "简体中文"),
    ("ja_JP", "日本語"),
    ("zh_TW", "繁體中文"),
    ("ko_KR", "한국어"),
    ("fr_FR", "Français"),
    ("de_DE", "Deutsch"),
    ("es_ES", "Español"),
    ("ru_RU", "Русский"),
];

pub struct I18nManager {
    pub active_lang: String,
    pub available_langs: Vec<String>,
    // Map: lang_id -> (key -> value)
    translations: HashMap<&'static str, HashMap<&'static str, &'static str>>,
}

impl I18nManager {
    pub fn new(lang_id: String) -> Self {
        let mut manager = Self {
            active_lang: lang_id,
            available_langs: Vec::new(),
            translations: HashMap::new(),
        };
        manager.load_all();
        manager
    }

    fn load_all(&mut self) {
        let lang_dir = Path::new("assets/lang");
        if !lang_dir.exists() {
            // Fallback if directory missing
            return;
        }

        if let Ok(entries) = fs::read_dir(lang_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(lang_id) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let lang_id_static: &'static str =
                                Box::leak(lang_id.to_string().into_boxed_str());
                            self.translations
                                .insert(lang_id_static, Self::parse_and_leak(&content));
                            self.available_langs.push(lang_id.to_string());
                        }
                    }
                }
            }
        }

        // Sort languages to keep order consistent
        self.available_langs.sort();

        // Ensure active_lang is valid, if not, fallback to "en_US" or first available
        if !self.available_langs.contains(&self.active_lang) {
            if self.available_langs.contains(&"en_US".to_string()) {
                self.active_lang = "en_US".into();
            } else if let Some(first) = self.available_langs.first() {
                self.active_lang = first.clone();
            } else {
                self.active_lang = "en_US".into();
            }
        }
    }

    fn parse_and_leak(json: &str) -> HashMap<&'static str, &'static str> {
        let raw: HashMap<String, String> = serde_json::from_str(json).unwrap_or_default();
        let mut leaked = HashMap::new();
        for (k, v) in raw {
            let k_static: &'static str = Box::leak(k.into_boxed_str());
            let v_static: &'static str = Box::leak(v.into_boxed_str());
            leaked.insert(k_static, v_static);
        }
        leaked
    }

    /// Translates a key based on active language.
    pub fn tr(&self, key: &str) -> &'static str {
        if let Some(map) = self.translations.get(self.active_lang.as_str()) {
            if let Some(val) = map.get(key) {
                return val;
            }
        }
        // Fallback to "en_US" if current lang doesn't have the key
        if self.active_lang != "en_US" {
            if let Some(en_map) = self.translations.get("en_US") {
                if let Some(val) = en_map.get(key) {
                    return val;
                }
            }
        }
        // Final fallback: return the key itself leaked to static
        Box::leak(key.to_string().into_boxed_str())
    }

    /// Gets the native name of a language ID from the master table.
    pub fn get_lang_name(&self, lang_id: &str) -> &'static str {
        for (code, native_name) in NATIVE_NAMES {
            if *code == lang_id {
                return native_name;
            }
        }
        // Fallback to ID itself if not in table
        Box::leak(lang_id.to_string().into_boxed_str())
    }
}
