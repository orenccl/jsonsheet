use std::collections::{BTreeMap, BTreeSet};

use dioxus::prelude::{Key, *};

use crate::state::data_model::{self, Row};
use crate::state::i18n::{self, Language};
use crate::state::table_state::{SortOrder, TableState};

#[derive(Clone, PartialEq)]
struct EditingCell {
    row: usize,
    column: String,
    draft: String,
}

#[component]
pub fn Table(
    data: Signal<TableState>,
    language: Signal<Language>,
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
    let column_styles: BTreeMap<String, String> = columns
        .iter()
        .map(|col| (col.clone(), snapshot.column_inline_style(col)))
        .collect();
    let has_summary = columns
        .iter()
        .any(|column| snapshot.summary_kind(column).is_some());

    if columns.is_empty() {
        let empty_message = i18n::tr(*language.read(), "table.empty_message");
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
                                column_styles: column_styles.clone(),
                                data,
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
fn TableRow(
    display_index: usize,
    data_index: usize,
    row: Row,
    columns: Vec<String>,
    computed_columns: BTreeSet<String>,
    column_styles: BTreeMap<String, String>,
    data: Signal<TableState>,
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
                            class: "cell-input",
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
                                commit_edit(data, editing);
                            },
                            onkeydown: move |evt| {
                                match evt.key() {
                                    Key::Enter => commit_edit(data, editing),
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
                            let is_computed = computed_columns.contains(col);
                            move |_| {
                                selected_row.set(Some(data_index));
                                selected_column.set(Some(col_name.clone()));
                                if !is_computed {
                                    editing.set(Some(EditingCell {
                                        row: data_index,
                                        column: col_name.clone(),
                                        draft: display_for_edit.clone(),
                                    }));
                                }
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

fn commit_edit(mut data: Signal<TableState>, mut editing: Signal<Option<EditingCell>>) {
    let edit = editing.read().as_ref().cloned();
    if let Some(edit) = edit {
        data.with_mut(|state| {
            state.set_cell_from_input(edit.row, &edit.column, &edit.draft);
        });
    }
    editing.set(None);
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}
