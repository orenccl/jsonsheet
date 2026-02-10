use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::io::json_io::{self, JsonIoError, Row};
use crate::state::jsheet::JSheetMeta;

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

pub fn load_json_and_sidecar(json_path: &Path) -> Result<(Vec<Row>, JSheetMeta), JSheetIoError> {
    let data = json_io::load_json(json_path)?;
    let meta = load_sidecar_for_json(json_path)?;
    Ok((data, meta))
}

pub fn load_sidecar_for_json(json_path: &Path) -> Result<JSheetMeta, JSheetIoError> {
    let path = sidecar_path_for_json(json_path);
    if !path.exists() {
        return Ok(JSheetMeta::default());
    }

    let content = fs::read_to_string(path)?;
    let meta = serde_json::from_str(&content)?;
    Ok(meta)
}

pub fn save_sidecar_for_json(json_path: &Path, meta: &JSheetMeta) -> Result<(), JSheetIoError> {
    let path = sidecar_path_for_json(json_path);
    let content = serde_json::to_string_pretty(meta)?;
    fs::write(path, content)?;
    Ok(())
}
