use std::collections::BTreeMap;
use serde_json::Value;

pub type Row = BTreeMap<String, Value>;
pub type TableData = Vec<Row>;

/// Returns the sorted union of all keys across all rows.
pub fn derive_columns(data: &TableData) -> Vec<String> {
    let mut cols = std::collections::BTreeSet::new();
    for row in data {
        for key in row.keys() {
            cols.insert(key.clone());
        }
    }
    cols.into_iter().collect()
}

/// Formats a JSON value for display in a table cell.
pub fn display_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}
