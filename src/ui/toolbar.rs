use dioxus::prelude::*;
use std::path::PathBuf;

use crate::io::{jsheet_io, json_io};
use crate::state::i18n::{self, Language};
use crate::state::jsheet::{ColumnType, SummaryKind};
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
    let mut computed_column = use_signal(String::new);
    let mut computed_formula = use_signal(String::new);
    let mut computed_bake = use_signal(|| false);

    let snapshot = data.read().clone();
    let current_language = *language.read();
    let columns = snapshot.display_columns();
    let can_undo = snapshot.can_undo();
    let can_redo = snapshot.can_redo();
    let filter_column_value = snapshot.filter_column().unwrap_or("").to_string();
    let filter_query_value = snapshot.filter_query().to_string();
    let search_query_value = snapshot.search_query().to_string();
    let selected_col = selected_column.read().clone();
    let selected_col_name = selected_col.clone().unwrap_or_default();
    let selected_col_name_for_type = selected_col_name.clone();
    let selected_col_name_for_summary = selected_col_name.clone();
    let selected_col_name_for_style_color = selected_col_name.clone();
    let selected_col_name_for_style_background = selected_col_name.clone();
    let selected_col_type = selected_col
        .as_deref()
        .and_then(|col| snapshot.column_type(col))
        .map(column_type_value)
        .unwrap_or("none")
        .to_string();
    let selected_summary = selected_col
        .as_deref()
        .and_then(|col| snapshot.summary_kind(col))
        .map(summary_kind_value)
        .unwrap_or("none")
        .to_string();
    let selected_style = selected_col
        .as_deref()
        .and_then(|col| snapshot.column_style(col))
        .unwrap_or_default();
    let style_color_value = selected_style.color.unwrap_or_default();
    let style_background_value = selected_style.background.unwrap_or_default();

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
    let column_type_label = i18n::tr(current_language, "toolbar.column_type_label");
    let summary_label = i18n::tr(current_language, "toolbar.summary_label");
    let style_color_placeholder = i18n::tr(current_language, "toolbar.style_color_placeholder");
    let style_background_placeholder =
        i18n::tr(current_language, "toolbar.style_background_placeholder");
    let computed_name_placeholder = i18n::tr(current_language, "toolbar.computed_name_placeholder");
    let computed_formula_placeholder =
        i18n::tr(current_language, "toolbar.computed_formula_placeholder");
    let computed_bake_label = i18n::tr(current_language, "toolbar.computed_bake_label");
    let apply_computed_label = i18n::tr(current_language, "toolbar.apply_computed");
    let remove_computed_label = i18n::tr(current_language, "toolbar.remove_computed");
    let no_column_selected = i18n::tr(current_language, "toolbar.no_column_selected");

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
            if let Some(col) = selected_col {
                span { class: "toolbar-label", id: "label-selected-column", "{selected_column_label}: {col}" }
                span { class: "toolbar-label", "{column_type_label}" }
                select {
                    class: "toolbar-select",
                    id: "select-column-type",
                    value: "{selected_col_type}",
                    onchange: move |evt| {
                        let value = evt.value();
                        data.with_mut(|state| {
                            state.set_column_type(&selected_col_name_for_type, parse_column_type(&value));
                        });
                        persist_sidecar_if_possible(data, file_path, error_message);
                    },
                    option { value: "none", "{i18n::tr(current_language, \"toolbar.option.none\")}" }
                    option { value: "string", "{i18n::tr(current_language, \"toolbar.option.type_string\")}" }
                    option { value: "number", "{i18n::tr(current_language, \"toolbar.option.type_number\")}" }
                    option { value: "bool", "{i18n::tr(current_language, \"toolbar.option.type_bool\")}" }
                    option { value: "null", "{i18n::tr(current_language, \"toolbar.option.type_null\")}" }
                }
                span { class: "toolbar-label", "{summary_label}" }
                select {
                    class: "toolbar-select",
                    id: "select-summary-kind",
                    value: "{selected_summary}",
                    onchange: move |evt| {
                        let value = evt.value();
                        data.with_mut(|state| {
                            state.set_summary_kind(
                                &selected_col_name_for_summary,
                                parse_summary_kind(&value),
                            );
                        });
                        persist_sidecar_if_possible(data, file_path, error_message);
                    },
                    option { value: "none", "{i18n::tr(current_language, \"toolbar.option.none\")}" }
                    option { value: "sum", "{i18n::tr(current_language, \"toolbar.option.summary_sum\")}" }
                    option { value: "avg", "{i18n::tr(current_language, \"toolbar.option.summary_avg\")}" }
                    option { value: "count", "{i18n::tr(current_language, \"toolbar.option.summary_count\")}" }
                    option { value: "min", "{i18n::tr(current_language, \"toolbar.option.summary_min\")}" }
                    option { value: "max", "{i18n::tr(current_language, \"toolbar.option.summary_max\")}" }
                }
                input {
                    class: "toolbar-input",
                    id: "input-style-color",
                    placeholder: "{style_color_placeholder}",
                    value: "{style_color_value}",
                    oninput: move |evt| {
                        let value = evt.value();
                        data.with_mut(|state| {
                            let current = state
                                .column_style(&selected_col_name_for_style_color)
                                .unwrap_or_default();
                            state.set_column_style(
                                &selected_col_name_for_style_color,
                                Some(value),
                                current.background,
                            );
                        });
                        persist_sidecar_if_possible(data, file_path, error_message);
                    }
                }
                input {
                    class: "toolbar-input",
                    id: "input-style-background",
                    placeholder: "{style_background_placeholder}",
                    value: "{style_background_value}",
                    oninput: move |evt| {
                        let value = evt.value();
                        data.with_mut(|state| {
                            let current = state
                                .column_style(&selected_col_name_for_style_background)
                                .unwrap_or_default();
                            state.set_column_style(
                                &selected_col_name_for_style_background,
                                current.color,
                                Some(value),
                            );
                        });
                        persist_sidecar_if_possible(data, file_path, error_message);
                    }
                }
            } else {
                span { class: "toolbar-label", "{no_column_selected}" }
            }
            input {
                class: "toolbar-input",
                id: "input-computed-column",
                placeholder: "{computed_name_placeholder}",
                value: "{computed_column.read()}",
                oninput: move |evt| computed_column.set(evt.value())
            }
            input {
                class: "toolbar-input",
                id: "input-computed-formula",
                placeholder: "{computed_formula_placeholder}",
                value: "{computed_formula.read()}",
                oninput: move |evt| computed_formula.set(evt.value())
            }
            label {
                class: "toolbar-label",
                input {
                    id: "check-computed-bake",
                    r#type: "checkbox",
                    checked: *computed_bake.read(),
                    onchange: move |_| {
                        let current = *computed_bake.read();
                        computed_bake.set(!current);
                    }
                }
                "{computed_bake_label}"
            }
            button {
                class: "toolbar-btn",
                id: "btn-apply-computed",
                onclick: move |_| {
                    let column = computed_column.read().trim().to_string();
                    let formula = computed_formula.read().trim().to_string();
                    if column.is_empty() || formula.is_empty() {
                        error_message
                            .set(Some(i18n::tr(*language.read(), "error.column_name_required").to_string()));
                        return;
                    }
                    let bake = *computed_bake.read();
                    let ok = data.with_mut(|state| state.set_computed_column(&column, formula, bake));
                    if ok {
                        selected_column.set(Some(column));
                        error_message.set(None);
                        persist_sidecar_if_possible(data, file_path, error_message);
                    } else {
                        error_message.set(Some(
                            i18n::tr(*language.read(), "error.invalid_computed_formula").to_string(),
                        ));
                    }
                },
                "{apply_computed_label}"
            }
            button {
                class: "toolbar-btn",
                id: "btn-remove-computed",
                onclick: move |_| {
                    let column = computed_column.read().trim().to_string();
                    if column.is_empty() {
                        return;
                    }
                    data.with_mut(|state| state.remove_computed_column(&column));
                    if selected_column.read().as_deref() == Some(column.as_str()) {
                        selected_column.set(None);
                    }
                    persist_sidecar_if_possible(data, file_path, error_message);
                },
                "{remove_computed_label}"
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

    if let Err(err) = jsheet_io::save_sidecar_for_json(&path, data.read().jsheet_meta()) {
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
    if let Err(err) = jsheet_io::save_sidecar_for_json(&path, data.read().jsheet_meta()) {
        error_message.set(Some(err.to_string()));
    } else {
        error_message.set(None);
    }
}

fn column_type_value(column_type: ColumnType) -> &'static str {
    match column_type {
        ColumnType::String => "string",
        ColumnType::Number => "number",
        ColumnType::Bool => "bool",
        ColumnType::Null => "null",
    }
}

fn parse_column_type(value: &str) -> Option<ColumnType> {
    match value {
        "string" => Some(ColumnType::String),
        "number" => Some(ColumnType::Number),
        "bool" => Some(ColumnType::Bool),
        "null" => Some(ColumnType::Null),
        _ => None,
    }
}

fn summary_kind_value(summary_kind: SummaryKind) -> &'static str {
    match summary_kind {
        SummaryKind::Sum => "sum",
        SummaryKind::Avg => "avg",
        SummaryKind::Count => "count",
        SummaryKind::Min => "min",
        SummaryKind::Max => "max",
    }
}

fn parse_summary_kind(value: &str) -> Option<SummaryKind> {
    match value {
        "sum" => Some(SummaryKind::Sum),
        "avg" => Some(SummaryKind::Avg),
        "count" => Some(SummaryKind::Count),
        "min" => Some(SummaryKind::Min),
        "max" => Some(SummaryKind::Max),
        _ => None,
    }
}
