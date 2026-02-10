use serde_json::Value;
use std::collections::BTreeMap;

use jsonsheet::state::jsheet::{ColumnType, SummaryKind};
use jsonsheet::state::table_state::TableState;

fn sample_state() -> TableState {
    TableState::from_data(vec![
        BTreeMap::from([
            ("name".to_string(), Value::String("Alice".to_string())),
            ("age".to_string(), Value::Number(30.into())),
        ]),
        BTreeMap::from([
            ("name".to_string(), Value::String("Bob".to_string())),
            ("age".to_string(), Value::Number(25.into())),
        ]),
    ])
}

#[test]
fn test_type_constraint_blocks_invalid_input() {
    let mut state = sample_state();
    state.set_column_type("age", Some(ColumnType::Number));

    assert!(!state.set_cell_from_input(0, "age", "not_a_number"));
    assert_eq!(state.data()[0]["age"], Value::Number(30.into()));
}

#[test]
fn test_type_constraint_coerces_valid_input() {
    let mut state = sample_state();
    state.set_column_type("age", Some(ColumnType::Number));

    assert!(state.set_cell_from_input(0, "age", "42"));
    assert_eq!(state.data()[0]["age"], Value::Number(42.into()));
}

#[test]
fn test_computed_column_display_and_export_baked_by_default() {
    let mut state = sample_state();
    assert!(state.set_computed_column("age2", "age * 2".to_string()));

    assert_eq!(state.cell_display_value(0, "age2"), "60");
    assert_eq!(state.cell_display_value(1, "age2"), "50");

    let exported = state.export_json_data().unwrap();
    assert_eq!(exported[0]["age2"], Value::Number(60.into()));
    assert_eq!(exported[1]["age2"], Value::Number(50.into()));
}

#[test]
fn test_summary_values_for_base_and_computed_columns() {
    let mut state = sample_state();
    assert!(state.set_computed_column("age2", "age * 2".to_string()));
    state.set_summary_kind("age", Some(SummaryKind::Avg));
    state.set_summary_kind("age2", Some(SummaryKind::Sum));

    assert_eq!(
        state.summary_display_for_column("age"),
        Some("27.5".to_string())
    );
    assert_eq!(
        state.summary_display_for_column("age2"),
        Some("110".to_string())
    );
}

#[test]
fn test_style_inline_output() {
    let mut state = sample_state();
    state.set_column_style(
        "age",
        Some("#aa0000".to_string()),
        Some("#f0f0f0".to_string()),
    );

    let style = state.column_style("age").unwrap_or_default();
    assert_eq!(style.color.as_deref(), Some("#aa0000"));
    assert_eq!(style.background.as_deref(), Some("#f0f0f0"));

    let inline = state.column_inline_style("age");
    assert!(inline.contains("color: #aa0000;"));
    assert!(inline.contains("background-color: #f0f0f0;"));
}

#[test]
fn test_comment_column_excluded_from_export_json() {
    let mut state = sample_state();
    assert!(state.add_column("note"));
    assert!(state.set_cell_from_input(0, "note", "keep in jsheet only"));
    state.set_comment_column("note", true);

    let exported = state.export_json_data().unwrap();
    assert!(!exported[0].contains_key("note"));
    assert!(!exported[1].contains_key("note"));
}

#[test]
fn test_comment_rows_are_captured_for_sidecar_save() {
    let mut state = sample_state();
    assert!(state.add_column("note"));
    assert!(state.set_cell_from_input(0, "note", "internal"));
    state.set_comment_column("note", true);

    let meta = state.jsheet_meta_for_save();
    assert!(meta.comment_columns.contains("note"));
    assert_eq!(
        meta.comment_rows[0]["note"],
        Value::String("internal".to_string())
    );
}

#[test]
fn test_set_computed_column_accepts_excel_style_formula_input() {
    let mut state = sample_state();
    assert!(state.set_computed_column("age2", "=age * 2".to_string()));
    assert_eq!(state.cell_display_value(0, "age2"), "60");
}

#[test]
fn test_computed_formula_takes_priority_over_existing_json_value() {
    let mut state = sample_state();
    assert!(state.add_column("age2"));
    assert!(state.set_cell_from_input(0, "age2", "1"));
    assert!(state.set_computed_column("age2", "=age * 2".to_string()));

    assert_eq!(state.cell_display_value(0, "age2"), "60");
    let exported = state.export_json_data().unwrap();
    assert_eq!(exported[0]["age2"], Value::Number(60.into()));
}
