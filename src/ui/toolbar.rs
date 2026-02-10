use dioxus::prelude::*;
use std::path::PathBuf;

use crate::io::{jsheet_io, json_io};
use crate::state::i18n::{self, Language};
use crate::state::table_state::TableState;

#[component]
pub fn Toolbar(
    data: Signal<TableState>,
    language: Signal<Language>,
    file_path: Signal<Option<PathBuf>>,
    error_message: Signal<Option<String>>,
    selected_row: Signal<Option<usize>>,
    selected_column: Signal<Option<String>>,
) -> Element {
    let mut new_column = use_signal(String::new);

    let snapshot = data.read().clone();
    let current_language = *language.read();
    let columns = snapshot.display_columns();
    let can_undo = snapshot.can_undo();
    let can_redo = snapshot.can_redo();
    let filter_column_value = snapshot.filter_column().unwrap_or("").to_string();
    let filter_query_value = snapshot.filter_query().to_string();
    let search_query_value = snapshot.search_query().to_string();

    let language_label = i18n::tr(current_language, "toolbar.language_label");
    let open_label = i18n::tr(current_language, "toolbar.open");
    let save_label = i18n::tr(current_language, "toolbar.save");
    let undo_label = i18n::tr(current_language, "toolbar.undo");
    let redo_label = i18n::tr(current_language, "toolbar.redo");
    let add_row_label = i18n::tr(current_language, "toolbar.add_row");
    let delete_row_label = i18n::tr(current_language, "toolbar.delete_row");
    let filter_column_placeholder = i18n::tr(current_language, "toolbar.filter_column_placeholder");
    let filter_value_placeholder = i18n::tr(current_language, "toolbar.filter_value_placeholder");
    let clear_filter_label = i18n::tr(current_language, "toolbar.clear_filter");
    let search_placeholder = i18n::tr(current_language, "toolbar.search_placeholder");
    let new_column_placeholder = i18n::tr(current_language, "toolbar.new_column_placeholder");
    let add_column_label = i18n::tr(current_language, "toolbar.add_column");
    let delete_column_label = i18n::tr(current_language, "toolbar.delete_column");
    let selected_column_label = i18n::tr(current_language, "toolbar.selected_column");

    rsx! {
        div { class: "toolbar",
            span { class: "toolbar-label", "{language_label}" }
            select {
                class: "toolbar-select",
                id: "select-language",
                value: "{current_language.code()}",
                onchange: move |evt| {
                    if let Some(next_language) = Language::from_code(&evt.value()) {
                        language.set(next_language);
                    }
                },
                for lang in Language::all().iter().copied() {
                    option { value: "{lang.code()}", "{i18n::tr(current_language, lang.label_key())}" }
                }
            }
            button {
                class: "toolbar-btn",
                id: "btn-open",
                onclick: move |_| {
                    spawn(async move {
                        open_file(data, language, file_path, error_message, selected_row, selected_column)
                            .await;
                    });
                },
                "{open_label}"
            }
            button {
                class: "toolbar-btn",
                id: "btn-save",
                disabled: file_path.read().is_none(),
                onclick: move |_| {
                    save_file(data, file_path, error_message);
                },
                "{save_label}"
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
                "{undo_label}"
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
                "{redo_label}"
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
                "{add_row_label}"
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
                            error_message.set(Some(
                                i18n::tr(*language.read(), "error.delete_row_failed").to_string(),
                            ));
                        }
                    } else {
                        error_message.set(Some(
                            i18n::tr(*language.read(), "error.select_row_to_delete").to_string(),
                        ));
                    }
                },
                "{delete_row_label}"
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
                option { value: "", "{filter_column_placeholder}" }
                for col in &columns {
                    option { value: "{col}", "{col}" }
                }
            }
            input {
                class: "toolbar-input",
                id: "input-filter-query",
                placeholder: "{filter_value_placeholder}",
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
                "{clear_filter_label}"
            }
            input {
                class: "toolbar-input",
                id: "input-search-query",
                placeholder: "{search_placeholder}",
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
                placeholder: "{new_column_placeholder}",
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
                        error_message.set(Some(
                            i18n::tr(*language.read(), "error.column_name_required").to_string(),
                        ));
                        return;
                    }

                    let added = data.with_mut(|state| state.add_column(&name));
                    if added {
                        selected_column.set(Some(name));
                        new_column.set(String::new());
                        error_message.set(None);
                    } else {
                        error_message
                            .set(Some(i18n::tr(*language.read(), "error.column_exists").to_string()));
                    }
                },
                "{add_column_label}"
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
                            persist_sidecar_if_possible(data, file_path, error_message);
                        } else {
                            error_message.set(Some(
                                i18n::tr(*language.read(), "error.delete_column_failed").to_string(),
                            ));
                        }
                    } else {
                        error_message.set(Some(
                            i18n::tr(*language.read(), "error.select_column_to_delete").to_string(),
                        ));
                    }
                },
                "{delete_column_label}"
            }
            if let Some(col) = selected_column.read().as_ref() {
                span {
                    class: "toolbar-label",
                    id: "label-selected-column",
                    "{selected_column_label}: {col}"
                }
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

fn save_file(
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

    let export = match data.read().export_json_data() {
        Ok(export) => export,
        Err(err) => {
            error_message.set(Some(err));
            return;
        }
    };

    if let Err(err) = json_io::save_json(&path, &export) {
        error_message.set(Some(err.to_string()));
        return;
    }

    let meta_for_save = data.read().jsheet_meta_for_save();
    if let Err(err) = jsheet_io::save_sidecar_for_json(&path, &meta_for_save) {
        error_message.set(Some(err.to_string()));
        return;
    }

    error_message.set(None);
}

fn persist_sidecar_if_possible(
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

    let meta_for_save = data.read().jsheet_meta_for_save();
    if let Err(err) = jsheet_io::save_sidecar_for_json(&path, &meta_for_save) {
        error_message.set(Some(err.to_string()));
    } else {
        error_message.set(None);
    }
}
