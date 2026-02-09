use dioxus::prelude::*;

use crate::state::data_model::{self, Row, TableData};

#[component]
pub fn Table(data: TableData) -> Element {
    let columns = data_model::derive_columns(&data);

    if columns.is_empty() {
        return rsx! {
            p { class: "empty-message", "No data loaded. Click \"Open\" to load a JSON file." }
        };
    }

    rsx! {
        div { class: "table-container",
            table {
                thead {
                    tr {
                        th { class: "row-number", "#" }
                        for col in &columns {
                            th { "{col}" }
                        }
                    }
                }
                tbody {
                    for (i, row) in data.iter().enumerate() {
                        TableRow { index: i, row: row.clone(), columns: columns.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn TableRow(index: usize, row: Row, columns: Vec<String>) -> Element {
    let row_class = if index % 2 == 0 { "even" } else { "odd" };

    rsx! {
        tr { class: "{row_class}",
            td { class: "row-number", "{index + 1}" }
            for col in &columns {
                td {
                    {
                        row.get(col)
                            .map(data_model::display_value)
                            .unwrap_or_default()
                    }
                }
            }
        }
    }
}
