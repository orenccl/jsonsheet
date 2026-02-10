use jsonsheet::state::i18n::{self, Language};

#[test]
fn test_default_language_is_english() {
    assert_eq!(Language::default(), Language::En);
    assert_eq!(i18n::tr(Language::default(), "toolbar.open"), "Open");
}

#[test]
fn test_language_switch_changes_ui_text() {
    assert_eq!(i18n::tr(Language::En, "toolbar.open"), "Open");
    assert_eq!(i18n::tr(Language::ZhHant, "toolbar.open"), "開啟");
}

#[test]
fn test_missing_key_falls_back_to_english() {
    assert_eq!(
        i18n::tr(Language::ZhHant, "test.fallback_only"),
        "Fallback value"
    );
}

#[test]
fn test_language_code_roundtrip() {
    assert_eq!(Language::from_code("en"), Some(Language::En));
    assert_eq!(Language::from_code("zh-Hant"), Some(Language::ZhHant));
    assert_eq!(Language::from_code("unknown"), None);
}
