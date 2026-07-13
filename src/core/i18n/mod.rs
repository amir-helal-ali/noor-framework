// ============================================================
// Internationalization (i18n) - التدويل
// ============================================================
// Multi-language support with translation files.
// Supports: Arabic, English, and any other language.
//
// دعم متعدد اللغات مع ملفات الترجمة.
// ============================================================

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Translation messages
/// رسائل الترجمة
pub type Messages = HashMap<String, String>;

/// Language translator
/// مترجم اللغة
pub struct Translator {
    /// All loaded translations (locale -> messages)
    translations: Arc<RwLock<HashMap<String, Messages>>>,
    /// Default locale
    default_locale: String,
    /// Fallback locale
    fallback_locale: String,
    /// Supported locales
    supported_locales: Vec<String>,
}

impl Default for Translator {
    fn default() -> Self {
        Self {
            translations: Arc::new(RwLock::new(HashMap::new())),
            default_locale: "en".to_string(),
            fallback_locale: "en".to_string(),
            supported_locales: vec!["en".to_string(), "ar".to_string()],
        }
    }
}

impl Translator {
    pub fn new(default_locale: &str, fallback_locale: &str) -> Self {
        // Noor is a bilingual (Arabic/English) framework, so both "ar" and
        // "en" are always considered supported for locale detection even if
        // the caller only passes the same value for default and fallback.
        let mut supported: Vec<String> = vec![default_locale.to_string(), fallback_locale.to_string()];
        for locale in &["en", "ar"] {
            let s = locale.to_string();
            if !supported.contains(&s) {
                supported.push(s);
            }
        }
        Self {
            translations: Arc::new(RwLock::new(HashMap::new())),
            default_locale: default_locale.to_string(),
            fallback_locale: fallback_locale.to_string(),
            supported_locales: supported,
        }
    }
    
    /// Load translations from a JSON file
    pub fn load_file(&self, locale: &str, path: &Path) -> crate::NoorResult<()> {
        let content = std::fs::read_to_string(path)?;
        let messages: Messages = serde_json::from_str(&content)?;
        
        self.translations
            .write()
            .insert(locale.to_string(), messages);
        
        Ok(())
    }
    
    /// Load all translations from a directory
    pub fn load_directory(&self, dir: &str) -> crate::NoorResult<()> {
        let dir_path = PathBuf::from(dir);
        
        if !dir_path.exists() {
            tracing::warn!("Translations directory not found: {}", dir);
            return Ok(());
        }
        
        for entry in std::fs::read_dir(&dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    self.load_file(stem, &path)?;
                    tracing::info!("Loaded translations for locale: {}", stem);
                }
            }
        }
        
        Ok(())
    }
    
    /// Add translations programmatically
    pub fn add_messages(&self, locale: &str, messages: Messages) {
        self.translations
            .write()
            .entry(locale.to_string())
            .or_insert_with(HashMap::new)
            .extend(messages);
    }
    
    /// Add a single translation
    pub fn add(&self, locale: &str, key: &str, value: &str) {
        self.translations
            .write()
            .entry(locale.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), value.to_string());
    }
    
    /// Translate a key
    pub fn translate(&self, key: &str, locale: Option<&str>) -> String {
        let locale = locale.unwrap_or(&self.default_locale);
        
        // Try the requested locale
        if let Some(translated) = self.lookup(key, locale) {
            return translated;
        }
        
        // Try fallback locale
        if locale != self.fallback_locale {
            if let Some(translated) = self.lookup(key, &self.fallback_locale) {
                return translated;
            }
        }
        
        // Return the key itself if no translation found
        key.to_string()
    }
    
    /// Translate with parameters
    ///
    /// Replaces `{name}`-style placeholders (single braces) with the
    /// corresponding values from `params`.
    pub fn translate_with_params(&self, key: &str, params: &HashMap<String, String>, locale: Option<&str>) -> String {
        let mut translated = self.translate(key, locale);

        for (k, v) in params {
            let placeholder = format!("{{{}}}", k);
            translated = translated.replace(&placeholder, v);
        }

        translated
    }
    
    /// Pluralization support
    pub fn plural(&self, key: &str, count: i64, locale: Option<&str>) -> String {
        let locale = locale.unwrap_or(&self.default_locale);
        
        // Try count-specific key first (e.g., "posts.count_0", "posts.count_1")
        let count_key = format!("{}.{}", key, count);
        if let Some(translated) = self.lookup(&count_key, locale) {
            return translated;
        }
        
        // Try plural form
        let plural_key = if count == 1 {
            format!("{}.one", key)
        } else {
            format!("{}.other", key)
        };
        
        if let Some(translated) = self.lookup(&plural_key, locale) {
            return translated.replace("{count}", &count.to_string());
        }
        
        // Fallback to base key
        self.translate(key, Some(locale)).replace("{count}", &count.to_string())
    }
    
    /// Lookup a key in a specific locale
    fn lookup(&self, key: &str, locale: &str) -> Option<String> {
        self.translations
            .read()
            .get(locale)
            .and_then(|messages| messages.get(key))
            .cloned()
    }
    
    /// Set the default locale
    pub fn set_default_locale(&mut self, locale: &str) {
        self.default_locale = locale.to_string();
    }
    
    /// Get the default locale
    pub fn default_locale(&self) -> &str {
        &self.default_locale
    }
    
    /// Get supported locales
    pub fn supported_locales(&self) -> &[String] {
        &self.supported_locales
    }
    
    /// Check if a locale is supported
    pub fn is_supported(&self, locale: &str) -> bool {
        self.supported_locales.contains(&locale.to_string())
    }
    
    /// Detect locale from Accept-Language header
    pub fn detect_locale(&self, accept_language: &str) -> String {
        // Parse Accept-Language header (e.g., "ar,en-US;q=0.9,en;q=0.8")
        let languages: Vec<&str> = accept_language.split(',').collect();
        
        for lang in languages {
            let locale = lang.split(';').next().unwrap_or("").trim();
            
            // Try exact match (e.g., "ar")
            if self.is_supported(locale) {
                return locale.to_string();
            }
            
            // Try base language (e.g., "en-US" -> "en")
            if let Some(base) = locale.split('-').next() {
                if self.is_supported(base) {
                    return base.to_string();
                }
            }
        }
        
        self.default_locale.clone()
    }
    
    /// Get text direction (rtl/ltr) for a locale
    pub fn direction(&self, locale: &str) -> TextDirection {
        match locale {
            "ar" | "he" | "fa" | "ur" => TextDirection::Rtl,
            _ => TextDirection::Ltr,
        }
    }
}

/// Text direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    Ltr,
    Rtl,
}

impl TextDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }
}

/// Global translator instance
static GLOBAL_TRANSLATOR: once_cell::sync::Lazy<Arc<Translator>> = 
    once_cell::sync::Lazy::new(|| Arc::new(Translator::default()));

/// Get the global translator
pub fn translator() -> Arc<Translator> {
    GLOBAL_TRANSLATOR.clone()
}

/// Translate a key using the global translator
pub fn t(key: &str) -> String {
    GLOBAL_TRANSLATOR.translate(key, None)
}

/// Translate with locale
pub fn tl(key: &str, locale: &str) -> String {
    GLOBAL_TRANSLATOR.translate(key, Some(locale))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_translation() {
        let translator = Translator::new("en", "en");
        
        translator.add("en", "welcome", "Welcome");
        translator.add("ar", "welcome", "أهلاً وسهلاً");
        
        assert_eq!(translator.translate("welcome", Some("en")), "Welcome");
        assert_eq!(translator.translate("welcome", Some("ar")), "أهلاً وسهلاً");
    }
    
    #[test]
    fn test_fallback() {
        let translator = Translator::new("ar", "en");
        
        translator.add("en", "hello", "Hello");
        // No Arabic translation
        
        assert_eq!(translator.translate("hello", Some("ar")), "Hello");
    }
    
    #[test]
    fn test_missing_key() {
        let translator = Translator::new("en", "en");
        
        assert_eq!(translator.translate("nonexistent", None), "nonexistent");
    }
    
    #[test]
    fn test_translation_with_params() {
        let translator = Translator::new("en", "en");
        
        translator.add("en", "greeting", "Hello, {name}!");
        
        let mut params = HashMap::new();
        params.insert("name".to_string(), "John".to_string());
        
        assert_eq!(
            translator.translate_with_params("greeting", &params, None),
            "Hello, John!"
        );
    }
    
    #[test]
    fn test_pluralization() {
        let translator = Translator::new("en", "en");
        
        translator.add("en", "items.one", "{count} item");
        translator.add("en", "items.other", "{count} items");
        translator.add("ar", "items.one", "عنصر واحد");
        translator.add("ar", "items.other", "{count} عناصر");
        
        assert_eq!(translator.plural("items", 1, Some("en")), "1 item");
        assert_eq!(translator.plural("items", 5, Some("en")), "5 items");
        assert_eq!(translator.plural("items", 1, Some("ar")), "عنصر واحد");
        assert_eq!(translator.plural("items", 5, Some("ar")), "5 عناصر");
    }
    
    #[test]
    fn test_locale_detection() {
        let translator = Translator::new("en", "en");
        
        assert_eq!(translator.detect_locale("ar,en;q=0.8"), "ar");
        assert_eq!(translator.detect_locale("en-US,en;q=0.9"), "en");
        assert_eq!(translator.detect_locale("fr,de"), "en"); // Fallback to default
    }
    
    #[test]
    fn test_text_direction() {
        let translator = Translator::new("en", "en");
        
        assert_eq!(translator.direction("ar"), TextDirection::Rtl);
        assert_eq!(translator.direction("en"), TextDirection::Ltr);
        assert_eq!(translator.direction("he"), TextDirection::Rtl);
    }
}
