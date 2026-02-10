use std::collections::BTreeSet;
use std::path::PathBuf;

use dioxus::html::input_data::MouseButton;
use dioxus::prelude::{Key, *};

use crate::io::jsheet_io;
use crate::state::data_model::{self, Row};
use crate::state::i18n::{self, Language};
use crate::state::jsheet::{ColumnType, JSheetMeta, SummaryKind};
use crate::state::table_state::{SortOrder, TableState};

#[derive(Clone, PartialEq)]
struct EditingCell {
    row: usize,
    column: String,
    draft: String,
}

#[derive(Clone, PartialEq)]
struct ContextCellMenu {
    row: usize,
    column: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CellPoint {
    row: usize,
    column: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CellRange {
    anchor: CellPoint,
    focus: CellPoint,
}

impl CellRange {
    fn single(point: CellPoint) -> Self {
        Self {
            anchor: point,
            focus: point,
        }
    }

    fn contains(&self, point: CellPoint) -> bool {
        let (row_start, row_end, column_start, column_end) = self.bounds();
        point.row >= row_start
            && point.row <= row_end
            && point.column >= column_start
            && point.column <= column_end
    }

    fn bounds(&self) -> (usize, usize, usize, usize) {
        let row_start = self.anchor.row.min(self.focus.row);
        let row_end = self.anchor.row.max(self.focus.row);
        let column_start = self.anchor.column.min(self.focus.column);
        let column_end = self.anchor.column.max(self.focus.column);
        (row_start, row_end, column_start, column_end)
    }
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
    let mut context_menu = use_signal::<Option<ContextCellMenu>>(|| None);
    let context_formula = use_signal(String::new);
    let context_text_color = use_signal(|| "#1a1a1a".to_string());
    let context_bg_color = use_signal(|| "#ffffff".to_string());
    let selected_range = use_signal::<Option<CellRange>>(|| None);
    let mut drag_selecting = use_signal(|| false);
    let drag_moved = use_signal(|| false);

    let snapshot = data.read().clone();
    let columns = snapshot.display_columns();
    let visible_rows = snapshot.visible_row_indices();
    let search_query = snapshot.search_query().to_string();
    let sort_spec = snapshot.sort_spec().cloned();
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
        div {
            class: "table-container",
            id: "table-container",
            onclick: move |_| {
                context_menu.set(None);
            },
            onmouseup: move |_| {
                drag_selecting.set(false);
            },
            onmouseleave: move |_| {
                drag_selecting.set(false);
            },
            table {
                thead {
                    tr {
                        th { class: "row-number", "#" }
                        for col in &columns {
                            th {
                                class: header_class(col, &selected_column),
                                id: format!("col-{}", sanitize_id(col)),
                                onclick: {
                                    let col_name = col.clone();
                                    let mut selected_column = selected_column;
                                    move |_| {
                                        selected_column.set(Some(col_name.clone()));
                                    }
                                },
                                div { class: "column-header-content",
                                    span { class: "column-header-label", "{col}" }
                                    button {
                                        class: {
                                            let indicator = sort_indicator_for_column(col, &sort_spec);
                                            format!("sort-toggle sort-{}", indicator.class_suffix)
                                        },
                                        id: format!("sort-{}", sanitize_id(col)),
                                        onclick: {
                                            let col_name = col.clone();
                                            let mut data = data;
                                            let mut selected_column = selected_column;
                                            move |evt: Event<MouseData>| {
                                                evt.stop_propagation();
                                                data.with_mut(|state| {
                                                    state.sort_by_column_toggle(&col_name);
                                                });
                                                selected_column.set(Some(col_name.clone()));
                                            }
                                        },
                                        "{sort_indicator_for_column(col, &sort_spec).symbol}"
                                    }
                                }
                            }
                        }
                    }
                    tr { class: "column-meta-row", id: "column-meta-row",
                        th { class: "row-number meta-label", "meta" }
                        for col in &columns {
                            ColumnMetaCell {
                                data,
                                language,
                                file_path,
                                error_message,
                                column: col.clone(),
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
                                data,
                                language,
                                file_path,
                                error_message,
                                selected_row,
                                selected_column,
                                editing,
                                context_menu,
                                context_formula,
                                context_text_color,
                                context_bg_color,
                                selected_range,
                                drag_selecting,
                                drag_moved,
                                search_query: search_query.clone(),
                            }
                        }
                    }
                }
                if has_summary {
                    tfoot {
                        tr { class: "summary-row", id: "summary-row",
                            td { class: "row-number summary-label", "S" }
                            for col in &columns {
                                td {
                                    class: "summary-cell",
                                    id: format!("summary-{}", sanitize_id(col)),
                                    "{snapshot.summary_display_for_column(col).unwrap_or_default()}"
                                }
                            }
                        }
                    }
                }
            }

            if let Some(menu) = context_menu.read().as_ref().cloned() {
                CellContextMenu {
                    data,
                    language,
                    file_path,
                    error_message,
                    context_menu,
                    context_formula,
                    context_text_color,
                    context_bg_color,
                    row_index: menu.row,
                    column: menu.column,
                    selected_range,
                    columns: columns.clone(),
                    visible_rows: visible_rows.clone(),
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
            }
        }
    }
}

#[component]
fn CellContextMenu(
    data: Signal<TableState>,
    language: Signal<Language>,
    file_path: Signal<Option<PathBuf>>,
    error_message: Signal<Option<String>>,
    context_menu: Signal<Option<ContextCellMenu>>,
    context_formula: Signal<String>,
    context_text_color: Signal<String>,
    context_bg_color: Signal<String>,
    row_index: usize,
    column: String,
    selected_range: Signal<Option<CellRange>>,
    columns: Vec<String>,
    visible_rows: Vec<usize>,
) -> Element {
    let current_language = *language.read();
    let formula_label = i18n::tr(current_language, "table.ctx_formula");
    let apply_formula_label = i18n::tr(current_language, "table.ctx_apply_formula");
    let clear_formula_label = i18n::tr(current_language, "table.ctx_clear_formula");
    let text_color_label = i18n::tr(current_language, "table.ctx_text_color");
    let bg_color_label = i18n::tr(current_language, "table.ctx_bg_color");
    let apply_style_label = i18n::tr(current_language, "table.ctx_apply_style");
    let clear_style_label = i18n::tr(current_language, "table.ctx_clear_style");
    let close_label = i18n::tr(current_language, "table.ctx_close");

    rsx! {
        div {
            class: "cell-context-menu",
            id: format!("context-menu-{}-{}", row_index, sanitize_id(&column)),
            onclick: move |evt| evt.stop_propagation(),
            div { class: "ctx-title", "R{row_index + 1} / {column}" }

            label { class: "ctx-label", "{formula_label}" }
            input {
                class: "ctx-input",
                id: "context-formula",
                value: "{context_formula.read()}",
                oninput: move |evt| {
                    context_formula.set(evt.value());
                }
            }

            div { class: "ctx-row",
                button {
                    class: "meta-btn",
                    id: "btn-context-apply-formula",
                    onclick: {
                        let col = column.clone();
                        let columns = columns.clone();
                        let visible_rows = visible_rows.clone();
                        move |_| {
                            let formula = context_formula.read().clone();
                            let Some(normalized_formula) = JSheetMeta::normalize_formula(&formula) else {
                                error_message.set(Some(
                                    i18n::tr(*language.read(), "error.invalid_computed_formula").to_string(),
                                ));
                                return;
                            };

                            let targets = selected_cell_targets(
                                selected_range.read().as_ref().copied(),
                                &columns,
                                &visible_rows,
                                row_index,
                                &col,
                            );
                            let mut changed = false;
                            data.with_mut(|state| {
                                for (target_row, target_col) in &targets {
                                    if state
                                        .cell_formula(*target_row, target_col)
                                        .as_deref()
                                        == Some(normalized_formula.as_str())
                                    {
                                        continue;
                                    }
                                    if state.set_cell_formula(
                                        *target_row,
                                        target_col,
                                        normalized_formula.clone(),
                                    ) {
                                        changed = true;
                                    }
                                }
                            });

                            if changed {
                                persist_sidecar_if_possible(data, file_path, error_message);
                            } else {
                                error_message.set(None);
                            }
                        }
                    },
                    "{apply_formula_label}"
                }
                button {
                    class: "meta-btn",
                    id: "btn-context-clear-formula",
                    onclick: {
                        let col = column.clone();
                        let columns = columns.clone();
                        let visible_rows = visible_rows.clone();
                        move |_| {
                            let targets = selected_cell_targets(
                                selected_range.read().as_ref().copied(),
                                &columns,
                                &visible_rows,
                                row_index,
                                &col,
                            );
                            let mut changed = false;
                            data.with_mut(|state| {
                                for (target_row, target_col) in &targets {
                                    if state.cell_formula(*target_row, target_col).is_some() {
                                        state.remove_cell_formula(*target_row, target_col);
                                        changed = true;
                                    }
                                }
                            });
                            context_formula.set(String::new());
                            if changed {
                                persist_sidecar_if_possible(data, file_path, error_message);
                            } else {
                                error_message.set(None);
                            }
                        }
                    },
                    "{clear_formula_label}"
                }
            }

            label { class: "ctx-label", "{text_color_label}" }
            input {
                class: "meta-color-input",
                id: "context-text-color",
                r#type: "color",
                value: "{context_text_color.read()}",
                oninput: move |evt: Event<FormData>| {
                    context_text_color.set(evt.value());
                }
            }
            label { class: "ctx-label", "{bg_color_label}" }
            input {
                class: "meta-color-input",
                id: "context-bg-color",
                r#type: "color",
                value: "{context_bg_color.read()}",
                oninput: move |evt: Event<FormData>| {
                    context_bg_color.set(evt.value());
                }
            }

            div { class: "ctx-row",
                button {
                    class: "meta-btn",
                    id: "btn-context-apply-style",
                    onclick: {
                        let col = column.clone();
                        let columns = columns.clone();
                        let visible_rows = visible_rows.clone();
                        move |_| {
                            let color = Some(context_text_color.read().clone());
                            let background = Some(context_bg_color.read().clone());
                            let targets = selected_cell_targets(
                                selected_range.read().as_ref().copied(),
                                &columns,
                                &visible_rows,
                                row_index,
                                &col,
                            );
                            data.with_mut(|state| {
                                for (target_row, target_col) in &targets {
                                    state.set_cell_style(
                                        *target_row,
                                        target_col,
                                        color.clone(),
                                        background.clone(),
                                    );
                                }
                            });
                            persist_sidecar_if_possible(data, file_path, error_message);
                        }
                    },
                    "{apply_style_label}"
                }
                button {
                    class: "meta-btn",
                    id: "btn-context-clear-style",
                    onclick: {
                        let col = column.clone();
                        let columns = columns.clone();
                        let visible_rows = visible_rows.clone();
                        move |_| {
                            let targets = selected_cell_targets(
                                selected_range.read().as_ref().copied(),
                                &columns,
                                &visible_rows,
                                row_index,
                                &col,
                            );
                            data.with_mut(|state| {
                                for (target_row, target_col) in &targets {
                                    state.clear_cell_style(*target_row, target_col);
                                }
                            });
                            context_text_color.set("#1a1a1a".to_string());
                            context_bg_color.set("#ffffff".to_string());
                            persist_sidecar_if_possible(data, file_path, error_message);
                        }
                    },
                    "{clear_style_label}"
                }
            }

            button {
                class: "meta-focus-btn",
                id: "btn-context-close",
                onclick: move |_| {
                    context_menu.set(None);
                },
                "{close_label}"
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
    data: Signal<TableState>,
    language: Signal<Language>,
    file_path: Signal<Option<PathBuf>>,
    error_message: Signal<Option<String>>,
    selected_row: Signal<Option<usize>>,
    selected_column: Signal<Option<String>>,
    editing: Signal<Option<EditingCell>>,
    context_menu: Signal<Option<ContextCellMenu>>,
    context_formula: Signal<String>,
    context_text_color: Signal<String>,
    context_bg_color: Signal<String>,
    selected_range: Signal<Option<CellRange>>,
    drag_selecting: Signal<bool>,
    drag_moved: Signal<bool>,
    search_query: String,
) -> Element {
    let snapshot = data.read().clone();
    let formula_columns: BTreeSet<String> = columns
        .iter()
        .filter(|col| snapshot.cell_formula(data_index, col).is_some())
        .cloned()
        .collect();

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
                    selected_row.set(Some(data_index));
                    context_menu.set(None);
                    selected_range.set(None);
                },
                "{display_index + 1}"
            }
            for (column_index, col) in columns.iter().enumerate() {
                if editing
                    .read()
                    .as_ref()
                    .map(|cell| cell.row == data_index && cell.column == col.as_str())
                    .unwrap_or(false)
                {
                    td {
                        class: "editing-cell",
                        style: "{snapshot.cell_inline_style(data_index, col)}",
                        input {
                            class: editing_input_class(editing),
                            id: format!("cell-input-{}-{}", data_index, sanitize_id(col)),
                            value: "{editing.read().as_ref().map(|cell| cell.draft.clone()).unwrap_or_default()}",
                            autofocus: true,
                            oninput: move |evt| {
                                let value = evt.value();
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
                                    Key::Escape => editing.set(None),
                                    _ => {}
                                }
                            }
                        }
                    }
                } else {
                    td {
                        class: cell_class(
                            &row,
                            col,
                            &search_query,
                            formula_columns.contains(col),
                            selected_range
                                .read()
                                .as_ref()
                                .map(|range| {
                                    range.contains(CellPoint {
                                        row: display_index,
                                        column: column_index,
                                    })
                                })
                                .unwrap_or(false),
                        ),
                        id: format!("cell-{}-{}", data_index, sanitize_id(col)),
                        style: "{snapshot.cell_inline_style(data_index, col)}",
                        onmousedown: {
                            let col_name = col.clone();
                            move |evt: Event<MouseData>| {
                                if evt.trigger_button() != Some(MouseButton::Primary) {
                                    return;
                                }

                                selected_column.set(Some(col_name.clone()));
                                context_menu.set(None);
                                editing.set(None);

                                let point = CellPoint {
                                    row: display_index,
                                    column: column_index,
                                };
                                if evt.modifiers().shift() {
                                    let anchor = selected_range
                                        .read()
                                        .as_ref()
                                        .copied()
                                        .map(|range| range.anchor)
                                        .unwrap_or(point);
                                    selected_range.set(Some(CellRange {
                                        anchor,
                                        focus: point,
                                    }));
                                } else {
                                    selected_range.set(Some(CellRange::single(point)));
                                }
                                drag_selecting.set(true);
                                drag_moved.set(false);
                            }
                        },
                        onmouseenter: move |_| {
                            if !*drag_selecting.read() {
                                return;
                            }

                            let point = CellPoint {
                                row: display_index,
                                column: column_index,
                            };
                            let mut changed = false;
                            selected_range.with_mut(|range| {
                                if let Some(range) = range {
                                    if range.focus != point {
                                        range.focus = point;
                                        changed = true;
                                    }
                                } else {
                                    *range = Some(CellRange::single(point));
                                }
                            });
                            if changed {
                                drag_moved.set(true);
                            }
                        },
                        onmouseup: move |_| {
                            drag_selecting.set(false);
                        },
                        onclick: {
                            let col_name = col.clone();
                            let display_for_edit = row
                                .get(col)
                                .map(data_model::display_value)
                                .unwrap_or_default();
                            let formula_for_edit = snapshot
                                .cell_formula(data_index, col)
                                .map(|formula| format!("={formula}"));
                            move |evt: Event<MouseData>| {
                                selected_column.set(Some(col_name.clone()));
                                context_menu.set(None);

                                let point = CellPoint {
                                    row: display_index,
                                    column: column_index,
                                };
                                if evt.modifiers().shift() {
                                    let anchor = selected_range
                                        .read()
                                        .as_ref()
                                        .copied()
                                        .map(|range| range.anchor)
                                        .unwrap_or(point);
                                    selected_range.set(Some(CellRange {
                                        anchor,
                                        focus: point,
                                    }));
                                    editing.set(None);
                                    return;
                                }

                                let dragged = *drag_moved.read();
                                if dragged {
                                    drag_moved.set(false);
                                    editing.set(None);
                                    return;
                                }

                                selected_range.set(Some(CellRange::single(point)));
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
                        oncontextmenu: {
                            let col_name = col.clone();
                            move |evt: Event<MouseData>| {
                                evt.prevent_default();
                                selected_column.set(Some(col_name.clone()));
                                editing.set(None);
                                drag_selecting.set(false);
                                drag_moved.set(false);

                                let point = CellPoint {
                                    row: display_index,
                                    column: column_index,
                                };
                                let in_range = selected_range
                                    .read()
                                    .as_ref()
                                    .map(|range| range.contains(point))
                                    .unwrap_or(false);
                                if !in_range {
                                    selected_range.set(Some(CellRange::single(point)));
                                }

                                let state = data.read();
                                let formula = state
                                    .cell_formula(data_index, &col_name)
                                    .map(|f| format!("={f}"))
                                    .unwrap_or_default();
                                let style = state.cell_style(data_index, &col_name).unwrap_or_default();
                                context_formula.set(formula);
                                context_text_color
                                    .set(style.color.unwrap_or_else(|| "#1a1a1a".to_string()));
                                context_bg_color
                                    .set(style.background.unwrap_or_else(|| "#ffffff".to_string()));
                                context_menu.set(Some(ContextCellMenu {
                                    row: data_index,
                                    column: col_name.clone(),
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

struct SortIndicator {
    symbol: &'static str,
    class_suffix: &'static str,
}

fn sort_indicator_for_column(
    col: &str,
    sort_spec: &Option<crate::state::table_state::SortSpec>,
) -> SortIndicator {
    match sort_spec.as_ref() {
        Some(spec) if spec.column == col => match spec.order {
            SortOrder::Asc => SortIndicator {
                symbol: "▴",
                class_suffix: "asc",
            },
            SortOrder::Desc => SortIndicator {
                symbol: "▾",
                class_suffix: "desc",
            },
        },
        _ => SortIndicator {
            symbol: "↕",
            class_suffix: "none",
        },
    }
}

fn header_class(col: &str, selected_column: &Signal<Option<String>>) -> String {
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

    selected_class.to_string()
}

fn cell_class(
    row: &Row,
    column: &str,
    search_query: &str,
    has_formula: bool,
    in_selected_range: bool,
) -> String {
    let mut class_name = if has_formula {
        "cell formula-cell"
    } else {
        "cell"
    }
    .to_string();

    if cell_matches_query(row, column, search_query) {
        class_name = join_classes(&class_name, "search-match");
    }
    if in_selected_range {
        class_name = join_classes(&class_name, "in-range");
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

fn selected_cell_targets(
    selected_range: Option<CellRange>,
    columns: &[String],
    visible_rows: &[usize],
    fallback_row: usize,
    fallback_column: &str,
) -> Vec<(usize, String)> {
    let Some(selected_range) = selected_range else {
        return vec![(fallback_row, fallback_column.to_string())];
    };

    let (row_start, row_end, column_start, column_end) = selected_range.bounds();
    let mut targets = Vec::new();
    for display_row in row_start..=row_end {
        let Some(data_row) = visible_rows.get(display_row).copied() else {
            continue;
        };
        for column_index in column_start..=column_end {
            let Some(column_name) = columns.get(column_index) else {
                continue;
            };
            targets.push((data_row, column_name.clone()));
        }
    }

    if targets.is_empty() {
        vec![(fallback_row, fallback_column.to_string())]
    } else {
        targets
    }
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
                if state.set_cell_formula(edit.row, &edit.column, draft_trimmed.clone()) {
                    (CommitResult::Applied, true)
                } else {
                    (CommitResult::InvalidFormula, false)
                }
            } else if state.cell_formula(edit.row, &edit.column).is_some() {
                if state.set_cell_from_input(edit.row, &edit.column, &edit.draft) {
                    state.remove_cell_formula(edit.row, &edit.column);
                    (CommitResult::Applied, true)
                } else {
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

    let state = data.read();
    let meta_for_save = state.jsheet_meta_for_save();
    let current_data = state.data();
    if let Err(err) = jsheet_io::save_sidecar_for_json(&path, &meta_for_save, current_data) {
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
