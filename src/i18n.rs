use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    ChineseSimplified,
    English,
}

impl Language {
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::ChineseSimplified => "简体中文",
            Language::English => "English",
        }
    }

    pub fn config_key(&self) -> &'static str {
        match self {
            Language::ChineseSimplified => "zh",
            Language::English => "en",
        }
    }
}

pub struct I18nManager {
    pub active_lang: Language,
    // 使用 Box::leak 后的静态映射，确保 tr 返回 &'static str
    translations: HashMap<&'static str, HashMap<&'static str, &'static str>>,
}

impl I18nManager {
    pub fn new(lang: Language) -> Self {
        let mut manager = Self {
            active_lang: lang,
            translations: HashMap::new(),
        };
        manager.load_all();
        manager
    }

    fn load_all(&mut self) {
        let zh_json = include_str!("../assets/lang_zh.json");
        let en_json = include_str!("../assets/lang_en.json");

        self.translations
            .insert("zh", Self::parse_and_leak(zh_json));
        self.translations
            .insert("en", Self::parse_and_leak(en_json));
    }

    fn parse_and_leak(json: &str) -> HashMap<&'static str, &'static str> {
        let raw: HashMap<String, String> = serde_json::from_str(json).unwrap_or_default();
        let mut leaked = HashMap::new();
        for (k, v) in raw {
            // 将读取到的字符串永久保留在内存中，换取 &'static 生命周期
            let k_static: &'static str = Box::leak(k.into_boxed_str());
            let v_static: &'static str = Box::leak(v.into_boxed_str());
            leaked.insert(k_static, v_static);
        }
        leaked
    }

    /// 现在返回的是 &'static str，不再与 self 绑定，解决了所有的 Borrow Checker 问题
    pub fn tr(&self, key: &str) -> &'static str {
        let lang_key = self.active_lang.config_key();
        if let Some(map) = self.translations.get(lang_key) {
            if let Some(val) = map.get(key) {
                return val;
            }
        }
        // 如果找不到翻译，为了维持 &'static，我们需要泄漏一下这个 key 或返回预定义的静态字符串
        // 考虑到 key 通常是字面量，这在实际场景中几乎不会发生
        Box::leak(key.to_string().into_boxed_str())
    }
}
