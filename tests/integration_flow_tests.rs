use serde_json::Value;

use jsonsheet::io::json_io;
use jsonsheet::state::data_model;

fn load_fixture(name: &str) -> Vec<std::collections::BTreeMap<String, Value>> {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.join("tests").join("data").join(name);
    json_io::load_json(&path).unwrap()
}

#[test]
fn test_e2e_open_edit_save_roundtrip() {
    let mut data = load_fixture("types.json");

    data_model::set_cell_value(&mut data, 0, "name", Value::String("Zoe".to_string()));
    data_model::set_cell_value(&mut data, 1, "active", Value::Bool(false));
    data_model::add_column(&mut data, "department");
    data_model::set_cell_value(&mut data, 0, "department", Value::String("R&D".to_string()));
    data_model::add_row(&mut data);
    data_model::set_cell_value(&mut data, 2, "name", Value::String("New Hire".to_string()));
    data_model::delete_column(&mut data, "tags");
    data_model::delete_row(&mut data, 1);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("roundtrip.json");
    json_io::save_json(&path, &data).unwrap();

    let loaded = json_io::load_json(&path).unwrap();
    assert_eq!(data, loaded);
}

#[test]
fn test_e2e_open_mixed_keys_add_column() {
    let mut data = load_fixture("mixed_keys.json");
    assert!(data_model::add_column(&mut data, "status"));
    data_model::set_cell_value(&mut data, 0, "status", Value::String("ok".to_string()));

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mixed_out.json");
    json_io::save_json(&path, &data).unwrap();

    let reloaded = json_io::load_json(&path).unwrap();
    assert_eq!(data, reloaded);
}

#[test]
fn test_e2e_bootstrap_empty_file_by_adding_column() {
    let mut data = load_fixture("empty.json");
    assert!(data_model::add_column(&mut data, "id"));
    data_model::set_cell_value(&mut data, 0, "id", Value::Number(1.into()));

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty_bootstrap_out.json");
    json_io::save_json(&path, &data).unwrap();

    let reloaded = json_io::load_json(&path).unwrap();
    assert_eq!(reloaded.len(), 1);
    assert_eq!(reloaded[0]["id"], Value::Number(1.into()));
}
