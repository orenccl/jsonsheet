use dioxus::prelude::*;
use std::path::PathBuf;

use crate::io::{jsheet_io, json_io};
use crate::state::i18n::{self, Language};
use crate::state::table_state::TableState;

pub async fn open_file(
    mut data: Signal<TableState>,
    language: Signal<Language>,
    mut file_path: Signal<Option<PathBuf>>,
    mut error_message: Signal<Option<String>>,
    mut selected_row: Signal<Option<usize>>,
    mut selected_column: Signal<Option<String>>,
) {
    let task = rfd::AsyncFileDialog::new()
        .add_filter(i18n::tr(*language.read(), "dialog.json_filter"), &["json"])
        .pick_file()
        .await;

    if let Some(handle) = task {
        let path = handle.path().to_path_buf();
        match jsheet_io::load_json_and_sidecar(&path) {
            Ok((rows, jsheet_meta)) => {
                data.with_mut(|state| {
                    state.replace_data_and_jsheet(rows, jsheet_meta);
                });
                file_path.set(Some(path));
                error_message.set(None);
                selected_row.set(None);
                selected_column.set(None);
            }
            Err(e) => {
                error_message.set(Some(e.to_string()));
            }
        }
    }
}

pub fn save_file(
    data: Signal<TableState>,
    file_path: Signal<Option<PathBuf>>,
    mut error_message: Signal<Option<String>>,
) -> bool {
    let path = {
        let read = file_path.read();
        let Some(path) = read.as_ref() else {
            return false;
        };
        path.clone()
    };

    let export = match data.read().export_json_data() {
        Ok(export) => export,
        Err(err) => {
            error_message.set(Some(err));
            return false;
        }
    };

    if let Err(err) = json_io::save_json(&path, &export) {
        error_message.set(Some(err.to_string()));
        return false;
    }

    let meta_for_save = data.read().jsheet_meta_for_save();
    if let Err(err) = jsheet_io::save_sidecar_for_json(&path, &meta_for_save, &export) {
        error_message.set(Some(err.to_string()));
        return false;
    }

    error_message.set(None);
    true
}

pub fn persist_sidecar_if_possible(
    data: Signal<TableState>,
    file_path: Signal<Option<PathBuf>>,
    mut error_message: Signal<Option<String>>,
) {
    let path = {
        let read = file_path.read();
        let Some(path) = read.as_ref() else {
            return;
        };
        path.clone()
    };

    let state = data.read();
    let meta_for_save = state.jsheet_meta_for_save();
    let current_data = state.data();
    if let Err(err) = jsheet_io::save_sidecar_for_json(&path, &meta_for_save, current_data) {
        error_message.set(Some(err.to_string()));
    } else {
        error_message.set(None);
    }
}
