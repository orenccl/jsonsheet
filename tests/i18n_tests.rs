use jsonsheet::state::i18n::{self, Language};
use std::collections::BTreeSet;

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

#[test]
fn test_zh_hant_catalog_matches_english_keys_except_fallback_probe() {
    let en: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(include_str!("../assets/i18n/en.json"))
            .expect("en.json should be valid JSON object");
    let zh: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(include_str!("../assets/i18n/zh-Hant.json"))
            .expect("zh-Hant.json should be valid JSON object");

    let allowed_missing: BTreeSet<&str> = BTreeSet::from(["test.fallback_only"]);

    let en_keys: BTreeSet<&str> = en.keys().map(String::as_str).collect();
    let zh_keys: BTreeSet<&str> = zh.keys().map(String::as_str).collect();

    let missing: Vec<&str> = en_keys
        .difference(&zh_keys)
        .copied()
        .filter(|key| !allowed_missing.contains(key))
        .collect();

    assert!(
        missing.is_empty(),
        "zh-Hant catalog is missing keys: {}",
        missing.join(", ")
    );
}
