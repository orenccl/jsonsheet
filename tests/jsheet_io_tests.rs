use std::collections::BTreeMap;

use serde_json::Value;

use jsonsheet::io::jsheet_io;
use jsonsheet::state::jsheet::{ColumnType, JSheetMeta};

fn sample_rows() -> Vec<BTreeMap<String, Value>> {
    vec![BTreeMap::from([
        ("name".to_string(), Value::String("Alice".to_string())),
        ("age".to_string(), Value::Number(30.into())),
    ])]
}

#[test]
fn test_sidecar_path_suffix() {
    let path = std::path::Path::new("D:/tmp/demo.json");
    let sidecar = jsheet_io::sidecar_path_for_json(path);
    assert!(sidecar.to_string_lossy().ends_with("demo.json.jsheet"));
}

#[test]
fn test_load_json_and_sidecar_missing_sidecar_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("data.json");
    jsonsheet::io::json_io::save_json(&json_path, &sample_rows()).unwrap();

    let (rows, meta) = jsheet_io::load_json_and_sidecar(&json_path).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(meta, JSheetMeta::default());
}

#[test]
fn test_load_json_and_sidecar_with_sidecar() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("data.json");
    jsonsheet::io::json_io::save_json(&json_path, &sample_rows()).unwrap();

    let sidecar = jsheet_io::sidecar_path_for_json(&json_path);
    std::fs::write(
        &sidecar,
        r#"{
          "columns": { "age": { "type": "number" } },
          "computed_columns": { "age2": { "formula": "age * 2", "bake": false } }
        }"#,
    )
    .unwrap();

    let (rows, meta) = jsheet_io::load_json_and_sidecar(&json_path).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        meta.columns.get("age").unwrap().value_type,
        ColumnType::Number
    );
    assert_eq!(
        meta.computed_columns.get("age2").unwrap().formula,
        "age * 2".to_string()
    );
}

#[test]
fn test_save_sidecar_for_json_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("data.json");
    jsonsheet::io::json_io::save_json(&json_path, &sample_rows()).unwrap();

    let mut meta = JSheetMeta::default();
    meta.set_computed_column("age2".to_string(), "age * 2".to_string(), false);
    jsheet_io::save_sidecar_for_json(&json_path, &meta).unwrap();

    let loaded = jsheet_io::load_sidecar_for_json(&json_path).unwrap();
    assert_eq!(
        loaded.computed_columns.get("age2").unwrap().formula,
        "age * 2".to_string()
    );
}
