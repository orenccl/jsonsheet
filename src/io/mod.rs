pub mod jsheet_io;
pub mod json_io;

use std::io::{self, Write};
use std::path::Path;

use tempfile::NamedTempFile;

pub(crate) fn atomic_write_string(path: &Path, content: &str) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temp_file = NamedTempFile::new_in(parent)?;
    temp_file.write_all(content.as_bytes())?;
    temp_file.as_file().sync_all()?;

    match temp_file.persist(path) {
        Ok(_) => Ok(()),
        Err(err) => {
            if err.error.kind() == io::ErrorKind::AlreadyExists {
                std::fs::remove_file(path)?;
                err.file.persist(path).map(|_| ()).map_err(|e| e.error)
            } else {
                Err(err.error)
            }
        }
    }
}
