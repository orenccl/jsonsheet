use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::io::json_io::{self, JsonIoError, Row};
use crate::state::jsheet::{
    ColumnConstraint, ColumnStyle, ConditionalFormat, JSheetMeta, SummaryKind,
};

#[derive(Debug)]
pub enum JSheetIoError {
    Json(JsonIoError),
    Io(io::Error),
    Parse(serde_json::Error),
}

impl std::fmt::Display for JSheetIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(err) => write!(f, "{err}"),
            Self::Io(err) => write!(f, "IO error: {err}"),
            Self::Parse(err) => write!(f, "JSheet parse error: {err}"),
        }
    }
}

impl std::error::Error for JSheetIoError {}

impl From<JsonIoError> for JSheetIoError {
    fn from(value: JsonIoError) -> Self {
        Self::Json(value)
    }
}

impl From<io::Error> for JSheetIoError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for JSheetIoError {
    fn from(value: serde_json::Error) -> Self {
        Self::Parse(value)
    }
}

pub fn sidecar_path_for_json(json_path: &Path) -> PathBuf {
    let mut os: OsString = json_path.as_os_str().to_os_string();
    os.push(".jsheet");
    PathBuf::from(os)
}

/// On-disk format for .jsheet files. When `row_key` is set, row-level metadata
/// is stored as `{ "key_value": { "col": ... } }` maps instead of arrays.
/// This prevents index misalignment when rows are edited outside the editor.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct JSheetFile {
    #[serde(default)]
    columns: BTreeMap<String, ColumnConstraint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    column_order: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    row_key: Option<String>,
    #[serde(default)]
    comment_columns: std::collections::BTreeSet<String>,
    #[serde(default)]
    summaries: BTreeMap<String, SummaryKind>,

    // Keyed format (used when row_key is set)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    keyed_cell_formulas: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    keyed_cell_styles: BTreeMap<String, BTreeMap<String, ColumnStyle>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    keyed_comment_rows: BTreeMap<String, Row>,

    // Legacy indexed format (used when row_key is not set, backward compat)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    cell_formulas: Vec<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    cell_styles: Vec<BTreeMap<String, ColumnStyle>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    comment_rows: Vec<Row>,

    // Conditional formatting (not row-level, stored directly)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    conditional_formats: Vec<ConditionalFormat>,
}

impl JSheetFile {
    fn into_meta(self, data: &[Row]) -> JSheetMeta {
        let row_count = data.len();

        let (cell_formulas, cell_styles, comment_rows) = if let Some(ref key) = self.row_key {
            // Convert keyed maps to Vec aligned with data row order
            let key_to_index: BTreeMap<String, usize> = data
                .iter()
                .enumerate()
                .filter_map(|(idx, row)| row.get(key).map(|v| (value_to_key_string(v), idx)))
                .collect();

            let mut formulas = vec![BTreeMap::new(); row_count];
            for (k, v) in &self.keyed_cell_formulas {
                if let Some(&idx) = key_to_index.get(k) {
                    formulas[idx] = v.clone();
                }
            }

            let mut styles = vec![BTreeMap::new(); row_count];
            for (k, v) in &self.keyed_cell_styles {
                if let Some(&idx) = key_to_index.get(k) {
                    styles[idx] = v.clone();
                }
            }

            let mut comments = vec![Row::new(); row_count];
            for (k, v) in &self.keyed_comment_rows {
                if let Some(&idx) = key_to_index.get(k) {
                    comments[idx] = v.clone();
                }
            }

            (formulas, styles, comments)
        } else {
            // Use legacy indexed arrays directly
            (self.cell_formulas, self.cell_styles, self.comment_rows)
        };

        JSheetMeta {
            columns: self.columns,
            column_order: self.column_order,
            row_key: self.row_key,
            comment_columns: self.comment_columns,
            comment_rows,
            summaries: self.summaries,
            cell_formulas,
            cell_styles,
            conditional_formats: self.conditional_formats,
        }
    }

    fn from_meta(meta: &JSheetMeta, data: &[Row]) -> Self {
        let (keyed_formulas, keyed_styles, keyed_comments, vec_formulas, vec_styles, vec_comments) =
            if let Some(ref key) = meta.row_key {
                // Convert Vec to keyed maps
                let mut kf = BTreeMap::new();
                let mut ks = BTreeMap::new();
                let mut kc = BTreeMap::new();

                for (idx, row) in data.iter().enumerate() {
                    let key_val = row
                        .get(key)
                        .map(value_to_key_string)
                        .unwrap_or_else(|| idx.to_string());

                    if let Some(f) = meta.cell_formulas.get(idx) {
                        if !f.is_empty() {
                            kf.insert(key_val.clone(), f.clone());
                        }
                    }
                    if let Some(s) = meta.cell_styles.get(idx) {
                        if !s.is_empty() {
                            ks.insert(key_val.clone(), s.clone());
                        }
                    }
                    if let Some(c) = meta.comment_rows.get(idx) {
                        if !c.is_empty() {
                            kc.insert(key_val.clone(), c.clone());
                        }
                    }
                }

                (kf, ks, kc, vec![], vec![], vec![])
            } else {
                (
                    BTreeMap::new(),
                    BTreeMap::new(),
                    BTreeMap::new(),
                    meta.cell_formulas.clone(),
                    meta.cell_styles.clone(),
                    meta.comment_rows.clone(),
                )
            };

        JSheetFile {
            columns: meta.columns.clone(),
            column_order: meta.column_order.clone(),
            row_key: meta.row_key.clone(),
            comment_columns: meta.comment_columns.clone(),
            summaries: meta.summaries.clone(),
            keyed_cell_formulas: keyed_formulas,
            keyed_cell_styles: keyed_styles,
            keyed_comment_rows: keyed_comments,
            cell_formulas: vec_formulas,
            cell_styles: vec_styles,
            comment_rows: vec_comments,
            conditional_formats: meta.conditional_formats.clone(),
        }
    }
}

fn value_to_key_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => v.to_string(),
    }
}

pub fn load_json_and_sidecar(json_path: &Path) -> Result<(Vec<Row>, JSheetMeta), JSheetIoError> {
    let data = json_io::load_json(json_path)?;
    let meta = load_sidecar_with_data(json_path, &data)?;
    Ok((data, meta))
}

pub fn load_sidecar_with_data(json_path: &Path, data: &[Row]) -> Result<JSheetMeta, JSheetIoError> {
    let path = sidecar_path_for_json(json_path);
    if !path.exists() {
        return Ok(JSheetMeta::default());
    }

    let content = fs::read_to_string(path)?;
    let file: JSheetFile = serde_json::from_str(&content)?;
    Ok(file.into_meta(data))
}

pub fn save_sidecar_for_json(
    json_path: &Path,
    meta: &JSheetMeta,
    data: &[Row],
) -> Result<(), JSheetIoError> {
    let path = sidecar_path_for_json(json_path);
    let file = JSheetFile::from_meta(meta, data);
    let content = serde_json::to_string_pretty(&file)?;
    fs::write(path, content)?;
    Ok(())
}
