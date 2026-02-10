use dioxus::prelude::*;
use std::path::PathBuf;

use crate::io::json_io;
use crate::state::data_model::{self, TableData};

#[component]
pub fn Toolbar(
    data: Signal<TableData>,
    file_path: Signal<Option<PathBuf>>,
    error_message: Signal<Option<String>>,
    selected_row: Signal<Option<usize>>,
    selected_column: Signal<Option<String>>,
) -> Element {
    let mut new_column = use_signal(String::new);

    rsx! {
        div { class: "toolbar",
            button {
                class: "toolbar-btn",
                id: "btn-open",
                onclick: move |_| {
                    spawn(async move {
                        open_file(data, file_path, error_message).await;
                    });
                },
                "Open"
            }
            button {
                class: "toolbar-btn",
                id: "btn-save",
                disabled: file_path.read().is_none(),
                onclick: move |_| {
                    save_file(data, file_path, error_message);
                },
                "Save"
            }
            button {
                class: "toolbar-btn",
                id: "btn-add-row",
                onclick: move |_| {
                    let mut new_index = None;
                    data.with_mut(|rows| {
                        data_model::add_row(rows);
                        if !rows.is_empty() {
                            new_index = Some(rows.len() - 1);
                        }
                    });
                    if let Some(idx) = new_index {
                        selected_row.set(Some(idx));
                    }
                    error_message.set(None);
                },
                "Add Row"
            }
            button {
                class: "toolbar-btn",
                id: "btn-delete-row",
                disabled: selected_row.read().is_none(),
                onclick: move |_| {
                    let row_index = *selected_row.read();
                    if let Some(row_index) = row_index {
                        let removed = data.with_mut(|rows| data_model::delete_row(rows, row_index));
                        if removed {
                            selected_row.set(None);
                            error_message.set(None);
                        } else {
                            error_message.set(Some("Failed to delete row.".to_string()));
                        }
                    } else {
                        error_message.set(Some("Select a row to delete.".to_string()));
                    }
                },
                "Delete Row"
            }
            input {
                class: "toolbar-input",
                id: "input-new-column",
                placeholder: "New column",
                value: "{new_column.read()}",
                oninput: move |evt| {
                    new_column.set(evt.value());
                }
            }
            button {
                class: "toolbar-btn",
                id: "btn-add-column",
                onclick: move |_| {
                    let name = new_column.read().trim().to_string();
                    if name.is_empty() {
                        error_message.set(Some("Column name is required.".to_string()));
                        return;
                    }

                    let added = data.with_mut(|rows| data_model::add_column(rows, &name));
                    if added {
                        selected_column.set(Some(name));
                        new_column.set(String::new());
                        error_message.set(None);
                    } else {
                        error_message.set(Some("Column already exists.".to_string()));
                    }
                },
                "Add Column"
            }
            button {
                class: "toolbar-btn",
                id: "btn-delete-column",
                disabled: selected_column.read().is_none(),
                onclick: move |_| {
                    let column = selected_column.read().clone();
                    if let Some(col) = column {
                        let removed = data.with_mut(|rows| data_model::delete_column(rows, &col));
                        if removed {
                            selected_column.set(None);
                            error_message.set(None);
                        } else {
                            error_message.set(Some("Failed to delete column.".to_string()));
                        }
                    } else {
                        error_message.set(Some("Select a column to delete.".to_string()));
                    }
                },
                "Delete Column"
            }
            if let Some(path) = file_path.read().as_ref() {
                span { class: "file-path", "{path.display()}" }
            }
            if let Some(err) = error_message.read().as_ref() {
                span { class: "error-message", "{err}" }
            }
        }
    }
}

async fn open_file(
    mut data: Signal<TableData>,
    mut file_path: Signal<Option<PathBuf>>,
    mut error_message: Signal<Option<String>>,
) {
    let task = rfd::AsyncFileDialog::new()
        .add_filter("JSON", &["json"])
        .pick_file()
        .await;

    if let Some(handle) = task {
        let path = handle.path().to_path_buf();
        match json_io::load_json(&path) {
            Ok(rows) => {
                data.set(rows);
                file_path.set(Some(path));
                error_message.set(None);
            }
            Err(e) => {
                error_message.set(Some(e.to_string()));
            }
        }
    }
}

fn save_file(
    data: Signal<TableData>,
    file_path: Signal<Option<PathBuf>>,
    mut error_message: Signal<Option<String>>,
) {
    if let Some(path) = file_path.read().as_ref() {
        match json_io::save_json(path, &data.read()) {
            Ok(()) => {
                error_message.set(None);
            }
            Err(e) => {
                error_message.set(Some(e.to_string()));
            }
        }
    }
}
