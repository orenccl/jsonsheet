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
fn test_computed_column_display_and_non_baked_export() {
    let mut state = sample_state();
    assert!(state.set_computed_column("age2", "age * 2".to_string(), false));

    assert_eq!(state.cell_display_value(0, "age2"), "60");
    assert_eq!(state.cell_display_value(1, "age2"), "50");

    let exported = state.export_json_data().unwrap();
    assert!(!exported[0].contains_key("age2"));
    assert!(!exported[1].contains_key("age2"));
}

#[test]
fn test_computed_column_baked_export() {
    let mut state = sample_state();
    assert!(state.set_computed_column("age2", "age * 2".to_string(), true));

    let exported = state.export_json_data().unwrap();
    assert_eq!(exported[0]["age2"], Value::Number(60.into()));
    assert_eq!(exported[1]["age2"], Value::Number(50.into()));
}

#[test]
fn test_summary_values_for_base_and_computed_columns() {
    let mut state = sample_state();
    assert!(state.set_computed_column("age2", "age * 2".to_string(), false));
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
