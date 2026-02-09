use std::collections::BTreeMap;
use serde_json::Value;

use jsonsheet::state::data_model::{self, Row, TableData};

#[test]
fn test_derive_columns_basic() {
    let data: TableData = vec![
        BTreeMap::from([
            ("name".to_string(), Value::String("Alice".to_string())),
            ("age".to_string(), Value::Number(30.into())),
        ]),
        BTreeMap::from([
            ("name".to_string(), Value::String("Bob".to_string())),
            ("age".to_string(), Value::Number(25.into())),
        ]),
    ];

    let cols = data_model::derive_columns(&data);
    assert_eq!(cols, vec!["age", "name"]);
}

#[test]
fn test_derive_columns_mixed_keys() {
    let data: TableData = vec![
        BTreeMap::from([
            ("a".to_string(), Value::Null),
            ("b".to_string(), Value::Null),
        ]),
        BTreeMap::from([
            ("b".to_string(), Value::Null),
            ("c".to_string(), Value::Null),
        ]),
    ];

    let cols = data_model::derive_columns(&data);
    assert_eq!(cols, vec!["a", "b", "c"]);
}

#[test]
fn test_derive_columns_empty() {
    let data: TableData = vec![];
    let cols = data_model::derive_columns(&data);
    assert!(cols.is_empty());
}

#[test]
fn test_derive_columns_single_row() {
    let data: TableData = vec![BTreeMap::from([
        ("x".to_string(), Value::Number(1.into())),
    ])];

    let cols = data_model::derive_columns(&data);
    assert_eq!(cols, vec!["x"]);
}

#[test]
fn test_derive_columns_sorted() {
    let data: TableData = vec![BTreeMap::from([
        ("zebra".to_string(), Value::Null),
        ("apple".to_string(), Value::Null),
        ("mango".to_string(), Value::Null),
    ])];

    let cols = data_model::derive_columns(&data);
    assert_eq!(cols, vec!["apple", "mango", "zebra"]);
}

#[test]
fn test_display_value_string() {
    let v = Value::String("hello".to_string());
    assert_eq!(data_model::display_value(&v), "hello");
}

#[test]
fn test_display_value_number() {
    let v = Value::Number(42.into());
    assert_eq!(data_model::display_value(&v), "42");
}

#[test]
fn test_display_value_bool() {
    assert_eq!(data_model::display_value(&Value::Bool(true)), "true");
    assert_eq!(data_model::display_value(&Value::Bool(false)), "false");
}

#[test]
fn test_display_value_null() {
    assert_eq!(data_model::display_value(&Value::Null), "");
}

#[test]
fn test_display_value_array() {
    let v: Value = serde_json::from_str("[1,2,3]").unwrap();
    assert_eq!(data_model::display_value(&v), "[1,2,3]");
}

#[test]
fn test_display_value_object() {
    let v: Value = serde_json::from_str(r#"{"key":"val"}"#).unwrap();
    assert_eq!(data_model::display_value(&v), r#"{"key":"val"}"#);
}
