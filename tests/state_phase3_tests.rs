use serde_json::Value;
use std::collections::BTreeMap;

use jsonsheet::state::table_state::{SortOrder, TableState};

fn sample_state() -> TableState {
    TableState::from_data(vec![
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
        BTreeMap::from([
            ("name".to_string(), Value::String("Carol".to_string())),
            ("age".to_string(), Value::Number(40.into())),
            ("active".to_string(), Value::Bool(true)),
        ]),
    ])
}

#[test]
fn test_undo_redo_cell_edit() {
    let mut state = sample_state();

    assert!(state.set_cell_value(0, "name", Value::String("Zed".to_string())));
    assert_eq!(state.data()[0]["name"], Value::String("Zed".to_string()));
    assert!(state.can_undo());

    assert!(state.undo());
    assert_eq!(state.data()[0]["name"], Value::String("Alice".to_string()));
    assert!(state.can_redo());

    assert!(state.redo());
    assert_eq!(state.data()[0]["name"], Value::String("Zed".to_string()));
}

#[test]
fn test_redo_cleared_after_new_operation() {
    let mut state = sample_state();
    assert!(state.set_cell_value(0, "name", Value::String("A".to_string())));
    assert!(state.undo());
    assert!(state.can_redo());

    assert!(state.add_row());
    assert!(!state.can_redo());
}

#[test]
fn test_sort_toggle_asc_desc() {
    let mut state = sample_state();

    assert!(state.sort_by_column_toggle("age"));
    assert_eq!(state.data()[0]["name"], Value::String("Bob".to_string()));
    assert_eq!(
        state.sort_spec().map(|spec| spec.order.clone()),
        Some(SortOrder::Asc)
    );

    assert!(state.sort_by_column_toggle("age"));
    assert_eq!(state.data()[0]["name"], Value::String("Carol".to_string()));
    assert_eq!(
        state.sort_spec().map(|spec| spec.order.clone()),
        Some(SortOrder::Desc)
    );
}

#[test]
fn test_filter_by_column_value() {
    let mut state = sample_state();
    state.set_filter(Some("name".to_string()), "bo".to_string());

    let visible = state.visible_row_indices();
    assert_eq!(visible, vec![1]);
}

#[test]
fn test_search_full_table_cell_match() {
    let mut state = sample_state();
    state.set_search("ali".to_string());

    assert!(state.cell_matches_search(0, "name"));
    assert!(!state.cell_matches_search(1, "name"));
    assert!(!state.cell_matches_search(0, "age"));
}

#[test]
fn test_undo_redo_sort_restores_sort_state() {
    let mut state = sample_state();
    assert!(state.sort_by_column_toggle("name"));
    assert_eq!(
        state.sort_spec().map(|spec| spec.order.clone()),
        Some(SortOrder::Asc)
    );

    assert!(state.undo());
    assert_eq!(state.sort_spec(), None);

    assert!(state.redo());
    assert_eq!(
        state.sort_spec().map(|spec| spec.order.clone()),
        Some(SortOrder::Asc)
    );
}

#[test]
fn test_sort_large_integer_values_without_precision_loss() {
    let mut state = TableState::from_data(vec![
        BTreeMap::from([(
            "id".to_string(),
            Value::Number(9_007_199_254_740_993u64.into()),
        )]),
        BTreeMap::from([(
            "id".to_string(),
            Value::Number(9_007_199_254_740_992u64.into()),
        )]),
    ]);

    assert!(state.sort_by_column_toggle("id"));
    assert_eq!(
        state.data()[0]["id"],
        Value::Number(9_007_199_254_740_992u64.into())
    );
    assert_eq!(
        state.data()[1]["id"],
        Value::Number(9_007_199_254_740_993u64.into())
    );
}

#[test]
fn test_delete_filtered_column_clears_filter_state() {
    let mut state = sample_state();
    state.set_filter(Some("name".to_string()), "bo".to_string());
    assert_eq!(state.visible_row_indices(), vec![1]);

    assert!(state.delete_column("name"));
    assert_eq!(state.filter_column(), None);
    assert_eq!(state.filter_query(), "");
    assert_eq!(state.visible_row_indices(), vec![0, 1, 2]);
}
