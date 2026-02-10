use serde_json::Value;
use std::collections::BTreeMap;

pub type Row = BTreeMap<String, Value>;
pub type TableData = Vec<Row>;

/// Parses a user input string into a JSON value.
///
/// # Examples
/// ```
/// use jsonsheet::state::data_model::parse_cell_input;
/// use serde_json::Value;
///
/// assert_eq!(parse_cell_input("true"), Value::Bool(true));
/// assert_eq!(parse_cell_input("42"), Value::Number(42.into()));
/// assert_eq!(parse_cell_input("hello"), Value::String("hello".to_string()));
/// ```
pub fn parse_cell_input(input: &str) -> Value {
    let trimmed = input.trim();

    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
        return Value::Null;
    }

    if trimmed.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }

    if trimmed.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }

    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        if let Ok(Value::String(s)) = serde_json::from_str::<Value>(trimmed) {
            return Value::String(s);
        }
    }

    if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2 {
        return Value::String(trimmed[1..trimmed.len() - 1].to_string());
    }

    if let Ok(int_val) = trimmed.parse::<i64>() {
        return Value::Number(int_val.into());
    }

    if let Ok(uint_val) = trimmed.parse::<u64>() {
        return Value::Number(uint_val.into());
    }

    if let Ok(float_val) = trimmed.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(float_val) {
            return Value::Number(num);
        }
    }

    Value::String(trimmed.to_string())
}

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

/// Updates a specific cell with a new value.
pub fn set_cell_value(data: &mut TableData, row_index: usize, column: &str, value: Value) -> bool {
    if let Some(row) = data.get_mut(row_index) {
        row.insert(column.to_string(), value);
        true
    } else {
        false
    }
}

/// Adds a new empty row, using existing columns.
pub fn add_row(data: &mut TableData) {
    let columns = derive_columns(data);
    let mut row = Row::new();
    for col in columns {
        row.insert(col, Value::Null);
    }
    data.push(row);
}

/// Deletes a row by index.
pub fn delete_row(data: &mut TableData, row_index: usize) -> bool {
    if row_index < data.len() {
        data.remove(row_index);
        true
    } else {
        false
    }
}

/// Adds a new column to all rows.
pub fn add_column(data: &mut TableData, name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Bootstrap empty sheets by materializing one row that contains the new column.
    if data.is_empty() {
        let mut row = Row::new();
        row.insert(trimmed.to_string(), Value::Null);
        data.push(row);
        return true;
    }

    if data.iter().any(|row| row.contains_key(trimmed)) {
        return false;
    }

    for row in data {
        row.insert(trimmed.to_string(), Value::Null);
    }

    true
}

/// Deletes a column from all rows.
pub fn delete_column(data: &mut TableData, name: &str) -> bool {
    let trimmed = name.trim();
    let mut removed = false;
    for row in data {
        removed |= row.remove(trimmed).is_some();
    }
    removed
}

/// Formats a JSON value for display in a table cell.
///
/// # Examples
/// ```
/// use jsonsheet::state::data_model::display_value;
/// use serde_json::json;
///
/// assert_eq!(display_value(&json!(true)), "true");
/// assert_eq!(display_value(&json!(123)), "123");
/// assert_eq!(display_value(&json!("hi")), "hi");
/// ```
pub fn display_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}
