use std::cmp::Ordering;

use serde_json::{Number, Value};

use crate::state::data_model::{self, Row, TableData};
use crate::state::jsheet::{ColumnStyle, ColumnType, ComputedColumn, JSheetMeta, SummaryKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SortSpec {
    pub column: String,
    pub order: SortOrder,
}

#[derive(Clone, Debug, PartialEq)]
struct HistoryEntry {
    data: TableData,
    sort_spec: Option<SortSpec>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TableState {
    data: TableData,
    jsheet_meta: JSheetMeta,
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    sort_spec: Option<SortSpec>,
    filter_column: Option<String>,
    filter_query: String,
    search_query: String,
}

impl TableState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_data(data: TableData) -> Self {
        Self {
            data,
            ..Self::default()
        }
    }

    pub fn from_data_and_jsheet(data: TableData, jsheet_meta: JSheetMeta) -> Self {
        Self {
            data,
            jsheet_meta,
            ..Self::default()
        }
    }

    pub fn replace_data(&mut self, data: TableData) {
        self.replace_data_and_jsheet(data, JSheetMeta::default());
    }

    pub fn replace_data_and_jsheet(&mut self, data: TableData, jsheet_meta: JSheetMeta) {
        self.data = data;
        self.jsheet_meta = jsheet_meta;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.sort_spec = None;
        self.filter_column = None;
        self.filter_query.clear();
        self.search_query.clear();
    }

    pub fn data(&self) -> &TableData {
        &self.data
    }

    pub fn jsheet_meta(&self) -> &JSheetMeta {
        &self.jsheet_meta
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn sort_spec(&self) -> Option<&SortSpec> {
        self.sort_spec.as_ref()
    }

    pub fn filter_column(&self) -> Option<&str> {
        self.filter_column.as_deref()
    }

    pub fn filter_query(&self) -> &str {
        &self.filter_query
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn display_columns(&self) -> Vec<String> {
        self.jsheet_meta.display_columns(&self.data)
    }

    pub fn column_type(&self, column: &str) -> Option<ColumnType> {
        self.jsheet_meta.column_type(column)
    }

    pub fn set_column_type(&mut self, column: &str, column_type: Option<ColumnType>) {
        self.jsheet_meta.set_column_type(column, column_type);
    }

    pub fn computed_column(&self, column: &str) -> Option<&ComputedColumn> {
        self.jsheet_meta.computed_column(column)
    }

    pub fn set_computed_column(&mut self, column: &str, formula: String, bake: bool) -> bool {
        let formula = formula.trim().to_string();
        if formula.is_empty() || !JSheetMeta::validate_formula(&formula) {
            return false;
        }
        self.jsheet_meta
            .set_computed_column(column.trim().to_string(), formula, bake);
        true
    }

    pub fn remove_computed_column(&mut self, column: &str) {
        self.jsheet_meta.remove_computed_column(column);
    }

    pub fn summary_kind(&self, column: &str) -> Option<SummaryKind> {
        self.jsheet_meta.summary_kind(column)
    }

    pub fn set_summary_kind(&mut self, column: &str, summary_kind: Option<SummaryKind>) {
        self.jsheet_meta.set_summary_kind(column, summary_kind);
    }

    pub fn column_style(&self, column: &str) -> Option<ColumnStyle> {
        self.jsheet_meta.style(column).cloned()
    }

    pub fn set_column_style(
        &mut self,
        column: &str,
        color: Option<String>,
        background: Option<String>,
    ) {
        self.jsheet_meta.set_style(column, color, background);
    }

    pub fn column_inline_style(&self, column: &str) -> String {
        self.jsheet_meta.style_inline(column)
    }

    pub fn export_json_data(&self) -> Result<TableData, String> {
        self.data
            .iter()
            .map(|row| self.jsheet_meta.export_row_with_baked_computed(row))
            .collect()
    }

    pub fn summary_display_for_column(&self, column: &str) -> Option<String> {
        let rows = self.visible_row_indices();
        self.jsheet_meta
            .summary_display_for_column(&self.data, &rows, column)
    }

    pub fn row_with_computed(&self, row_index: usize) -> Option<Row> {
        let base = self.data.get(row_index)?;
        let mut row = base.clone();
        for column in self.jsheet_meta.computed_columns.keys() {
            if let Some(value) = self.jsheet_meta.value_for_column(base, column) {
                row.insert(column.clone(), value);
            }
        }
        Some(row)
    }

    pub fn cell_value(&self, row_index: usize, column: &str) -> Option<Value> {
        let row = self.data.get(row_index)?;
        self.jsheet_meta.value_for_column(row, column)
    }

    pub fn cell_display_value(&self, row_index: usize, column: &str) -> String {
        self.cell_value(row_index, column)
            .as_ref()
            .map(data_model::display_value)
            .unwrap_or_default()
    }

    pub fn undo(&mut self) -> bool {
        if let Some(entry) = self.undo_stack.pop() {
            self.redo_stack.push(self.snapshot());
            self.restore(entry);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(entry) = self.redo_stack.pop() {
            self.undo_stack.push(self.snapshot());
            self.restore(entry);
            true
        } else {
            false
        }
    }

    pub fn set_cell_from_input(&mut self, row_index: usize, column: &str, input: &str) -> bool {
        let parsed = data_model::parse_cell_input(input);
        let Some(value) = self
            .jsheet_meta
            .coerce_value_for_column(column, &parsed, Some(input))
        else {
            return false;
        };
        self.set_cell_value(row_index, column, value)
    }

    pub fn set_cell_value(&mut self, row_index: usize, column: &str, value: Value) -> bool {
        let Some(value) = self
            .jsheet_meta
            .coerce_value_for_column(column, &value, None)
        else {
            return false;
        };

        let Some(row) = self.data.get(row_index) else {
            return false;
        };

        if row.get(column) == Some(&value) {
            return false;
        }

        self.push_undo_snapshot();
        self.sort_spec = None;
        data_model::set_cell_value(&mut self.data, row_index, column, value)
    }

    pub fn add_row(&mut self) -> bool {
        self.push_undo_snapshot();
        self.sort_spec = None;
        data_model::add_row(&mut self.data);
        true
    }

    pub fn delete_row(&mut self, row_index: usize) -> bool {
        if row_index >= self.data.len() {
            return false;
        }

        self.push_undo_snapshot();
        self.sort_spec = None;
        data_model::delete_row(&mut self.data, row_index)
    }

    pub fn add_column(&mut self, name: &str) -> bool {
        let mut next = self.data.clone();
        if !data_model::add_column(&mut next, name) {
            return false;
        }

        self.push_undo_snapshot();
        self.sort_spec = None;
        self.data = next;
        true
    }

    pub fn delete_column(&mut self, name: &str) -> bool {
        let trimmed = name.trim();
        let mut next = self.data.clone();
        if !data_model::delete_column(&mut next, trimmed) {
            return false;
        }

        self.push_undo_snapshot();
        self.sort_spec = None;
        self.jsheet_meta.remove_column_metadata(trimmed);
        if self.filter_column.as_deref() == Some(trimmed) {
            self.clear_filter();
        }
        self.data = next;
        true
    }

    pub fn sort_by_column_toggle(&mut self, column: &str) -> bool {
        let next_order = match self.sort_spec.as_ref() {
            Some(spec) if spec.column == column => toggle_sort_order(&spec.order),
            _ => SortOrder::Asc,
        };

        let mut sorted = self.data.clone();
        let meta = self.jsheet_meta.clone();
        sorted.sort_by(|a, b| {
            let left = meta.value_for_column(a, column);
            let right = meta.value_for_column(b, column);
            compare_values(left.as_ref(), right.as_ref())
        });
        if matches!(next_order, SortOrder::Desc) {
            sorted.reverse();
        }

        let next_spec = Some(SortSpec {
            column: column.to_string(),
            order: next_order,
        });

        if sorted == self.data && next_spec == self.sort_spec {
            return false;
        }

        self.push_undo_snapshot();
        self.data = sorted;
        self.sort_spec = next_spec;
        true
    }

    pub fn set_filter(&mut self, column: Option<String>, query: String) {
        self.filter_column = column;
        self.filter_query = query.trim().to_string();
    }

    pub fn clear_filter(&mut self) {
        self.filter_column = None;
        self.filter_query.clear();
    }

    pub fn set_search(&mut self, query: String) {
        self.search_query = query.trim().to_string();
    }

    pub fn visible_row_indices(&self) -> Vec<usize> {
        self.data
            .iter()
            .enumerate()
            .filter_map(|(idx, row)| self.row_matches_filter(row).then_some(idx))
            .collect()
    }

    pub fn cell_matches_search(&self, row_index: usize, column: &str) -> bool {
        if self.search_query.is_empty() {
            return false;
        }

        let Some(row) = self.data.get(row_index) else {
            return false;
        };

        let needle = self.search_query.to_ascii_lowercase();
        self.jsheet_meta
            .value_for_column(row, column)
            .map(|value| {
                data_model::display_value(&value)
                    .to_ascii_lowercase()
                    .contains(&needle)
            })
            .unwrap_or(false)
    }

    fn row_matches_filter(&self, row: &Row) -> bool {
        if self.filter_query.is_empty() {
            return true;
        }

        let Some(column) = self.filter_column.as_ref() else {
            return true;
        };

        let needle = self.filter_query.to_ascii_lowercase();
        self.jsheet_meta
            .value_for_column(row, column)
            .map(|value| {
                data_model::display_value(&value)
                    .to_ascii_lowercase()
                    .contains(&needle)
            })
            .unwrap_or(false)
    }

    fn snapshot(&self) -> HistoryEntry {
        HistoryEntry {
            data: self.data.clone(),
            sort_spec: self.sort_spec.clone(),
        }
    }

    fn push_undo_snapshot(&mut self) {
        self.undo_stack.push(self.snapshot());
        self.redo_stack.clear();
    }

    fn restore(&mut self, entry: HistoryEntry) {
        self.data = entry.data;
        self.sort_spec = entry.sort_spec;
    }
}

fn toggle_sort_order(order: &SortOrder) -> SortOrder {
    match order {
        SortOrder::Asc => SortOrder::Desc,
        SortOrder::Desc => SortOrder::Asc,
    }
}

fn compare_values(a: Option<&Value>, b: Option<&Value>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(left), Some(right)) => compare_value_pair(left, right),
    }
}

fn compare_value_pair(left: &Value, right: &Value) -> Ordering {
    match (left, right) {
        (Value::Null, Value::Null) => Ordering::Equal,
        (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
        (Value::Number(a), Value::Number(b)) => compare_numbers(a, b),
        (Value::String(a), Value::String(b)) => a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()),
        _ => type_rank(left)
            .cmp(&type_rank(right))
            .then_with(|| data_model::display_value(left).cmp(&data_model::display_value(right))),
    }
}

fn compare_numbers(left: &Number, right: &Number) -> Ordering {
    match (left.as_i64(), left.as_u64(), right.as_i64(), right.as_u64()) {
        (Some(a), _, Some(b), _) => a.cmp(&b),
        (Some(a), _, _, Some(b)) => {
            if a < 0 {
                Ordering::Less
            } else {
                (a as u64).cmp(&b)
            }
        }
        (_, Some(a), Some(b), _) => {
            if b < 0 {
                Ordering::Greater
            } else {
                a.cmp(&(b as u64))
            }
        }
        (_, Some(a), _, Some(b)) => a.cmp(&b),
        _ => {
            let left = left.as_f64().unwrap_or(f64::NAN);
            let right = right.as_f64().unwrap_or(f64::NAN);
            left.partial_cmp(&right).unwrap_or(Ordering::Equal)
        }
    }
}

fn type_rank(value: &Value) -> u8 {
    match value {
        Value::Null => 0,
        Value::Bool(_) => 1,
        Value::Number(_) => 2,
        Value::String(_) => 3,
        Value::Array(_) => 4,
        Value::Object(_) => 5,
    }
}
