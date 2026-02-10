use serde_json::Value;
use std::collections::BTreeMap;

use jsonsheet::state::data_model::{self, TableData};

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
    let data: TableData = vec![BTreeMap::from([("x".to_string(), Value::Number(1.into()))])];

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

#[test]
fn test_parse_cell_input_null_bool() {
    assert_eq!(data_model::parse_cell_input(""), Value::Null);
    assert_eq!(data_model::parse_cell_input("null"), Value::Null);
    assert_eq!(data_model::parse_cell_input("TRUE"), Value::Bool(true));
    assert_eq!(data_model::parse_cell_input("false"), Value::Bool(false));
}

#[test]
fn test_parse_cell_input_numbers() {
    assert_eq!(data_model::parse_cell_input("42"), Value::Number(42.into()));
    let float_val = data_model::parse_cell_input("3.14");
    match float_val {
        Value::Number(n) => {
            let expected = "3.14".parse::<f64>().unwrap();
            assert_eq!(n.as_f64().unwrap(), expected);
        }
        _ => panic!("expected number"),
    }
}

#[test]
fn test_parse_cell_input_strings() {
    assert_eq!(
        data_model::parse_cell_input("hello"),
        Value::String("hello".to_string())
    );
    assert_eq!(
        data_model::parse_cell_input("\"quoted\""),
        Value::String("quoted".to_string())
    );
    assert_eq!(
        data_model::parse_cell_input("'single'"),
        Value::String("single".to_string())
    );
}

#[test]
fn test_add_delete_row() {
    let mut data: TableData = vec![BTreeMap::from([("x".to_string(), Value::Null)])];
    data_model::add_row(&mut data);
    assert_eq!(data.len(), 2);
    assert!(data[1].contains_key("x"));

    assert!(data_model::delete_row(&mut data, 0));
    assert_eq!(data.len(), 1);
    assert!(!data_model::delete_row(&mut data, 99));
}

#[test]
fn test_add_delete_column() {
    let mut data: TableData = vec![BTreeMap::from([("a".to_string(), Value::Null)])];
    assert!(data_model::add_column(&mut data, "b"));
    assert!(data[0].contains_key("b"));
    assert!(!data_model::add_column(&mut data, "b"));

    assert!(data_model::delete_column(&mut data, "a"));
    assert!(!data[0].contains_key("a"));
}

#[test]
fn test_set_cell_value() {
    let mut data: TableData = vec![BTreeMap::from([("a".to_string(), Value::Null)])];
    assert!(data_model::set_cell_value(
        &mut data,
        0,
        "a",
        Value::String("ok".to_string())
    ));
    assert_eq!(data[0]["a"], Value::String("ok".to_string()));
    assert!(!data_model::set_cell_value(&mut data, 10, "a", Value::Null));
}

#[test]
fn test_add_column_invalid_name() {
    let mut data: TableData = vec![BTreeMap::from([("a".to_string(), Value::Null)])];
    assert!(!data_model::add_column(&mut data, ""));
    assert!(!data_model::add_column(&mut data, "   "));
}

#[test]
fn test_delete_column_missing() {
    let mut data: TableData = vec![BTreeMap::from([("a".to_string(), Value::Null)])];
    assert!(!data_model::delete_column(&mut data, "missing"));
}

#[test]
fn test_add_column_bootstraps_empty_table() {
    let mut data: TableData = vec![];
    assert!(data_model::add_column(&mut data, "first_col"));
    assert_eq!(data.len(), 1);
    assert_eq!(data_model::derive_columns(&data), vec!["first_col"]);
    assert_eq!(data[0]["first_col"], Value::Null);
}
