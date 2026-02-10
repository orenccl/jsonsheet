use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use dioxus::prelude::{Key, *};

use crate::io::jsheet_io;
use crate::state::data_model::{self, Row};
use crate::state::i18n::{self, Language};
use crate::state::jsheet::{ColumnType, SummaryKind};
use crate::state::table_state::{SortOrder, TableState};

#[derive(Clone, PartialEq)]
struct EditingCell {
    row: usize,
    column: String,
    draft: String,
}

enum CommitResult {
    Applied,
    InvalidFormula,
    InvalidTypedValue,
}

#[component]
pub fn Table(
    data: Signal<TableState>,
    language: Signal<Language>,
    file_path: Signal<Option<PathBuf>>,
    error_message: Signal<Option<String>>,
    selected_row: Signal<Option<usize>>,
    selected_column: Signal<Option<String>>,
) -> Element {
    let editing = use_signal::<Option<EditingCell>>(|| None);
    let snapshot = data.read().clone();
    let columns = snapshot.display_columns();
    let visible_rows = snapshot.visible_row_indices();
    let search_query = snapshot.search_query().to_string();
    let sort_spec = snapshot.sort_spec().cloned();
    let computed_columns: BTreeSet<String> = snapshot
        .jsheet_meta()
        .computed_columns
        .keys()
        .cloned()
        .collect();
    let computed_formulas: BTreeMap<String, String> = snapshot
        .jsheet_meta()
        .computed_columns
        .iter()
        .map(|(column, computed)| (column.clone(), computed.formula.clone()))
        .collect();
    let column_styles: BTreeMap<String, String> = columns
        .iter()
        .map(|col| (col.clone(), snapshot.column_inline_style(col)))
        .collect();
    let has_summary = columns
        .iter()
        .any(|column| snapshot.summary_kind(column).is_some());
    let current_language = *language.read();

    if columns.is_empty() {
        let empty_message = i18n::tr(current_language, "table.empty_message");
        return rsx! {
            p { class: "empty-message", id: "empty-message", "{empty_message}" }
        };
    }

    rsx! {
        div { class: "table-container", id: "table-container",
            table {
                thead {
                    tr {
                        th { class: "row-number", "#" }
                        for col in &columns {
                            th {
                                class: header_class(col, &sort_spec, &selected_column),
                                id: format!("col-{}", sanitize_id(col)),
                                style: "{column_styles.get(col).cloned().unwrap_or_default()}",
                                onclick: {
                                    let col_name = col.clone();
                                    let mut data = data;
                                    let mut selected_column = selected_column;
                                    move |_| {
                                        data.with_mut(|state| {
                                            state.sort_by_column_toggle(&col_name);
                                        });
                                        selected_column.set(Some(col_name.clone()));
                                    }
                                },
                                "{col}"
                            }
                        }
                    }
                    tr { class: "column-meta-row", id: "column-meta-row",
                        th { class: "row-number meta-label", "≡" }
                        for col in &columns {
                            ColumnMetaCell {
                                data,
                                language,
                                file_path,
                                error_message,
                                column: col.clone(),
                                selected_column,
                            }
                        }
                    }
                }
                tbody {
                    for (display_index, data_index) in visible_rows.iter().enumerate() {
                        if let Some(row) = snapshot.row_with_computed(*data_index) {
                            TableRow {
                                display_index,
                                data_index: *data_index,
                                row,
                                columns: columns.clone(),
                                computed_columns: computed_columns.clone(),
                                computed_formulas: computed_formulas.clone(),
                                column_styles: column_styles.clone(),
                                data,
                                language,
                                file_path,
                                error_message,
                                selected_row,
                                selected_column,
                                editing,
                                search_query: search_query.clone(),
                            }
                        }
                    }
                }
                if has_summary {
                    tfoot {
                        tr { class: "summary-row", id: "summary-row",
                            td { class: "row-number summary-label", "Σ" }
                            for col in &columns {
                                td {
                                    class: "summary-cell",
                                    id: format!("summary-{}", sanitize_id(col)),
                                    style: "{column_styles.get(col).cloned().unwrap_or_default()}",
                                    "{snapshot.summary_display_for_column(col).unwrap_or_default()}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ColumnMetaCell(
    data: Signal<TableState>,
    language: Signal<Language>,
    file_path: Signal<Option<PathBuf>>,
    error_message: Signal<Option<String>>,
    column: String,
    selected_column: Signal<Option<String>>,
) -> Element {
    let snapshot = data.read().clone();
    let column_type = snapshot
        .column_type(&column)
        .map(column_type_value)
        .unwrap_or("none")
        .to_string();
    let summary_kind = snapshot
        .summary_kind(&column)
        .map(summary_kind_value)
        .unwrap_or("none")
        .to_string();
    let is_comment = snapshot.is_comment_column(&column);
    let style = snapshot.column_style(&column).unwrap_or_default();
    let text_color = style.color.unwrap_or_else(|| "#1a1a1a".to_string());
    let background_color = style.background.unwrap_or_else(|| "#ffffff".to_string());
    let computed_formula = snapshot
        .computed_column(&column)
        .map(|computed| format!("={}", computed.formula))
        .unwrap_or_default();
    let current_language = *language.read();
    let option_none = i18n::tr(current_language, "toolbar.option.none");
    let option_type_string = i18n::tr(current_language, "toolbar.option.type_string");
    let option_type_number = i18n::tr(current_language, "toolbar.option.type_number");
    let option_type_bool = i18n::tr(current_language, "toolbar.option.type_bool");
    let option_type_null = i18n::tr(current_language, "toolbar.option.type_null");
    let option_summary_sum = i18n::tr(current_language, "toolbar.option.summary_sum");
    let option_summary_avg = i18n::tr(current_language, "toolbar.option.summary_avg");
    let option_summary_count = i18n::tr(current_language, "toolbar.option.summary_count");
    let option_summary_min = i18n::tr(current_language, "toolbar.option.summary_min");
    let option_summary_max = i18n::tr(current_language, "toolbar.option.summary_max");
    let meta_comment_label = i18n::tr(current_language, "table.meta_comment");
    let meta_style_reset_label = i18n::tr(current_language, "table.meta_style_reset");
    let meta_formula_hint_label = i18n::tr(current_language, "table.meta_formula_hint");
    let meta_focus_label = i18n::tr(current_language, "table.meta_focus");

    rsx! {
        th {
            class: "column-meta-cell",
            id: format!("meta-{}", sanitize_id(&column)),
            div { class: "meta-controls",
                select {
                    class: "meta-select",
                    id: format!("meta-type-{}", sanitize_id(&column)),
                    value: "{column_type}",
                    onchange: {
                        let column_name = column.clone();
                        let mut data = data;
                        move |evt: Event<FormData>| {
                            let value = evt.value();
                            data.with_mut(|state| {
                                state.set_column_type(&column_name, parse_column_type(&value));
                            });
                            persist_sidecar_if_possible(data, file_path, error_message);
                        }
                    },
                    option { value: "none", "{option_none}" }
                    option { value: "string", "{option_type_string}" }
                    option { value: "number", "{option_type_number}" }
                    option { value: "bool", "{option_type_bool}" }
                    option { value: "null", "{option_type_null}" }
                }
                select {
                    class: "meta-select",
                    id: format!("meta-summary-{}", sanitize_id(&column)),
                    value: "{summary_kind}",
                    onchange: {
                        let column_name = column.clone();
                        let mut data = data;
                        move |evt: Event<FormData>| {
                            let value = evt.value();
                            data.with_mut(|state| {
                                state.set_summary_kind(&column_name, parse_summary_kind(&value));
                            });
                            persist_sidecar_if_possible(data, file_path, error_message);
                        }
                    },
                    option { value: "none", "{option_none}" }
                    option { value: "sum", "{option_summary_sum}" }
                    option { value: "avg", "{option_summary_avg}" }
                    option { value: "count", "{option_summary_count}" }
                    option { value: "min", "{option_summary_min}" }
                    option { value: "max", "{option_summary_max}" }
                }
                label { class: "meta-check-label",
                    input {
                        id: format!("meta-comment-{}", sanitize_id(&column)),
                        r#type: "checkbox",
                        checked: is_comment,
                        onchange: {
                            let column_name = column.clone();
                            let mut data = data;
                            move |_| {
                                data.with_mut(|state| {
                                    let next = !state.is_comment_column(&column_name);
                                    state.set_comment_column(&column_name, next);
                                });
                                persist_sidecar_if_possible(data, file_path, error_message);
                            }
                        }
                    }
                    "{meta_comment_label}"
                }
                div { class: "meta-color-wrap",
                    span { class: "meta-color-label", "A" }
                    input {
                        class: "meta-color-input",
                        id: format!("meta-text-color-{}", sanitize_id(&column)),
                        r#type: "color",
                        value: "{text_color}",
                        oninput: {
                            let column_name = column.clone();
                            let mut data = data;
                            move |evt: Event<FormData>| {
                                let value = evt.value();
                                data.with_mut(|state| {
                                    let current = state.column_style(&column_name).unwrap_or_default();
                                    state.set_column_style(&column_name, Some(value), current.background);
                                });
                                persist_sidecar_if_possible(data, file_path, error_message);
                            }
                        }
                    }
                }
                div { class: "meta-color-wrap",
                    span { class: "meta-color-label", "Bg" }
                    input {
                        class: "meta-color-input",
                        id: format!("meta-bg-color-{}", sanitize_id(&column)),
                        r#type: "color",
                        value: "{background_color}",
                        oninput: {
                            let column_name = column.clone();
                            let mut data = data;
                            move |evt: Event<FormData>| {
                                let value = evt.value();
                                data.with_mut(|state| {
                                    let current = state.column_style(&column_name).unwrap_or_default();
                                    state.set_column_style(&column_name, current.color, Some(value));
                                });
                                persist_sidecar_if_possible(data, file_path, error_message);
                            }
                        }
                    }
                }
                button {
                    class: "meta-btn",
                    id: format!("meta-style-clear-{}", sanitize_id(&column)),
                    onclick: {
                        let column_name = column.clone();
                        let mut data = data;
                        move |_| {
                            data.with_mut(|state| {
                                state.set_column_style(&column_name, None, None);
                            });
                            persist_sidecar_if_possible(data, file_path, error_message);
                        }
                    },
                    "{meta_style_reset_label}"
                }
            }
            if !computed_formula.is_empty() {
                div { class: "meta-formula", "{computed_formula}" }
            }
            div {
                class: "meta-open-formula-help",
                "{meta_formula_hint_label}"
            }
            button {
                class: "meta-focus-btn",
                id: format!("meta-focus-{}", sanitize_id(&column)),
                onclick: move |_| {
                    selected_column.set(Some(column.clone()));
                },
                "{meta_focus_label}"
            }
        }
    }
}

#[component]
fn TableRow(
    display_index: usize,
    data_index: usize,
    row: Row,
    columns: Vec<String>,
    computed_columns: BTreeSet<String>,
    computed_formulas: BTreeMap<String, String>,
    column_styles: BTreeMap<String, String>,
    data: Signal<TableState>,
    language: Signal<Language>,
    file_path: Signal<Option<PathBuf>>,
    error_message: Signal<Option<String>>,
    selected_row: Signal<Option<usize>>,
    selected_column: Signal<Option<String>>,
    editing: Signal<Option<EditingCell>>,
    search_query: String,
) -> Element {
    let is_selected = selected_row
        .read()
        .as_ref()
        .map(|row_index| *row_index == data_index)
        .unwrap_or(false);
    let mut row_class = if display_index % 2 == 0 {
        "even"
    } else {
        "odd"
    }
    .to_string();
    if is_selected {
        row_class.push_str(" selected-row");
    }

    rsx! {
        tr { class: "{row_class}", id: format!("row-{data_index}"),
            td {
                class: "row-number",
                onclick: move |_| {
                    let mut selected_row = selected_row;
                    selected_row.set(Some(data_index));
                },
                "{display_index + 1}"
            }
            for col in &columns {
                if editing
                    .read()
                    .as_ref()
                    .map(|cell| cell.row == data_index && cell.column == col.as_str())
                    .unwrap_or(false)
                {
                    td { class: "editing-cell",
                        style: "{column_styles.get(col).cloned().unwrap_or_default()}",
                        input {
                            class: editing_input_class(editing),
                            id: format!("cell-input-{}-{}", data_index, sanitize_id(col)),
                            value: "{editing.read().as_ref().map(|cell| cell.draft.clone()).unwrap_or_default()}",
                            autofocus: true,
                            oninput: move |evt| {
                                let value = evt.value();
                                let mut editing = editing;
                                editing.with_mut(|cell| {
                                    if let Some(cell) = cell {
                                        cell.draft = value;
                                    }
                                });
                            },
                            onblur: move |_| {
                                commit_edit(data, language, file_path, error_message, editing);
                            },
                            onkeydown: move |evt| {
                                match evt.key() {
                                    Key::Enter => commit_edit(data, language, file_path, error_message, editing),
                                    Key::Escape => {
                                        let mut editing = editing;
                                        editing.set(None);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                } else {
                    td {
                        class: cell_class(&row, col, &search_query, computed_columns.contains(col)),
                        id: format!("cell-{}-{}", data_index, sanitize_id(col)),
                        style: "{column_styles.get(col).cloned().unwrap_or_default()}",
                        onclick: {
                            let mut selected_row = selected_row;
                            let mut selected_column = selected_column;
                            let mut editing = editing;
                            let col_name = col.clone();
                            let display_for_edit = row
                                .get(col)
                                .map(data_model::display_value)
                                .unwrap_or_default();
                            let formula_for_edit = computed_formulas
                                .get(col)
                                .map(|formula| format!("={formula}"));
                            move |_| {
                                selected_row.set(Some(data_index));
                                selected_column.set(Some(col_name.clone()));
                                let draft = formula_for_edit
                                    .clone()
                                    .unwrap_or_else(|| display_for_edit.clone());
                                editing.set(Some(EditingCell {
                                    row: data_index,
                                    column: col_name.clone(),
                                    draft,
                                }));
                            }
                        },
                        "{row.get(col).map(data_model::display_value).unwrap_or_default()}"
                    }
                }
            }
        }
    }
}

fn header_class(
    col: &str,
    sort_spec: &Option<crate::state::table_state::SortSpec>,
    selected_column: &Signal<Option<String>>,
) -> String {
    let selected_class = if selected_column
        .read()
        .as_ref()
        .map(|c| c == col)
        .unwrap_or(false)
    {
        "selected-col"
    } else {
        ""
    };

    let sort_class = match sort_spec.as_ref() {
        Some(spec) if spec.column == col => match spec.order {
            SortOrder::Asc => "sorted-asc",
            SortOrder::Desc => "sorted-desc",
        },
        _ => "",
    };

    join_classes(selected_class, sort_class)
}

fn cell_class(row: &Row, column: &str, search_query: &str, is_computed: bool) -> String {
    let mut class_name = if is_computed {
        "cell computed-cell"
    } else {
        "cell"
    }
    .to_string();
    if cell_matches_query(row, column, search_query) {
        class_name = join_classes(&class_name, "search-match");
    }
    class_name
}

fn editing_input_class(editing: Signal<Option<EditingCell>>) -> String {
    let is_formula_mode = editing
        .read()
        .as_ref()
        .map(|cell| cell.draft.trim_start().starts_with('='))
        .unwrap_or(false);

    if is_formula_mode {
        "cell-input formula-input".to_string()
    } else {
        "cell-input".to_string()
    }
}

fn join_classes(a: &str, b: &str) -> String {
    if a.is_empty() {
        return b.to_string();
    }
    if b.is_empty() {
        return a.to_string();
    }
    format!("{a} {b}")
}

fn cell_matches_query(row: &Row, column: &str, query: &str) -> bool {
    if query.is_empty() {
        return false;
    }

    let needle = query.to_ascii_lowercase();
    row.get(column)
        .map(data_model::display_value)
        .map(|value| value.to_ascii_lowercase().contains(&needle))
        .unwrap_or(false)
}

fn commit_edit(
    mut data: Signal<TableState>,
    language: Signal<Language>,
    file_path: Signal<Option<PathBuf>>,
    mut error_message: Signal<Option<String>>,
    mut editing: Signal<Option<EditingCell>>,
) {
    let edit = editing.read().as_ref().cloned();
    if let Some(edit) = edit {
        let draft_trimmed = edit.draft.trim().to_string();
        let (result, sidecar_changed) = data.with_mut(|state| {
            if draft_trimmed.starts_with('=') {
                if state.set_computed_column(&edit.column, draft_trimmed.clone()) {
                    (CommitResult::Applied, true)
                } else {
                    (CommitResult::InvalidFormula, false)
                }
            } else if let Some(previous_computed) = state.computed_column(&edit.column).cloned() {
                state.remove_computed_column(&edit.column);
                if state.set_cell_from_input(edit.row, &edit.column, &edit.draft) {
                    (CommitResult::Applied, true)
                } else {
                    state.set_computed_column(&edit.column, previous_computed.formula);
                    (CommitResult::InvalidTypedValue, false)
                }
            } else if state.set_cell_from_input(edit.row, &edit.column, &edit.draft) {
                (CommitResult::Applied, false)
            } else {
                (CommitResult::InvalidTypedValue, false)
            }
        });

        match result {
            CommitResult::Applied => {
                if sidecar_changed {
                    persist_sidecar_if_possible(data, file_path, error_message);
                } else {
                    error_message.set(None);
                }
            }
            CommitResult::InvalidFormula => {
                error_message.set(Some(
                    i18n::tr(*language.read(), "error.invalid_computed_formula").to_string(),
                ));
            }
            CommitResult::InvalidTypedValue => {
                error_message.set(Some(
                    i18n::tr(*language.read(), "error.invalid_value_for_column_type").to_string(),
                ));
            }
        }
    }
    editing.set(None);
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

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}
