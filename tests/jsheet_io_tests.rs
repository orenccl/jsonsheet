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
fn test_load_json_and_sidecar_with_legacy_indexed_format() {
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
fn test_save_sidecar_roundtrip_without_row_key() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("data.json");
    let rows = sample_rows();
    jsonsheet::io::json_io::save_json(&json_path, &rows).unwrap();

    let mut meta = JSheetMeta::default();
    assert!(meta.set_formula_for_cell(0, "age2", "age * 2".to_string()));
    meta.set_cell_style(0, "age", Some("#aa0000".to_string()), None);
    jsheet_io::save_sidecar_for_json(&json_path, &meta, &rows).unwrap();

    let loaded = jsheet_io::load_sidecar_with_data(&json_path, &rows).unwrap();
    assert_eq!(loaded.formula_for_cell(0, "age2"), Some("age * 2"));
    assert_eq!(
        loaded.cell_style(0, "age").and_then(|s| s.color.clone()),
        Some("#aa0000".to_string())
    );
}

#[test]
fn test_save_sidecar_roundtrip_with_row_key() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("data.json");
    let rows = vec![
        BTreeMap::from([
            ("id".to_string(), Value::Number(1.into())),
            ("name".to_string(), Value::String("Alice".to_string())),
            ("age".to_string(), Value::Number(30.into())),
        ]),
        BTreeMap::from([
            ("id".to_string(), Value::Number(2.into())),
            ("name".to_string(), Value::String("Bob".to_string())),
            ("age".to_string(), Value::Number(25.into())),
        ]),
    ];
    jsonsheet::io::json_io::save_json(&json_path, &rows).unwrap();

    let mut meta = JSheetMeta::default();
    meta.set_row_key(Some("id".to_string()));
    assert!(meta.set_formula_for_cell(0, "score", "age * 2".to_string()));
    assert!(meta.set_formula_for_cell(1, "score", "age * 3".to_string()));
    meta.set_cell_style(0, "name", Some("#ff0000".to_string()), None);
    jsheet_io::save_sidecar_for_json(&json_path, &meta, &rows).unwrap();

    // Read back the raw file to verify keyed format
    let sidecar_path = jsheet_io::sidecar_path_for_json(&json_path);
    let raw: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&sidecar_path).unwrap()).unwrap();
    assert!(raw["keyed_cell_formulas"]["1"].is_object());
    assert!(raw["keyed_cell_formulas"]["2"].is_object());
    // cell_formulas should be absent or empty (skipped when empty Vec)
    assert!(raw.get("cell_formulas").is_none() || raw["cell_formulas"].is_null());

    // Load back and verify correct row alignment
    let loaded = jsheet_io::load_sidecar_with_data(&json_path, &rows).unwrap();
    assert_eq!(loaded.formula_for_cell(0, "score"), Some("age * 2"));
    assert_eq!(loaded.formula_for_cell(1, "score"), Some("age * 3"));
    assert_eq!(
        loaded.cell_style(0, "name").and_then(|s| s.color.clone()),
        Some("#ff0000".to_string())
    );
}

#[test]
fn test_row_key_survives_row_reorder() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("data.json");
    let rows = vec![
        BTreeMap::from([
            ("id".to_string(), Value::Number(1.into())),
            ("val".to_string(), Value::Number(10.into())),
        ]),
        BTreeMap::from([
            ("id".to_string(), Value::Number(2.into())),
            ("val".to_string(), Value::Number(20.into())),
        ]),
    ];
    jsonsheet::io::json_io::save_json(&json_path, &rows).unwrap();

    let mut meta = JSheetMeta::default();
    meta.set_row_key(Some("id".to_string()));
    assert!(meta.set_formula_for_cell(0, "doubled", "val * 2".to_string()));
    assert!(meta.set_formula_for_cell(1, "doubled", "val * 3".to_string()));
    jsheet_io::save_sidecar_for_json(&json_path, &meta, &rows).unwrap();

    // Simulate row reorder: swap rows in JSON
    let reordered = vec![rows[1].clone(), rows[0].clone()];
    jsonsheet::io::json_io::save_json(&json_path, &reordered).unwrap();

    // Load with reordered data — formulas should follow their keyed rows
    let loaded = jsheet_io::load_sidecar_with_data(&json_path, &reordered).unwrap();
    // id=2 is now at index 0, had formula "val * 3"
    assert_eq!(loaded.formula_for_cell(0, "doubled"), Some("val * 3"));
    // id=1 is now at index 1, had formula "val * 2"
    assert_eq!(loaded.formula_for_cell(1, "doubled"), Some("val * 2"));
}

#[test]
fn test_column_order_stored_and_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("data.json");
    let rows = sample_rows();
    jsonsheet::io::json_io::save_json(&json_path, &rows).unwrap();

    let mut meta = JSheetMeta::default();
    meta.set_column_order(vec!["age".to_string(), "name".to_string()]);
    jsheet_io::save_sidecar_for_json(&json_path, &meta, &rows).unwrap();

    let loaded = jsheet_io::load_sidecar_with_data(&json_path, &rows).unwrap();
    assert_eq!(loaded.column_order, vec!["age", "name"]);
}

#[test]
fn test_column_order_affects_display_columns() {
    let rows = vec![BTreeMap::from([
        ("c".to_string(), Value::Number(1.into())),
        ("a".to_string(), Value::Number(2.into())),
        ("b".to_string(), Value::Number(3.into())),
    ])];

    // Without column_order: alphabetical (BTreeSet)
    let meta_default = JSheetMeta::default();
    let cols = meta_default.display_columns(&rows);
    assert_eq!(cols, vec!["a", "b", "c"]);

    // With column_order: respects order, appends new
    let mut meta_ordered = JSheetMeta::default();
    meta_ordered.set_column_order(vec!["c".to_string(), "a".to_string()]);
    let cols = meta_ordered.display_columns(&rows);
    assert_eq!(cols, vec!["c", "a", "b"]); // b appended
}

#[test]
fn test_auto_detect_row_key() {
    let rows = vec![
        BTreeMap::from([
            ("id".to_string(), Value::Number(1.into())),
            ("name".to_string(), Value::String("Alice".to_string())),
        ]),
        BTreeMap::from([
            ("id".to_string(), Value::Number(2.into())),
            ("name".to_string(), Value::String("Bob".to_string())),
        ]),
    ];

    let mut meta = JSheetMeta::default();
    meta.auto_detect_row_key(&rows);
    // "id" comes first alphabetically and has unique values
    assert_eq!(meta.row_key(), Some("id"));
}
