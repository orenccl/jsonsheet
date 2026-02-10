use dioxus::prelude::*;
use std::path::PathBuf;

use crate::io::json_io;
use crate::state::data_model;
use crate::state::table_state::TableState;

#[component]
pub fn Toolbar(
    data: Signal<TableState>,
    file_path: Signal<Option<PathBuf>>,
    error_message: Signal<Option<String>>,
    selected_row: Signal<Option<usize>>,
    selected_column: Signal<Option<String>>,
) -> Element {
    let mut new_column = use_signal(String::new);
    let snapshot = data.read().clone();
    let columns = data_model::derive_columns(snapshot.data());
    let can_undo = snapshot.can_undo();
    let can_redo = snapshot.can_redo();
    let filter_column_value = snapshot.filter_column().unwrap_or("").to_string();
    let filter_query_value = snapshot.filter_query().to_string();
    let search_query_value = snapshot.search_query().to_string();

    rsx! {
        div { class: "toolbar",
            button {
                class: "toolbar-btn",
                id: "btn-open",
                onclick: move |_| {
                    spawn(async move {
                        open_file(data, file_path, error_message, selected_row, selected_column).await;
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
                id: "btn-undo",
                disabled: !can_undo,
                onclick: move |_| {
                    let undone = data.with_mut(|state| state.undo());
                    if undone {
                        selected_row.set(None);
                        selected_column.set(None);
                        error_message.set(None);
                    }
                },
                "Undo"
            }
            button {
                class: "toolbar-btn",
                id: "btn-redo",
                disabled: !can_redo,
                onclick: move |_| {
                    let redone = data.with_mut(|state| state.redo());
                    if redone {
                        selected_row.set(None);
                        selected_column.set(None);
                        error_message.set(None);
                    }
                },
                "Redo"
            }
            button {
                class: "toolbar-btn",
                id: "btn-add-row",
                onclick: move |_| {
                    let mut new_index = None;
                    data.with_mut(|state| {
                        if state.add_row() {
                            new_index = state.data().len().checked_sub(1);
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
                        let removed = data.with_mut(|state| state.delete_row(row_index));
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
            select {
                class: "toolbar-select",
                id: "select-filter-column",
                value: "{filter_column_value}",
                onchange: move |evt| {
                    let value = evt.value();
                    data.with_mut(|state| {
                        let query = state.filter_query().to_string();
                        let col = if value.is_empty() { None } else { Some(value) };
                        state.set_filter(col, query);
                    });
                },
                option { value: "", "Filter column..." }
                for col in &columns {
                    option { value: "{col}", "{col}" }
                }
            }
            input {
                class: "toolbar-input",
                id: "input-filter-query",
                placeholder: "Filter value",
                value: "{filter_query_value}",
                oninput: move |evt| {
                    let query = evt.value();
                    data.with_mut(|state| {
                        let col = state.filter_column().map(|c| c.to_string());
                        state.set_filter(col, query);
                    });
                }
            }
            button {
                class: "toolbar-btn",
                id: "btn-clear-filter",
                onclick: move |_| {
                    data.with_mut(|state| {
                        state.clear_filter();
                    });
                },
                "Clear Filter"
            }
            input {
                class: "toolbar-input",
                id: "input-search-query",
                placeholder: "Search all cells",
                value: "{search_query_value}",
                oninput: move |evt| {
                    let query = evt.value();
                    data.with_mut(|state| {
                        state.set_search(query);
                    });
                }
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

                    let added = data.with_mut(|state| state.add_column(&name));
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
                        let removed = data.with_mut(|state| state.delete_column(&col));
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
    mut data: Signal<TableState>,
    mut file_path: Signal<Option<PathBuf>>,
    mut error_message: Signal<Option<String>>,
    mut selected_row: Signal<Option<usize>>,
    mut selected_column: Signal<Option<String>>,
) {
    let task = rfd::AsyncFileDialog::new()
        .add_filter("JSON", &["json"])
        .pick_file()
        .await;

    if let Some(handle) = task {
        let path = handle.path().to_path_buf();
        match json_io::load_json(&path) {
            Ok(rows) => {
                data.with_mut(|state| {
                    state.replace_data(rows);
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

fn save_file(
    data: Signal<TableState>,
    file_path: Signal<Option<PathBuf>>,
    mut error_message: Signal<Option<String>>,
) {
    if let Some(path) = file_path.read().as_ref() {
        match json_io::save_json(path, data.read().data()) {
            Ok(()) => {
                error_message.set(None);
            }
            Err(e) => {
                error_message.set(Some(e.to_string()));
            }
        }
    }
}
