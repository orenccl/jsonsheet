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
        r##"{
          "columns": { "age": { "type": "number" } },
          "cell_formulas": [
            { "age2": "age * 2" }
          ],
          "comment_columns": ["note"],
          "comment_rows": [{ "note": "internal" }],
          "cell_styles": [
            { "age": { "color": "#aa0000" } }
          ]
        }"##,
    )
    .unwrap();

    let (rows, meta) = jsheet_io::load_json_and_sidecar(&json_path).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        meta.columns.get("age").unwrap().value_type,
        ColumnType::Number
    );
    assert_eq!(meta.formula_for_cell(0, "age2"), Some("age * 2"));
    assert!(meta.comment_columns.contains("note"));
    assert_eq!(
        meta.comment_rows[0]["note"],
        Value::String("internal".to_string())
    );
    assert_eq!(
        meta.cell_style(0, "age").and_then(|s| s.color.clone()),
        Some("#aa0000".to_string())
    );
}

#[test]
fn test_save_sidecar_for_json_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("data.json");
    jsonsheet::io::json_io::save_json(&json_path, &sample_rows()).unwrap();

    let mut meta = JSheetMeta::default();
    assert!(meta.set_formula_for_cell(0, "age2", "age * 2".to_string()));
    meta.set_cell_style(0, "age", Some("#aa0000".to_string()), None);
    jsheet_io::save_sidecar_for_json(&json_path, &meta).unwrap();

    let loaded = jsheet_io::load_sidecar_for_json(&json_path).unwrap();
    assert_eq!(loaded.formula_for_cell(0, "age2"), Some("age * 2"));
    assert_eq!(
        loaded.cell_style(0, "age").and_then(|s| s.color.clone()),
        Some("#aa0000".to_string())
    );
}
