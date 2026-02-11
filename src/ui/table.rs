use std::collections::BTreeSet;
use std::path::PathBuf;

use dioxus::html::input_data::MouseButton;
use dioxus::prelude::{Key, *};
use serde_json::Value;

use crate::state::data_model::{self, Row};
use crate::state::i18n::{self, Language};
use crate::state::jsheet::{
    ColumnStyle, ColumnType, ConditionalFormat, JSheetMeta, ParsedCondRule, SummaryKind,
};
use crate::state::table_state::{CellEdit, CellEditKind, SortOrder, TableState};
use crate::ui::actions;

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
    x: f64,
    y: f64,
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

struct AutoFillPlan {
    edits: Vec<CellEdit>,
    touches_formula: bool,
}

#[component]
pub fn Table(
    data: Signal<TableState>,
    language: Signal<Language>,
    file_path: Signal<Option<PathBuf>>,
    error_message: Signal<Option<String>>,
    selected_row: Signal<Option<usize>>,
    selected_column: Signal<Option<String>>,
    show_meta_row: Signal<bool>,
) -> Element {
    let mut editing = use_signal::<Option<EditingCell>>(|| None);
    let mut context_menu = use_signal::<Option<ContextCellMenu>>(|| None);
    let context_formula = use_signal(String::new);
    let context_text_color = use_signal(|| "#1a1a1a".to_string());
    let context_bg_color = use_signal(|| "#ffffff".to_string());
    let context_cond_rule = use_signal(String::new);
    let context_cond_color = use_signal(|| "#ff0000".to_string());
    let selected_range = use_signal::<Option<CellRange>>(|| None);
    let mut drag_selecting = use_signal(|| false);
    let drag_moved = use_signal(|| false);
    let autofill_dragging = use_signal(|| false);
    let autofill_source = use_signal::<Option<CellRange>>(|| None);
    let autofill_target = use_signal::<Option<CellPoint>>(|| None);

    let snapshot = data.read().clone();
    let columns = snapshot.display_columns();
    let visible_rows = snapshot.visible_row_indices();
    let search_query = snapshot.search_query().to_string();
    let sort_spec = snapshot.sort_spec().cloned();
    let has_summary = columns
        .iter()
        .any(|column| snapshot.summary_kind(column).is_some());
    let frozen_count = snapshot.frozen_columns();
    let current_language = *language.read();

    if columns.is_empty() {
        let empty_hint = i18n::tr(current_language, "table.empty_hint");
        let open_label = i18n::tr(current_language, "toolbar.open");
        return rsx! {
            div { class: "empty-state", id: "empty-state",
                div { class: "empty-state-icon", "\u{1F4C4}" }
                h2 { class: "empty-state-title", "JsonSheet" }
                p { class: "empty-state-hint", "{empty_hint}" }
                button {
                    class: "empty-state-btn",
                    id: "btn-empty-open",
                    onclick: move |_| {
                        spawn(async move {
                            actions::open_file(data, language, file_path, error_message, selected_row, selected_column).await;
                        });
                    },
                    "\u{1F4C2} {open_label}"
                }
            }
        };
    }

    rsx! {
        div {
            class: "table-container",
            id: "table-container",
            tabindex: "0",
            onclick: move |_| {
                context_menu.set(None);
            },
            onkeydown: {
                let columns = columns.clone();
                let visible_rows = visible_rows.clone();
                move |evt: Event<KeyboardData>| {
                    let col_count = columns.len();
                    let row_count = visible_rows.len();
                    if col_count == 0 || row_count == 0 {
                        return;
                    }

                    // If editing, handle edit-specific keys
                    if editing.read().is_some() {
                        if evt.key() == Key::Escape {
                            editing.set(None);
                        }
                        return;
                    }

                    match evt.key() {
                        Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight => {
                            evt.prevent_default();
                            let (dr, dc): (isize, isize) = match evt.key() {
                                Key::ArrowUp => (-1, 0),
                                Key::ArrowDown => (1, 0),
                                Key::ArrowLeft => (0, -1),
                                Key::ArrowRight => (0, 1),
                                _ => (0, 0),
                            };
                            move_selection(
                                selected_range, selected_row, selected_column,
                                dr, dc, row_count, col_count, &columns, &visible_rows,
                            );
                        }
                        Key::Enter | Key::F2 => {
                            evt.prevent_default();
                            enter_edit_from_selection(
                                selected_range, editing, data,
                                &columns, &visible_rows,
                            );
                        }
                        Key::Delete => {
                            evt.prevent_default();
                            delete_selected_cells(
                                data, language, file_path, error_message,
                                selected_range, &columns, &visible_rows,
                            );
                        }
                        Key::Tab => {
                            evt.prevent_default();
                            let dc: isize = if evt.modifiers().shift() { -1 } else { 1 };
                            move_selection(
                                selected_range, selected_row, selected_column,
                                0, dc, row_count, col_count, &columns, &visible_rows,
                            );
                        }
                        _ => {
                            // Start typing to edit
                            if let Key::Character(ref c) = evt.key() {
                                if !evt.modifiers().ctrl() && !evt.modifiers().meta() && !evt.modifiers().alt()
                                    && c.len() == 1
                                {
                                    // Start editing with the typed character
                                    if let Some(range) = selected_range.read().as_ref().copied() {
                                        let point = range.anchor;
                                        if let (Some(&data_row), Some(col_name)) = (visible_rows.get(point.row), columns.get(point.column)) {
                                            editing.set(Some(EditingCell {
                                                row: data_row,
                                                column: col_name.clone(),
                                                draft: c.clone(),
                                            }));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            onmouseup: {
                let columns = columns.clone();
                let visible_rows = visible_rows.clone();
                move |_| {
                    drag_selecting.set(false);
                    finish_autofill_drag(
                        data,
                        file_path,
                        error_message,
                        selected_range,
                        autofill_dragging,
                        autofill_source,
                        autofill_target,
                        &columns,
                        &visible_rows,
                    );
                }
            },
            onmouseleave: {
                let columns = columns.clone();
                let visible_rows = visible_rows.clone();
                move |_| {
                    drag_selecting.set(false);
                    finish_autofill_drag(
                        data,
                        file_path,
                        error_message,
                        selected_range,
                        autofill_dragging,
                        autofill_source,
                        autofill_target,
                        &columns,
                        &visible_rows,
                    );
                }
            },
            table {
                thead {
                    tr {
                        th {
                            class: if frozen_count > 0 { "row-number frozen-col" } else { "row-number" },
                            style: if frozen_count > 0 { "left: 0px;" } else { "" },
                            "#"
                        }
                        for (col_idx, col) in columns.iter().enumerate() {
                            th {
                                class: frozen_header_class(col, &selected_column, col_idx, frozen_count),
                                style: frozen_left_style(col_idx, frozen_count),
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
                    if *show_meta_row.read() {
                        tr { class: "column-meta-row", id: "column-meta-row",
                            th {
                                class: if frozen_count > 0 { "row-number meta-label frozen-col" } else { "row-number meta-label" },
                                style: if frozen_count > 0 { "left: 0px;" } else { "" },
                                "meta"
                            }
                            for (col_idx, col) in columns.iter().enumerate() {
                                ColumnMetaCell {
                                    data,
                                    language,
                                    file_path,
                                    error_message,
                                    column: col.clone(),
                                    frozen: col_idx < frozen_count,
                                    frozen_left: frozen_left_px(col_idx, frozen_count),
                                }
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
                                autofill_dragging,
                                autofill_source,
                                autofill_target,
                                search_query: search_query.clone(),
                                frozen_count,
                                visible_row_count: visible_rows.len(),
                            }
                        }
                    }
                }
                if has_summary {
                    tfoot {
                        tr { class: "summary-row", id: "summary-row",
                            td {
                                class: if frozen_count > 0 { "row-number summary-label frozen-col" } else { "row-number summary-label" },
                                style: if frozen_count > 0 { "left: 0px;" } else { "" },
                                "S"
                            }
                            for (col_idx, col) in columns.iter().enumerate() {
                                td {
                                    class: if col_idx < frozen_count { "summary-cell frozen-col" } else { "summary-cell" },
                                    style: frozen_left_style(col_idx, frozen_count),
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
                    context_cond_rule,
                    context_cond_color,
                    row_index: menu.row,
                    column: menu.column,
                    menu_x: menu.x,
                    menu_y: menu.y,
                    selected_range,
                    columns: columns.clone(),
                    visible_rows: visible_rows.clone(),
                }
            }
        }
        {
            let total_rows = snapshot.data().len();
            let visible_count = visible_rows.len();
            let has_filter = snapshot.filter_column().is_some();
            let row_count_label = i18n::tr(current_language, "status.row_count");
            let visible_label = i18n::tr(current_language, "status.visible");
            let filter_active_label = i18n::tr(current_language, "status.filter_active");
            let selection_label = i18n::tr(current_language, "status.selection");

            let selection_text = if let Some(range) = selected_range.read().as_ref() {
                let (r1, r2, c1, c2) = range.bounds();
                if r1 == r2 && c1 == c2 {
                    if let Some(col_name) = columns.get(c1) {
                        format!("{selection_label}: R{}:{col_name}", r1 + 1)
                    } else {
                        String::new()
                    }
                } else {
                    format!("{selection_label}: R{}-R{}, C{}-C{}", r1 + 1, r2 + 1, c1 + 1, c2 + 1)
                }
            } else {
                String::new()
            };

            rsx! {
                div { class: "status-bar", id: "status-bar",
                    span { class: "status-item", "{row_count_label}: {total_rows}" }
                    if total_rows != visible_count {
                        span { class: "status-item", "{visible_label}: {visible_count}" }
                    }
                    if has_filter {
                        span { class: "status-item status-filter-active", "{filter_active_label}" }
                    }
                    if !selection_text.is_empty() {
                        span { class: "status-item", "{selection_text}" }
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
    frozen: bool,
    frozen_left: String,
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
    let validation = snapshot.validation_rule(&column).cloned();
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

    let meta_class = if frozen {
        "column-meta-cell frozen-col"
    } else {
        "column-meta-cell"
    };
    let meta_style = if frozen {
        format!("left: {frozen_left};")
    } else {
        String::new()
    };

    rsx! {
        th {
            class: "{meta_class}",
            style: "{meta_style}",
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
                            actions::persist_sidecar_if_possible(data, file_path, error_message);
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
                            actions::persist_sidecar_if_possible(data, file_path, error_message);
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
                                actions::persist_sidecar_if_possible(data, file_path, error_message);
                            }
                        }
                    }
                    "{meta_comment_label}"
                }
                {
                    let min_label = i18n::tr(current_language, "table.meta_validation_min");
                    let max_label = i18n::tr(current_language, "table.meta_validation_max");
                    let enum_label = i18n::tr(current_language, "table.meta_validation_enum");
                    let min_val = validation
                        .as_ref()
                        .and_then(|r| r.min)
                        .map(format_validation_number)
                        .unwrap_or_default();
                    let max_val = validation
                        .as_ref()
                        .and_then(|r| r.max)
                        .map(format_validation_number)
                        .unwrap_or_default();
                    let enum_val = validation.as_ref().and_then(|r| r.enum_values.as_ref()).map(|v| v.join(", ")).unwrap_or_default();
                    rsx! {
                        div { class: "meta-validation-row",
                            input {
                                class: "meta-input-sm",
                                id: format!("meta-val-min-{}", sanitize_id(&column)),
                                r#type: "number",
                                placeholder: "{min_label}",
                                value: "{min_val}",
                                onchange: {
                                    let col = column.clone();
                                    move |evt: Event<FormData>| {
                                        let val = evt.value();
                                        data.with_mut(|state| {
                                            let mut rule = state.validation_rule(&col).cloned().unwrap_or_default();
                                            rule.min = if val.trim().is_empty() { None } else { val.trim().parse::<f64>().ok() };
                                            let is_empty = rule.min.is_none() && rule.max.is_none() && rule.enum_values.is_none();
                                            state.set_validation_rule(&col, if is_empty { None } else { Some(rule) });
                                        });
                                        actions::persist_sidecar_if_possible(data, file_path, error_message);
                                    }
                                }
                            }
                            input {
                                class: "meta-input-sm",
                                id: format!("meta-val-max-{}", sanitize_id(&column)),
                                r#type: "number",
                                placeholder: "{max_label}",
                                value: "{max_val}",
                                onchange: {
                                    let col = column.clone();
                                    move |evt: Event<FormData>| {
                                        let val = evt.value();
                                        data.with_mut(|state| {
                                            let mut rule = state.validation_rule(&col).cloned().unwrap_or_default();
                                            rule.max = if val.trim().is_empty() { None } else { val.trim().parse::<f64>().ok() };
                                            let is_empty = rule.min.is_none() && rule.max.is_none() && rule.enum_values.is_none();
                                            state.set_validation_rule(&col, if is_empty { None } else { Some(rule) });
                                        });
                                        actions::persist_sidecar_if_possible(data, file_path, error_message);
                                    }
                                }
                            }
                            input {
                                class: "meta-input-sm meta-input-enum",
                                id: format!("meta-val-enum-{}", sanitize_id(&column)),
                                placeholder: "{enum_label}",
                                value: "{enum_val}",
                                onchange: {
                                    let col = column.clone();
                                    move |evt: Event<FormData>| {
                                        let val = evt.value();
                                        data.with_mut(|state| {
                                            let mut rule = state.validation_rule(&col).cloned().unwrap_or_default();
                                            let trimmed = val.trim();
                                            rule.enum_values = if trimmed.is_empty() {
                                                None
                                            } else {
                                                Some(trimmed.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                                            };
                                            let is_empty = rule.min.is_none() && rule.max.is_none() && rule.enum_values.is_none();
                                            state.set_validation_rule(&col, if is_empty { None } else { Some(rule) });
                                        });
                                        actions::persist_sidecar_if_possible(data, file_path, error_message);
                                    }
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
fn CellContextMenu(
    data: Signal<TableState>,
    language: Signal<Language>,
    file_path: Signal<Option<PathBuf>>,
    error_message: Signal<Option<String>>,
    context_menu: Signal<Option<ContextCellMenu>>,
    context_formula: Signal<String>,
    context_text_color: Signal<String>,
    context_bg_color: Signal<String>,
    context_cond_rule: Signal<String>,
    context_cond_color: Signal<String>,
    row_index: usize,
    column: String,
    menu_x: f64,
    menu_y: f64,
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
            style: format!("left: {menu_x}px; top: {menu_y}px;"),
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
                                actions::persist_sidecar_if_possible(data, file_path, error_message);
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
                                actions::persist_sidecar_if_possible(data, file_path, error_message);
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
                            actions::persist_sidecar_if_possible(data, file_path, error_message);
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
                            actions::persist_sidecar_if_possible(data, file_path, error_message);
                        }
                    },
                    "{clear_style_label}"
                }
            }

            // Conditional formatting section
            {
                let cond_format_label = i18n::tr(current_language, "table.ctx_cond_format");
                let cond_rule_placeholder = i18n::tr(current_language, "table.ctx_cond_rule_placeholder");
                let add_cond_label = i18n::tr(current_language, "table.ctx_add_cond_format");
                let remove_cond_label = i18n::tr(current_language, "table.ctx_remove_cond_format");

                let snapshot = data.read();
                let existing_rules: Vec<(usize, ConditionalFormat)> = snapshot
                    .conditional_formats()
                    .iter()
                    .enumerate()
                    .filter(|(_, cf)| cf.column == column)
                    .map(|(i, cf)| (i, cf.clone()))
                    .collect();
                drop(snapshot);

                rsx! {
                    label { class: "ctx-label", "{cond_format_label} ({column})" }
                    for (global_idx, cf) in existing_rules.iter() {
                        div { class: "ctx-row ctx-cond-rule",
                            span { class: "ctx-cond-text",
                                "{cf.rule}"
                                if cf.style.color.is_some() || cf.style.background.is_some() {
                                    " [style]"
                                    if let Some(ref c) = cf.style.color {
                                        span { style: "color:{c}", " A" }
                                    }
                                    if let Some(ref bg) = cf.style.background {
                                        span { style: "background:{bg}", "  " }
                                    }
                                }
                            }
                            button {
                                class: "meta-btn meta-btn-sm",
                                onclick: {
                                    let idx = *global_idx;
                                    move |_| {
                                        let removed = data.with_mut(|state| state.remove_conditional_format(idx));
                                        if removed {
                                            actions::persist_sidecar_if_possible(data, file_path, error_message);
                                        }
                                    }
                                },
                                "{remove_cond_label}"
                            }
                        }
                    }
                    div { class: "ctx-row",
                        input {
                            class: "ctx-input ctx-input-sm",
                            id: "context-cond-rule",
                            placeholder: "{cond_rule_placeholder}",
                            value: "{context_cond_rule.read()}",
                            oninput: move |evt| {
                                context_cond_rule.set(evt.value());
                            }
                        }
                        input {
                            class: "meta-color-input",
                            id: "context-cond-color",
                            r#type: "color",
                            value: "{context_cond_color.read()}",
                            oninput: move |evt: Event<FormData>| {
                                context_cond_color.set(evt.value());
                            }
                        }
                        button {
                            class: "meta-btn",
                            id: "btn-context-add-cond-format",
                            onclick: {
                                let col = column.clone();
                                move |_| {
                                    let rule_text = context_cond_rule.read().clone();
                                    if ParsedCondRule::parse(&rule_text).is_none() {
                                        error_message.set(Some("Invalid rule syntax. Use operators like < 100, >= 50, == legendary, != 0".to_string()));
                                        return;
                                    }
                                    let color_val = context_cond_color.read().clone();
                                    data.with_mut(|state| {
                                        state.add_conditional_format(ConditionalFormat {
                                            column: col.clone(),
                                            rule: rule_text,
                                            style: ColumnStyle {
                                                color: Some(color_val),
                                                background: None,
                                            },
                                        });
                                    });
                                    context_cond_rule.set(String::new());
                                    actions::persist_sidecar_if_possible(data, file_path, error_message);
                                }
                            },
                            "{add_cond_label}"
                        }
                    }
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
    autofill_dragging: Signal<bool>,
    autofill_source: Signal<Option<CellRange>>,
    autofill_target: Signal<Option<CellPoint>>,
    search_query: String,
    frozen_count: usize,
    visible_row_count: usize,
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
                class: if frozen_count > 0 { "row-number frozen-col" } else { "row-number" },
                style: if frozen_count > 0 { "left: 0px;" } else { "" },
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
                        class: if column_index < frozen_count { "editing-cell frozen-col" } else { "editing-cell" },
                        style: "{frozen_left_style(column_index, frozen_count)}{snapshot.cell_inline_style(data_index, col)}",
                        input {
                            class: editing_input_class(editing),
                            id: format!("cell-input-{}-{}", data_index, sanitize_id(col)),
                            list: "{editing_enum_list_id(&snapshot, col)}",
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
                            onkeydown: {
                                let columns = columns.clone();
                                let visible_rows_count = visible_row_count;
                                move |evt: Event<KeyboardData>| {
                                    match evt.key() {
                                        Key::Enter => {
                                            commit_edit(data, language, file_path, error_message, editing);
                                            // Move down
                                            move_selection(
                                                selected_range, selected_row, selected_column,
                                                1, 0, visible_rows_count, columns.len(), &columns, &[],
                                            );
                                        }
                                        Key::Tab => {
                                            evt.prevent_default();
                                            commit_edit(data, language, file_path, error_message, editing);
                                            let dc: isize = if evt.modifiers().shift() { -1 } else { 1 };
                                            move_selection(
                                                selected_range, selected_row, selected_column,
                                                0, dc, visible_rows_count, columns.len(), &columns, &[],
                                            );
                                        }
                                        Key::Escape => editing.set(None),
                                        _ => {}
                                    }
                                }
                            }
                        }
                        if has_enum_options(&snapshot, col) {
                            datalist {
                                id: enum_list_id(col),
                                for option in enum_values_for_column(&snapshot, col) {
                                    option { value: "{option}" }
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
                            range_contains_cell(
                                selected_range.read().as_ref().copied(),
                                display_index,
                                column_index,
                            ),
                            autofill_preview_contains_cell(
                                autofill_source.read().as_ref().copied(),
                                autofill_target.read().as_ref().copied(),
                                display_index,
                                column_index,
                            ),
                            column_index < frozen_count,
                            is_single_selected_cell(
                                selected_range.read().as_ref().copied(),
                                display_index,
                                column_index,
                            ),
                        ),
                        id: format!("cell-{}-{}", data_index, sanitize_id(col)),
                        style: "{frozen_left_style(column_index, frozen_count)}{snapshot.cell_inline_style(data_index, col)}",
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
                            if *autofill_dragging.read() {
                                autofill_target.set(Some(CellPoint {
                                    row: display_index,
                                    column: column_index,
                                }));
                                drag_moved.set(true);
                                return;
                            }

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
                            move |evt: Event<MouseData>| {
                                if *autofill_dragging.read() {
                                    return;
                                }
                                selected_row.set(Some(data_index));
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
                                editing.set(None);
                            }
                        },
                        ondoubleclick: {
                            let col_name = col.clone();
                            let display_for_edit = row
                                .get(col)
                                .map(data_model::display_value)
                                .unwrap_or_default();
                            let formula_for_edit = snapshot
                                .cell_formula(data_index, col)
                                .map(|formula| format!("={formula}"));
                            move |_| {
                                selected_row.set(Some(data_index));
                                selected_column.set(Some(col_name.clone()));
                                context_menu.set(None);

                                let point = CellPoint {
                                    row: display_index,
                                    column: column_index,
                                };
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
                                let coords = evt.client_coordinates();
                                context_menu.set(Some(ContextCellMenu {
                                    row: data_index,
                                    column: col_name.clone(),
                                    x: coords.x,
                                    y: coords.y,
                                }));
                            }
                        },
                        "{row.get(col).map(data_model::display_value).unwrap_or_default()}"
                        if is_autofill_handle_cell(
                            selected_range.read().as_ref().copied(),
                            display_index,
                            column_index,
                        ) {
                            div {
                                class: "fill-handle",
                                id: format!("fill-handle-{}-{}", display_index, column_index),
                                onmousedown: move |evt: Event<MouseData>| {
                                    if evt.trigger_button() != Some(MouseButton::Primary) {
                                        return;
                                    }
                                    evt.prevent_default();
                                    evt.stop_propagation();
                                    let current = selected_range.read().as_ref().copied();
                                    if current.is_none() {
                                        return;
                                    }
                                    autofill_source.set(current);
                                    autofill_target.set(Some(CellPoint {
                                        row: display_index,
                                        column: column_index,
                                    }));
                                    autofill_dragging.set(true);
                                    drag_selecting.set(false);
                                    drag_moved.set(true);
                                    editing.set(None);
                                    context_menu.set(None);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn move_selection(
    mut selected_range: Signal<Option<CellRange>>,
    mut selected_row: Signal<Option<usize>>,
    mut selected_column: Signal<Option<String>>,
    dr: isize,
    dc: isize,
    row_count: usize,
    col_count: usize,
    columns: &[String],
    visible_rows: &[usize],
) {
    let current = selected_range
        .read()
        .as_ref()
        .copied()
        .map(|r| r.anchor)
        .unwrap_or(CellPoint { row: 0, column: 0 });

    let new_row = (current.row as isize + dr).clamp(0, row_count as isize - 1) as usize;
    let new_col = (current.column as isize + dc).clamp(0, col_count as isize - 1) as usize;

    let point = CellPoint {
        row: new_row,
        column: new_col,
    };
    selected_range.set(Some(CellRange::single(point)));

    if let Some(col_name) = columns.get(new_col) {
        selected_column.set(Some(col_name.clone()));
    }
    if let Some(&data_row) = visible_rows.get(new_row) {
        selected_row.set(Some(data_row));
    }
}

fn enter_edit_from_selection(
    selected_range: Signal<Option<CellRange>>,
    mut editing: Signal<Option<EditingCell>>,
    data: Signal<TableState>,
    columns: &[String],
    visible_rows: &[usize],
) {
    let Some(range) = selected_range.read().as_ref().copied() else {
        return;
    };
    let point = range.anchor;
    let Some(&data_row) = visible_rows.get(point.row) else {
        return;
    };
    let Some(col_name) = columns.get(point.column) else {
        return;
    };

    let snapshot = data.read();
    let draft = snapshot
        .cell_formula(data_row, col_name)
        .map(|f| format!("={f}"))
        .unwrap_or_else(|| {
            snapshot
                .cell_value(data_row, col_name)
                .map(|v| data_model::display_value(&v))
                .unwrap_or_default()
        });

    editing.set(Some(EditingCell {
        row: data_row,
        column: col_name.clone(),
        draft,
    }));
}

#[allow(clippy::too_many_arguments)]
fn delete_selected_cells(
    mut data: Signal<TableState>,
    _language: Signal<Language>,
    _file_path: Signal<Option<PathBuf>>,
    mut error_message: Signal<Option<String>>,
    selected_range: Signal<Option<CellRange>>,
    columns: &[String],
    visible_rows: &[usize],
) {
    let Some(range) = selected_range.read().as_ref().copied() else {
        return;
    };
    let (row_start, row_end, col_start, col_end) = range.bounds();
    let mut edits = Vec::new();
    for display_row in row_start..=row_end {
        let Some(&data_row) = visible_rows.get(display_row) else {
            continue;
        };
        for col_idx in col_start..=col_end {
            let Some(col_name) = columns.get(col_idx) else {
                continue;
            };
            edits.push(CellEdit {
                row_index: data_row,
                column: col_name.clone(),
                kind: CellEditKind::Value(Value::Null),
            });
        }
    }
    if !edits.is_empty() {
        data.with_mut(|state| state.apply_cell_edits(edits));
        error_message.set(None);
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
                symbol: "\u{25B2}",
                class_suffix: "asc",
            },
            SortOrder::Desc => SortIndicator {
                symbol: "\u{25BC}",
                class_suffix: "desc",
            },
        },
        _ => SortIndicator {
            symbol: "\u{25BD}",
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

#[allow(clippy::too_many_arguments)]
fn cell_class(
    row: &Row,
    column: &str,
    search_query: &str,
    has_formula: bool,
    in_selected_range: bool,
    in_autofill_preview: bool,
    frozen: bool,
    is_selected_cell: bool,
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
    if in_autofill_preview {
        class_name = join_classes(&class_name, "autofill-preview");
    }
    if frozen {
        class_name = join_classes(&class_name, "frozen-col");
    }
    if is_selected_cell {
        class_name = join_classes(&class_name, "selected-cell");
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

fn range_contains_cell(selected_range: Option<CellRange>, row: usize, column: usize) -> bool {
    selected_range
        .map(|range| range.contains(CellPoint { row, column }))
        .unwrap_or(false)
}

fn is_single_selected_cell(selected_range: Option<CellRange>, row: usize, column: usize) -> bool {
    selected_range
        .map(|range| {
            range.anchor == range.focus && range.anchor.row == row && range.anchor.column == column
        })
        .unwrap_or(false)
}

fn is_autofill_handle_cell(selected_range: Option<CellRange>, row: usize, column: usize) -> bool {
    let Some(range) = selected_range else {
        return false;
    };
    let (_, row_end, _, column_end) = range.bounds();
    row == row_end && column == column_end
}

fn autofill_preview_contains_cell(
    source: Option<CellRange>,
    target: Option<CellPoint>,
    row: usize,
    column: usize,
) -> bool {
    let point = CellPoint { row, column };
    let Some(source_range) = source else {
        return false;
    };
    let Some(expanded) = autofill_expanded_range(source, target) else {
        return false;
    };
    expanded.contains(point) && !source_range.contains(point)
}

fn autofill_expanded_range(
    source: Option<CellRange>,
    target: Option<CellPoint>,
) -> Option<CellRange> {
    let source_range = source?;
    let target_point = target?;
    let (src_row_start, src_row_end, src_col_start, src_col_end) = source_range.bounds();
    Some(CellRange {
        anchor: CellPoint {
            row: src_row_start.min(target_point.row),
            column: src_col_start.min(target_point.column),
        },
        focus: CellPoint {
            row: src_row_end.max(target_point.row),
            column: src_col_end.max(target_point.column),
        },
    })
}

#[allow(clippy::too_many_arguments)]
fn finish_autofill_drag(
    mut data: Signal<TableState>,
    file_path: Signal<Option<PathBuf>>,
    mut error_message: Signal<Option<String>>,
    mut selected_range: Signal<Option<CellRange>>,
    mut autofill_dragging: Signal<bool>,
    mut autofill_source: Signal<Option<CellRange>>,
    mut autofill_target: Signal<Option<CellPoint>>,
    columns: &[String],
    visible_rows: &[usize],
) {
    if !*autofill_dragging.read() {
        return;
    }

    let source = autofill_source.read().as_ref().copied();
    let target = autofill_target.read().as_ref().copied();
    if let Some(expanded) = autofill_expanded_range(source, target) {
        selected_range.set(Some(expanded));
    }

    let snapshot = data.read().clone();
    let plan = build_autofill_plan(&snapshot, columns, visible_rows, source, target);
    drop(snapshot);

    let changed = if plan.edits.is_empty() {
        0
    } else {
        data.with_mut(|state| state.apply_cell_edits(plan.edits))
    };

    if changed > 0 {
        if plan.touches_formula {
            actions::persist_sidecar_if_possible(data, file_path, error_message);
        } else {
            error_message.set(None);
        }
    } else {
        error_message.set(None);
    }

    autofill_dragging.set(false);
    autofill_source.set(None);
    autofill_target.set(None);
}

fn build_autofill_plan(
    snapshot: &TableState,
    columns: &[String],
    visible_rows: &[usize],
    source: Option<CellRange>,
    target: Option<CellPoint>,
) -> AutoFillPlan {
    let mut plan = AutoFillPlan {
        edits: Vec::new(),
        touches_formula: false,
    };

    let Some(source_range) = source else {
        return plan;
    };
    let Some(expanded_range) = autofill_expanded_range(source, target) else {
        return plan;
    };

    let (src_row_start, src_row_end, src_col_start, src_col_end) = source_range.bounds();
    let (full_row_start, full_row_end, full_col_start, full_col_end) = expanded_range.bounds();
    let src_rows = src_row_end - src_row_start + 1;
    let src_cols = src_col_end - src_col_start + 1;
    let single_source = src_rows == 1 && src_cols == 1;

    for display_row in full_row_start..=full_row_end {
        for display_col in full_col_start..=full_col_end {
            let target_point = CellPoint {
                row: display_row,
                column: display_col,
            };
            if source_range.contains(target_point) {
                continue;
            }

            let Some(target_row_index) = visible_rows.get(display_row).copied() else {
                continue;
            };
            let Some(target_column_name) = columns.get(display_col).cloned() else {
                continue;
            };

            let source_display_row =
                src_row_start + wrap_index(display_row as isize - src_row_start as isize, src_rows);
            let source_display_col =
                src_col_start + wrap_index(display_col as isize - src_col_start as isize, src_cols);
            let Some(source_row_index) = visible_rows.get(source_display_row).copied() else {
                continue;
            };
            let Some(source_column_name) = columns.get(source_display_col) else {
                continue;
            };

            if let Some(source_formula) =
                snapshot.cell_formula(source_row_index, source_column_name)
            {
                plan.touches_formula = true;
                plan.edits.push(CellEdit {
                    row_index: target_row_index,
                    column: target_column_name,
                    kind: CellEditKind::Formula(source_formula),
                });
                continue;
            }

            let Some(mut source_value) = snapshot.cell_value(source_row_index, source_column_name)
            else {
                continue;
            };

            if let Some(delta) =
                increment_delta_for_target(single_source, source_range, target_point)
            {
                if let Some(incremented) = increment_numeric_value(&source_value, delta) {
                    source_value = incremented;
                }
            }

            plan.edits.push(CellEdit {
                row_index: target_row_index,
                column: target_column_name,
                kind: CellEditKind::Value(source_value),
            });
        }
    }

    plan
}

fn wrap_index(offset: isize, span: usize) -> usize {
    offset.rem_euclid(span as isize) as usize
}

fn increment_delta_for_target(
    single_source: bool,
    source_range: CellRange,
    target_point: CellPoint,
) -> Option<isize> {
    if !single_source {
        return None;
    }

    let (src_row_start, _, src_col_start, _) = source_range.bounds();
    if target_point.column == src_col_start && target_point.row != src_row_start {
        return Some(target_point.row as isize - src_row_start as isize);
    }
    if target_point.row == src_row_start && target_point.column != src_col_start {
        return Some(target_point.column as isize - src_col_start as isize);
    }
    None
}

fn increment_numeric_value(value: &Value, delta: isize) -> Option<Value> {
    let Value::Number(number) = value else {
        return None;
    };
    let base = number.as_f64()?;
    let next = base + delta as f64;
    json_number_from_f64(next).map(Value::Number)
}

fn json_number_from_f64(value: f64) -> Option<serde_json::Number> {
    if !value.is_finite() {
        return None;
    }

    if value.fract() == 0.0 {
        if value >= i64::MIN as f64 && value <= i64::MAX as f64 {
            return Some((value as i64).into());
        }
        if value >= 0.0 && value <= u64::MAX as f64 {
            return Some((value as u64).into());
        }
    }

    serde_json::Number::from_f64(value)
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
                    actions::persist_sidecar_if_possible(data, file_path, error_message);
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

fn enum_values_for_column(snapshot: &TableState, column: &str) -> Vec<String> {
    snapshot
        .validation_rule(column)
        .and_then(|rule| rule.enum_values.as_ref())
        .cloned()
        .unwrap_or_default()
}

fn has_enum_options(snapshot: &TableState, column: &str) -> bool {
    !enum_values_for_column(snapshot, column).is_empty()
}

fn enum_list_id(column: &str) -> String {
    format!("enum-options-{}", sanitize_id(column))
}

fn editing_enum_list_id(snapshot: &TableState, column: &str) -> String {
    if has_enum_options(snapshot, column) {
        enum_list_id(column)
    } else {
        String::new()
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

const ROW_NUMBER_WIDTH: usize = 50;
const FROZEN_COL_WIDTH: usize = 150;

fn frozen_left_px(col_idx: usize, frozen_count: usize) -> String {
    if col_idx >= frozen_count {
        return String::new();
    }
    let left = ROW_NUMBER_WIDTH + col_idx * FROZEN_COL_WIDTH;
    format!("{left}px")
}

fn frozen_left_style(col_idx: usize, frozen_count: usize) -> String {
    if col_idx >= frozen_count {
        return String::new();
    }
    let left = ROW_NUMBER_WIDTH + col_idx * FROZEN_COL_WIDTH;
    format!("left: {left}px;")
}

fn frozen_header_class(
    col: &str,
    selected_column: &Signal<Option<String>>,
    col_idx: usize,
    frozen_count: usize,
) -> String {
    let base = header_class(col, selected_column);
    if col_idx < frozen_count {
        join_classes(&base, "frozen-col")
    } else {
        base
    }
}

fn format_validation_number(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{n:.0}")
    } else {
        n.to_string()
    }
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}
