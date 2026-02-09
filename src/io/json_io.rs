use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde_json::Value;

pub type Row = BTreeMap<String, Value>;

#[derive(Debug)]
pub enum JsonIoError {
    Io(io::Error),
    Parse(serde_json::Error),
    NotAnArray,
    NotArrayOfObjects,
}

impl std::fmt::Display for JsonIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonIoError::Io(e) => write!(f, "IO error: {e}"),
            JsonIoError::Parse(e) => write!(f, "JSON parse error: {e}"),
            JsonIoError::NotAnArray => write!(f, "JSON root is not an array"),
            JsonIoError::NotArrayOfObjects => {
                write!(f, "JSON array contains non-object elements")
            }
        }
    }
}

impl std::error::Error for JsonIoError {}

impl From<io::Error> for JsonIoError {
    fn from(e: io::Error) -> Self {
        JsonIoError::Io(e)
    }
}

impl From<serde_json::Error> for JsonIoError {
    fn from(e: serde_json::Error) -> Self {
        JsonIoError::Parse(e)
    }
}

pub fn load_json(path: &Path) -> Result<Vec<Row>, JsonIoError> {
    let content = fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&content)?;

    match value {
        Value::Array(arr) => {
            let mut rows = Vec::with_capacity(arr.len());
            for item in arr {
                match item {
                    Value::Object(map) => {
                        rows.push(map.into_iter().collect());
                    }
                    _ => return Err(JsonIoError::NotArrayOfObjects),
                }
            }
            Ok(rows)
        }
        _ => Err(JsonIoError::NotAnArray),
    }
}

pub fn save_json(path: &Path, data: &[Row]) -> Result<(), JsonIoError> {
    let array: Vec<Value> = data
        .iter()
        .map(|row| {
            let map: serde_json::Map<String, Value> =
                row.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            Value::Object(map)
        })
        .collect();

    let json = serde_json::to_string_pretty(&array)?;
    fs::write(path, json)?;
    Ok(())
}
