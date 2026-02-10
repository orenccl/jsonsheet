use dioxus::prelude::{Key, *};

use crate::state::data_model::{self, Row, TableData};

#[derive(Clone, PartialEq)]
struct EditingCell {
    row: usize,
    column: String,
    draft: String,
}

#[component]
pub fn Table(
    data: Signal<TableData>,
    selected_row: Signal<Option<usize>>,
    selected_column: Signal<Option<String>>,
) -> Element {
    let columns = data_model::derive_columns(&data.read());
    let editing = use_signal::<Option<EditingCell>>(|| None);

    if columns.is_empty() {
        return rsx! {
            p { class: "empty-message", id: "empty-message", "No data loaded. Click \"Open\" to load a JSON file." }
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
                                class: if selected_column
                                    .read()
                                    .as_ref()
                                    .map(|c| c == col)
                                    .unwrap_or(false) {
                                        "selected-col"
                                    } else { "" },
                                id: format!("col-{}", sanitize_id(col)),
                                onclick: {
                                    let mut selected_column = selected_column;
                                    let col_name = col.clone();
                                    move |_| {
                                        selected_column.set(Some(col_name.clone()));
                                    }
                                },
                                "{col}"
                            }
                        }
                    }
                }
                tbody {
                    for (i, row) in data.read().iter().enumerate() {
                        TableRow {
                            index: i,
                            row: row.clone(),
                            columns: columns.clone(),
                            data,
                            selected_row,
                            selected_column,
                            editing,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TableRow(
    index: usize,
    row: Row,
    columns: Vec<String>,
    data: Signal<TableData>,
    selected_row: Signal<Option<usize>>,
    selected_column: Signal<Option<String>>,
    editing: Signal<Option<EditingCell>>,
) -> Element {
    let is_selected = selected_row
        .read()
        .as_ref()
        .map(|row_index| *row_index == index)
        .unwrap_or(false);
    let mut row_class = if index % 2 == 0 { "even" } else { "odd" }.to_string();
    if is_selected {
        row_class.push_str(" selected-row");
    }

    rsx! {
        tr { class: "{row_class}", id: format!("row-{index}"),
            td {
                class: "row-number",
                onclick: move |_| {
                    let mut selected_row = selected_row;
                    selected_row.set(Some(index));
                },
                "{index + 1}"
            }
            for col in &columns {
                if editing
                    .read()
                    .as_ref()
                    .map(|cell| cell.row == index && cell.column == col.as_str())
                    .unwrap_or(false)
                {
                    td { class: "editing-cell",
                        input {
                            class: "cell-input",
                            id: format!("cell-input-{}-{}", index, sanitize_id(col)),
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
                        class: "cell",
                        id: format!("cell-{}-{}", index, sanitize_id(col)),
                        onclick: {
                            let mut selected_row = selected_row;
                            let mut selected_column = selected_column;
                            let mut editing = editing;
                            let col_name = col.clone();
                            let display_for_edit = row
                                .get(col)
                                .map(data_model::display_value)
                                .unwrap_or_default();
                            move |_| {
                                selected_row.set(Some(index));
                                selected_column.set(Some(col_name.clone()));
                                editing.set(Some(EditingCell {
                                    row: index,
                                    column: col_name.clone(),
                                    draft: display_for_edit.clone(),
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

fn commit_edit(mut data: Signal<TableData>, mut editing: Signal<Option<EditingCell>>) {
    let edit = editing.read().as_ref().cloned();
    if let Some(edit) = edit {
        let value = data_model::parse_cell_input(&edit.draft);
        data.with_mut(|rows| {
            data_model::set_cell_value(rows, edit.row, &edit.column, value);
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
