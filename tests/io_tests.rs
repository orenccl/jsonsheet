use std::collections::BTreeMap;

use serde_json::Value;

// Import from the crate
use jsonsheet::io::json_io::{self, JsonIoError, Row};

fn sample_data() -> Vec<Row> {
    vec![
        BTreeMap::from([
            ("name".to_string(), Value::String("Alice".to_string())),
            ("age".to_string(), Value::Number(30.into())),
            ("active".to_string(), Value::Bool(true)),
        ]),
        BTreeMap::from([
            ("name".to_string(), Value::String("Bob".to_string())),
            ("age".to_string(), Value::Number(25.into())),
            ("active".to_string(), Value::Bool(false)),
        ]),
    ]
}

#[test]
fn test_load_json_valid() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.json");
    std::fs::write(
        &path,
        r#"[{"name":"Alice","age":30},{"name":"Bob","age":25}]"#,
    )
    .unwrap();

    let rows = json_io::load_json(&path).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], Value::String("Alice".to_string()));
    assert_eq!(rows[1]["age"], Value::Number(25.into()));
}

#[test]
fn test_load_json_empty_array() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.json");
    std::fs::write(&path, "[]").unwrap();

    let rows = json_io::load_json(&path).unwrap();
    assert!(rows.is_empty());
}

#[test]
fn test_load_json_not_array() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("obj.json");
    std::fs::write(&path, r#"{"key": "value"}"#).unwrap();

    let err = json_io::load_json(&path).unwrap_err();
    assert!(matches!(err, JsonIoError::NotAnArray));
}

#[test]
fn test_load_json_not_objects() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nums.json");
    std::fs::write(&path, "[1, 2, 3]").unwrap();

    let err = json_io::load_json(&path).unwrap_err();
    assert!(matches!(err, JsonIoError::NotArrayOfObjects));
}

#[test]
fn test_load_json_invalid_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.json");
    std::fs::write(&path, "not json at all").unwrap();

    let err = json_io::load_json(&path).unwrap_err();
    assert!(matches!(err, JsonIoError::Parse(_)));
}

#[test]
fn test_load_json_file_not_found() {
    let path = std::path::Path::new("/nonexistent/path/file.json");
    let err = json_io::load_json(path).unwrap_err();
    assert!(matches!(err, JsonIoError::Io(_)));
}

#[test]
fn test_save_and_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("roundtrip.json");

    let original = sample_data();
    json_io::save_json(&path, &original).unwrap();
    let loaded = json_io::load_json(&path).unwrap();

    assert_eq!(original, loaded);
}

#[test]
fn test_save_json_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.json");
    assert!(!path.exists());

    json_io::save_json(&path, &sample_data()).unwrap();
    assert!(path.exists());
}

#[test]
fn test_save_json_pretty_printed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pretty.json");

    json_io::save_json(&path, &sample_data()).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    // Pretty-printed JSON contains newlines
    assert!(content.contains('\n'));
}

#[test]
fn test_load_json_with_null_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nulls.json");
    std::fs::write(&path, r#"[{"name":"Alice","email":null}]"#).unwrap();

    let rows = json_io::load_json(&path).unwrap();
    assert_eq!(rows[0]["email"], Value::Null);
}

#[test]
fn test_load_json_with_mixed_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mixed.json");
    std::fs::write(&path, r#"[{"a":1,"b":2},{"b":3,"c":4}]"#).unwrap();

    let rows = json_io::load_json(&path).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].contains_key("a"));
    assert!(!rows[0].contains_key("c"));
    assert!(rows[1].contains_key("c"));
}

#[test]
fn test_load_json_from_fixtures() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let types_path = manifest_dir.join("tests").join("data").join("types.json");
    let mixed_path = manifest_dir
        .join("tests")
        .join("data")
        .join("mixed_keys.json");

    let types_rows = json_io::load_json(&types_path).unwrap();
    assert_eq!(types_rows.len(), 2);
    assert_eq!(types_rows[0]["name"], Value::String("Alice".to_string()));
    assert_eq!(types_rows[1]["active"], Value::Bool(false));

    let mixed_rows = json_io::load_json(&mixed_path).unwrap();
    assert_eq!(mixed_rows.len(), 3);
    assert!(mixed_rows[0].contains_key("name"));
    assert!(mixed_rows[1].contains_key("note"));
}
