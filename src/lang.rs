use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct LangConfig {
    path: PathBuf,
}

impl LangConfig {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn set(&self, project: &str, lang: &str) {
        let mut config = self.load();
        config.insert(project.to_string(), lang.to_string());
        self.save(&config);
    }

    pub fn get(&self, project: &str) -> Option<String> {
        self.load().get(project).cloned()
    }

    pub fn unset(&self, project: &str) {
        let mut config = self.load();
        config.remove(project);
        self.save(&config);
    }

    fn load(&self) -> HashMap<String, String> {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save(&self, config: &HashMap<String, String>) {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let json = serde_json::to_string(config).unwrap();
        fs::write(&self.path, json).ok();
    }
}

const MIN_TEXT_LENGTH: usize = 8;
const CONFIDENCE_THRESHOLD: f64 = 0.5;

pub fn resolve_lang(code: &str) -> Option<whatlang::Lang> {
    if let Some(lang) = whatlang::Lang::from_code(code) {
        return Some(lang);
    }
    match code {
        "ja" => Some(whatlang::Lang::Jpn),
        "en" => Some(whatlang::Lang::Eng),
        "zh" => Some(whatlang::Lang::Cmn),
        "ko" => Some(whatlang::Lang::Kor),
        "es" => Some(whatlang::Lang::Spa),
        "fr" => Some(whatlang::Lang::Fra),
        "de" => Some(whatlang::Lang::Deu),
        "it" => Some(whatlang::Lang::Ita),
        "pt" => Some(whatlang::Lang::Por),
        "ru" => Some(whatlang::Lang::Rus),
        "ar" => Some(whatlang::Lang::Ara),
        "hi" => Some(whatlang::Lang::Hin),
        "nl" => Some(whatlang::Lang::Nld),
        "sv" => Some(whatlang::Lang::Swe),
        "tr" => Some(whatlang::Lang::Tur),
        "vi" => Some(whatlang::Lang::Vie),
        _ => None,
    }
}

fn lang_to_code(lang: whatlang::Lang) -> &'static str {
    match lang {
        whatlang::Lang::Jpn => "ja",
        whatlang::Lang::Eng => "en",
        whatlang::Lang::Cmn => "zh",
        whatlang::Lang::Kor => "ko",
        whatlang::Lang::Spa => "es",
        whatlang::Lang::Fra => "fr",
        whatlang::Lang::Deu => "de",
        whatlang::Lang::Ita => "it",
        whatlang::Lang::Por => "pt",
        whatlang::Lang::Rus => "ru",
        whatlang::Lang::Ara => "ar",
        whatlang::Lang::Hin => "hi",
        whatlang::Lang::Nld => "nl",
        whatlang::Lang::Swe => "sv",
        whatlang::Lang::Tur => "tr",
        whatlang::Lang::Vie => "vi",
        _ => lang.code(),
    }
}

pub fn validate_language(text: &str, expected_code: &str) -> Result<(), String> {
    if text.chars().count() < MIN_TEXT_LENGTH {
        return Ok(());
    }

    let expected = resolve_lang(expected_code)
        .ok_or_else(|| format!("unsupported language code: '{expected_code}'"))?;

    match whatlang::detect(text) {
        Some(info) if info.confidence() >= CONFIDENCE_THRESHOLD && info.lang() != expected => {
            Err(format!(
                "language mismatch: expected '{}' but detected '{}' (confidence: {:.2})",
                expected_code,
                lang_to_code(info.lang()),
                info.confidence()
            ))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config() -> (LangConfig, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lang.json");
        (LangConfig::new(path), dir)
    }

    #[test]
    fn get_returns_none_when_not_set() {
        let (config, _dir) = temp_config();
        assert!(config.get("test/proj").is_none());
    }

    #[test]
    fn set_and_get() {
        let (config, _dir) = temp_config();
        config.set("test/proj", "ja");
        assert_eq!(config.get("test/proj"), Some("ja".to_string()));
    }

    #[test]
    fn unset_removes() {
        let (config, _dir) = temp_config();
        config.set("test/proj", "ja");
        config.unset("test/proj");
        assert!(config.get("test/proj").is_none());
    }

    #[test]
    fn separate_projects() {
        let (config, _dir) = temp_config();
        config.set("proj/a", "ja");
        config.set("proj/b", "en");
        assert_eq!(config.get("proj/a"), Some("ja".to_string()));
        assert_eq!(config.get("proj/b"), Some("en".to_string()));
    }

    #[test]
    fn resolve_lang_iso639_1() {
        assert_eq!(resolve_lang("ja"), Some(whatlang::Lang::Jpn));
        assert_eq!(resolve_lang("en"), Some(whatlang::Lang::Eng));
    }

    #[test]
    fn resolve_lang_iso639_3() {
        assert_eq!(resolve_lang("jpn"), Some(whatlang::Lang::Jpn));
        assert_eq!(resolve_lang("eng"), Some(whatlang::Lang::Eng));
    }

    #[test]
    fn resolve_lang_unknown() {
        assert!(resolve_lang("xx").is_none());
    }

    #[test]
    fn validate_short_text_always_ok() {
        assert!(validate_language("short", "ja").is_ok());
        assert!(validate_language("abc", "en").is_ok());
    }

    #[test]
    fn validate_japanese_text_with_ja() {
        assert!(validate_language("これは日本語のテストタイトルです", "ja").is_ok());
    }

    #[test]
    fn validate_english_text_with_en() {
        assert!(validate_language("This is an English test title for validation", "en").is_ok());
    }

    #[test]
    fn validate_japanese_text_with_en_fails() {
        let result = validate_language("これは日本語のテストタイトルです", "en");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("language mismatch"));
        assert!(err.contains("expected 'en'"));
    }

    #[test]
    fn validate_english_text_with_ja_fails() {
        let result = validate_language(
            "This is an English test title for validation purposes",
            "ja",
        );
        assert!(result.is_err());
    }

    #[test]
    fn validate_unsupported_code_fails() {
        let result = validate_language("some longer text here", "xx");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported"));
    }
}
