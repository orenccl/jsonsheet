use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::state::data_model;
use crate::state::data_model::{Row, TableData};

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct JSheetMeta {
    #[serde(default)]
    pub columns: BTreeMap<String, ColumnConstraint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub column_order: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_key: Option<String>,
    #[serde(default)]
    pub comment_columns: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comment_rows: Vec<Row>,
    #[serde(default)]
    pub summaries: BTreeMap<String, SummaryKind>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cell_formulas: Vec<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cell_styles: Vec<BTreeMap<String, ColumnStyle>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColumnType {
    String,
    Number,
    Bool,
    Null,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnConstraint {
    #[serde(rename = "type")]
    pub value_type: ColumnType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SummaryKind {
    Sum,
    Avg,
    Count,
    Min,
    Max,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ColumnStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
}

impl JSheetMeta {
    pub fn display_columns(&self, data: &TableData) -> Vec<String> {
        let mut all: BTreeSet<String> = data_model::derive_columns(data).into_iter().collect();
        all.extend(self.columns.keys().cloned());
        all.extend(self.comment_columns.iter().cloned());
        all.extend(self.summaries.keys().cloned());
        for row in &self.cell_formulas {
            all.extend(row.keys().cloned());
        }
        for row in &self.cell_styles {
            all.extend(row.keys().cloned());
        }

        if self.column_order.is_empty() {
            return all.into_iter().collect();
        }

        // Respect stored column_order, then append any new columns not in the order
        let mut result: Vec<String> = self
            .column_order
            .iter()
            .filter(|c| all.contains(c.as_str()))
            .cloned()
            .collect();
        let ordered: BTreeSet<String> = result.iter().cloned().collect();
        for col in &all {
            if !ordered.contains(col) {
                result.push(col.clone());
            }
        }
        result
    }

    pub fn set_column_order(&mut self, order: Vec<String>) {
        self.column_order = order;
    }

    pub fn row_key(&self) -> Option<&str> {
        self.row_key.as_deref()
    }

    pub fn set_row_key(&mut self, key: Option<String>) {
        self.row_key = key;
    }

    /// Auto-detect a suitable row key from the data (first column with all unique values).
    pub fn auto_detect_row_key(&mut self, data: &TableData) {
        if self.row_key.is_some() || data.is_empty() {
            return;
        }
        let columns = data_model::derive_columns(data);
        for col in &columns {
            let values: Vec<&Value> = data.iter().filter_map(|row| row.get(col)).collect();
            if values.len() != data.len() {
                continue;
            }
            let unique: BTreeSet<String> = values.iter().map(|v| v.to_string()).collect();
            if unique.len() == data.len() && values.iter().all(|v| !v.is_null()) {
                self.row_key = Some(col.clone());
                return;
            }
        }
    }

    pub fn column_type(&self, column: &str) -> Option<ColumnType> {
        self.columns.get(column).map(|c| c.value_type)
    }

    pub fn set_column_type(&mut self, column: &str, column_type: Option<ColumnType>) {
        match column_type {
            Some(column_type) => {
                self.columns.insert(
                    column.to_string(),
                    ColumnConstraint {
                        value_type: column_type,
                    },
                );
            }
            None => {
                self.columns.remove(column);
            }
        }
    }

    pub fn resize_row_metadata(&mut self, row_count: usize) {
        self.ensure_row_metadata_len(row_count);
        self.cell_formulas.truncate(row_count);
        self.cell_styles.truncate(row_count);
    }

    pub fn remove_row_metadata(&mut self, row_index: usize) {
        if row_index < self.cell_formulas.len() {
            self.cell_formulas.remove(row_index);
        }
        if row_index < self.cell_styles.len() {
            self.cell_styles.remove(row_index);
        }
    }

    pub fn reorder_row_metadata(&mut self, ordered_old_indices: &[usize]) {
        let old_formulas = std::mem::take(&mut self.cell_formulas);
        let old_styles = std::mem::take(&mut self.cell_styles);

        self.cell_formulas = ordered_old_indices
            .iter()
            .map(|idx| old_formulas.get(*idx).cloned().unwrap_or_default())
            .collect();
        self.cell_styles = ordered_old_indices
            .iter()
            .map(|idx| old_styles.get(*idx).cloned().unwrap_or_default())
            .collect();
    }

    pub fn formula_for_cell(&self, row_index: usize, column: &str) -> Option<&str> {
        self.cell_formulas
            .get(row_index)
            .and_then(|row| row.get(column))
            .map(String::as_str)
    }

    pub fn set_formula_for_cell(
        &mut self,
        row_index: usize,
        column: &str,
        formula: String,
    ) -> bool {
        let column = column.trim();
        if column.is_empty() {
            return false;
        }
        let Some(formula) = Self::normalize_formula(&formula) else {
            return false;
        };
        if !Self::validate_formula(&formula) {
            return false;
        }

        self.ensure_row_metadata_len(row_index + 1);
        self.cell_formulas[row_index].insert(column.to_string(), formula);
        true
    }

    pub fn remove_formula_for_cell(&mut self, row_index: usize, column: &str) {
        if let Some(row) = self.cell_formulas.get_mut(row_index) {
            row.remove(column);
        }
    }

    pub fn cell_style(&self, row_index: usize, column: &str) -> Option<&ColumnStyle> {
        self.cell_styles
            .get(row_index)
            .and_then(|row| row.get(column))
    }

    pub fn set_cell_style(
        &mut self,
        row_index: usize,
        column: &str,
        color: Option<String>,
        background: Option<String>,
    ) {
        let color = normalize_color(color);
        let background = normalize_color(background);

        self.ensure_row_metadata_len(row_index + 1);
        if color.is_none() && background.is_none() {
            if let Some(row) = self.cell_styles.get_mut(row_index) {
                row.remove(column);
            }
        } else {
            self.cell_styles[row_index]
                .insert(column.to_string(), ColumnStyle { color, background });
        }
    }

    fn ensure_row_metadata_len(&mut self, row_count: usize) {
        if self.cell_formulas.len() < row_count {
            self.cell_formulas
                .resize_with(row_count, BTreeMap::<String, String>::new);
        }
        if self.cell_styles.len() < row_count {
            self.cell_styles
                .resize_with(row_count, BTreeMap::<String, ColumnStyle>::new);
        }
    }

    pub fn clear_cell_style(&mut self, row_index: usize, column: &str) {
        if let Some(row) = self.cell_styles.get_mut(row_index) {
            row.remove(column);
        }
    }

    pub fn cell_style_inline(&self, row_index: usize, column: &str) -> String {
        let Some(style) = self.cell_style(row_index, column) else {
            return String::new();
        };

        let mut out = String::new();
        if let Some(color) = style.color.as_deref() {
            out.push_str("color: ");
            out.push_str(color);
            out.push(';');
        }
        if let Some(background) = style.background.as_deref() {
            out.push_str("background-color: ");
            out.push_str(background);
            out.push(';');
        }
        out
    }

    pub fn is_comment_column(&self, column: &str) -> bool {
        self.comment_columns.contains(column)
    }

    pub fn set_comment_column(&mut self, column: &str, is_comment: bool) {
        if is_comment {
            self.comment_columns.insert(column.to_string());
        } else {
            self.comment_columns.remove(column);
            for row in &mut self.comment_rows {
                row.remove(column);
            }
        }
    }

    pub fn apply_comment_rows(&self, data: &mut TableData) {
        if self.comment_columns.is_empty() {
            return;
        }

        for (idx, row) in data.iter_mut().enumerate() {
            if let Some(comment_row) = self.comment_rows.get(idx) {
                for column in &self.comment_columns {
                    if let Some(value) = comment_row.get(column) {
                        row.insert(column.clone(), value.clone());
                    } else {
                        row.entry(column.clone()).or_insert(Value::Null);
                    }
                }
            } else {
                for column in &self.comment_columns {
                    row.entry(column.clone()).or_insert(Value::Null);
                }
            }
        }
    }

    pub fn capture_comment_rows(&mut self, data: &TableData) {
        if self.comment_columns.is_empty() {
            self.comment_rows.clear();
            return;
        }

        self.comment_rows = data
            .iter()
            .map(|row| {
                self.comment_columns
                    .iter()
                    .filter_map(|column| {
                        row.get(column)
                            .cloned()
                            .map(|value| (column.clone(), value))
                    })
                    .collect()
            })
            .collect();
    }

    pub fn summary_kind(&self, column: &str) -> Option<SummaryKind> {
        self.summaries.get(column).copied()
    }

    pub fn set_summary_kind(&mut self, column: &str, summary_kind: Option<SummaryKind>) {
        match summary_kind {
            Some(kind) => {
                self.summaries.insert(column.to_string(), kind);
            }
            None => {
                self.summaries.remove(column);
            }
        }
    }

    pub fn remove_column_metadata(&mut self, column: &str) {
        self.columns.remove(column);
        self.comment_columns.remove(column);
        for row in &mut self.comment_rows {
            row.remove(column);
        }
        self.summaries.remove(column);
        for row in &mut self.cell_formulas {
            row.remove(column);
        }
        for row in &mut self.cell_styles {
            row.remove(column);
        }
    }

    pub fn validate_formula(formula: &str) -> bool {
        let Some(normalized) = Self::normalize_formula(formula) else {
            return false;
        };
        Parser::new(&normalized).parse().is_ok()
    }

    pub fn normalize_formula(raw: &str) -> Option<String> {
        let mut formula = raw.trim();
        if let Some(stripped) = formula.strip_prefix('=') {
            formula = stripped.trim_start();
        }
        if formula.is_empty() {
            return None;
        }
        Some(formula.to_string())
    }

    pub fn value_for_cell(&self, row: &Row, row_index: usize, column: &str) -> Option<Value> {
        self.value_for_cell_inner(row, row_index, column, &mut BTreeSet::new())
    }

    pub fn coerce_value_for_column(
        &self,
        column: &str,
        value: &Value,
        input: Option<&str>,
    ) -> Option<Value> {
        match self.column_type(column) {
            Some(column_type) => coerce_value(value, input, column_type),
            None => Some(value.clone()),
        }
    }

    pub fn summary_display_for_column(
        &self,
        data: &TableData,
        visible_rows: &[usize],
        column: &str,
    ) -> Option<String> {
        let kind = self.summary_kind(column)?;
        let values: Vec<Value> = visible_rows
            .iter()
            .filter_map(|idx| {
                data.get(*idx)
                    .and_then(|row| self.value_for_cell(row, *idx, column))
            })
            .collect();

        match kind {
            SummaryKind::Count => Some(values.iter().filter(|v| !v.is_null()).count().to_string()),
            SummaryKind::Sum => summarize_numbers(values, |nums| nums.iter().sum()),
            SummaryKind::Avg => {
                summarize_numbers(values, |nums| nums.iter().sum::<f64>() / nums.len() as f64)
            }
            SummaryKind::Min => summarize_numbers(values, |nums| {
                nums.iter()
                    .copied()
                    .fold(f64::INFINITY, |acc, next| acc.min(next))
            }),
            SummaryKind::Max => summarize_numbers(values, |nums| {
                nums.iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, |acc, next| acc.max(next))
            }),
        }
    }

    pub fn export_row_with_formulas(&self, row: &Row, row_index: usize) -> Result<Row, String> {
        let mut out = row.clone();

        if let Some(row_formulas) = self.cell_formulas.get(row_index) {
            for column in row_formulas.keys() {
                let Some(value) = self.value_for_cell(row, row_index, column) else {
                    return Err(format!(
                        "Failed to evaluate formula at row {row_index}, column '{column}'"
                    ));
                };
                out.insert(column.clone(), value);
            }
        }

        for (column, constraint) in &self.columns {
            if let Some(value) = out.get(column).cloned() {
                let Some(coerced) = coerce_value(&value, None, constraint.value_type) else {
                    return Err(format!(
                        "Column '{column}' value does not match declared type"
                    ));
                };
                out.insert(column.clone(), coerced);
            }
        }

        for column in &self.comment_columns {
            out.remove(column);
        }

        Ok(out)
    }

    fn value_for_cell_inner(
        &self,
        row: &Row,
        row_index: usize,
        column: &str,
        stack: &mut BTreeSet<(usize, String)>,
    ) -> Option<Value> {
        if let Some(formula) = self.formula_for_cell(row_index, column) {
            let key = (row_index, column.to_string());
            if !stack.insert(key.clone()) {
                return None;
            }

            let parsed = Parser::new(formula).parse().ok()?;
            let value = self.eval_expr_for_row(&parsed, row, row_index, stack);
            stack.remove(&key);
            return Some(value);
        }

        row.get(column).cloned()
    }

    fn eval_expr_for_row(
        &self,
        expr: &Expr,
        row: &Row,
        row_index: usize,
        stack: &mut BTreeSet<(usize, String)>,
    ) -> Value {
        match expr {
            Expr::Number(n) => json_number_from_f64(*n)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            Expr::String(s) => Value::String(s.clone()),
            Expr::Ident(name) => self
                .value_for_cell_inner(row, row_index, name, stack)
                .unwrap_or(Value::Null),
            Expr::UnaryMinus(inner) => {
                value_as_f64(self.eval_expr_for_row(inner, row, row_index, stack))
                    .and_then(|n| json_number_from_f64(-n))
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            }
            Expr::Binary { op, left, right } => {
                let left = self.eval_expr_for_row(left, row, row_index, stack);
                let right = self.eval_expr_for_row(right, row, row_index, stack);
                eval_binary(*op, left, right)
            }
        }
    }
}

fn summarize_numbers<F>(values: Vec<Value>, op: F) -> Option<String>
where
    F: Fn(&[f64]) -> f64,
{
    let nums: Vec<f64> = values.into_iter().filter_map(value_as_f64).collect();
    if nums.is_empty() {
        return None;
    }
    Some(format_number(op(&nums)))
}

fn normalize_color(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn coerce_value(value: &Value, input: Option<&str>, column_type: ColumnType) -> Option<Value> {
    match column_type {
        ColumnType::String => Some(Value::String(data_model::display_value(value))),
        ColumnType::Number => coerce_number(value, input).map(Value::Number),
        ColumnType::Bool => coerce_bool(value, input).map(Value::Bool),
        ColumnType::Null => {
            if value.is_null()
                || input
                    .map(|raw| {
                        let trimmed = raw.trim();
                        trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null")
                    })
                    .unwrap_or(false)
            {
                Some(Value::Null)
            } else {
                None
            }
        }
    }
}

fn coerce_number(value: &Value, input: Option<&str>) -> Option<serde_json::Number> {
    match value {
        Value::Number(n) => Some(n.clone()),
        Value::String(s) => parse_number(s),
        Value::Bool(b) => Some(if *b { 1 } else { 0 }.into()),
        Value::Null => input.and_then(parse_number),
        _ => None,
    }
}

fn parse_number(raw: &str) -> Option<serde_json::Number> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(i) = trimmed.parse::<i64>() {
        return Some(i.into());
    }
    if let Ok(u) = trimmed.parse::<u64>() {
        return Some(u.into());
    }
    let f = trimmed.parse::<f64>().ok()?;
    serde_json::Number::from_f64(f)
}

fn coerce_bool(value: &Value, input: Option<&str>) -> Option<bool> {
    match value {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => value_as_f64(Value::Number(n.clone())).map(|v| v != 0.0),
        Value::String(s) => parse_bool(s),
        Value::Null => input.and_then(parse_bool),
        _ => None,
    }
}

fn parse_bool(raw: &str) -> Option<bool> {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("true") || trimmed == "1" {
        Some(true)
    } else if trimmed.eq_ignore_ascii_case("false") || trimmed == "0" {
        Some(false)
    } else {
        None
    }
}

fn value_as_f64(value: Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        Value::Bool(b) => Some(if b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return String::new();
    }
    if value.fract() == 0.0 {
        return format!("{value:.0}");
    }
    let mut out = format!("{value:.6}");
    while out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
enum Expr {
    Number(f64),
    String(String),
    Ident(String),
    UnaryMinus(Box<Expr>),
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

fn eval_binary(op: BinOp, left: Value, right: Value) -> Value {
    match op {
        BinOp::Add => {
            if let (Some(a), Some(b)) = (value_as_f64(left.clone()), value_as_f64(right.clone())) {
                json_number_from_f64(a + b)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            } else {
                Value::String(format!(
                    "{}{}",
                    data_model::display_value(&left),
                    data_model::display_value(&right)
                ))
            }
        }
        BinOp::Sub => numeric_binary(left, right, |a, b| a - b),
        BinOp::Mul => numeric_binary(left, right, |a, b| a * b),
        BinOp::Div => numeric_binary(left, right, |a, b| if b == 0.0 { f64::NAN } else { a / b }),
    }
}

fn numeric_binary(left: Value, right: Value, op: impl Fn(f64, f64) -> f64) -> Value {
    let Some(a) = value_as_f64(left) else {
        return Value::Null;
    };
    let Some(b) = value_as_f64(right) else {
        return Value::Null;
    };
    json_number_from_f64(op(a, b))
        .map(Value::Number)
        .unwrap_or(Value::Null)
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

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    String(String),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    fn tokenize(mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        while let Some(ch) = self.chars.peek().copied() {
            if ch.is_ascii_whitespace() {
                self.chars.next();
                continue;
            }

            if ch.is_ascii_digit() || ch == '.' {
                tokens.push(Token::Number(self.consume_number()?));
                continue;
            }

            if ch == '"' {
                tokens.push(Token::String(self.consume_string()?));
                continue;
            }

            if ch.is_ascii_alphabetic() || ch == '_' {
                tokens.push(Token::Ident(self.consume_ident()));
                continue;
            }

            self.chars.next();
            tokens.push(match ch {
                '+' => Token::Plus,
                '-' => Token::Minus,
                '*' => Token::Star,
                '/' => Token::Slash,
                '(' => Token::LParen,
                ')' => Token::RParen,
                _ => return Err(format!("Unexpected token '{ch}'")),
            });
        }
        Ok(tokens)
    }

    fn consume_number(&mut self) -> Result<f64, String> {
        let mut buf = String::new();
        while let Some(ch) = self.chars.peek().copied() {
            if ch.is_ascii_digit() || ch == '.' {
                buf.push(ch);
                self.chars.next();
            } else {
                break;
            }
        }
        buf.parse::<f64>()
            .map_err(|_| format!("Invalid number literal '{buf}'"))
    }

    fn consume_string(&mut self) -> Result<String, String> {
        let mut out = String::new();
        let mut escaped = false;
        self.chars.next(); // opening quote
        for ch in self.chars.by_ref() {
            if escaped {
                out.push(match ch {
                    'n' => '\n',
                    't' => '\t',
                    '"' => '"',
                    '\\' => '\\',
                    other => other,
                });
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                return Ok(out);
            }
            out.push(ch);
        }
        Err("Unterminated string literal".to_string())
    }

    fn consume_ident(&mut self) -> String {
        let mut out = String::new();
        while let Some(ch) = self.chars.peek().copied() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                out.push(ch);
                self.chars.next();
            } else {
                break;
            }
        }
        out
    }
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    source: &'a str,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            tokens: Vec::new(),
            pos: 0,
            source,
        }
    }

    fn parse(mut self) -> Result<Expr, String> {
        self.tokens = Lexer::new(self.source).tokenize()?;
        if self.tokens.is_empty() {
            return Err("Formula is empty".to_string());
        }
        let expr = self.parse_expr()?;
        if self.pos != self.tokens.len() {
            return Err("Unexpected trailing tokens".to_string());
        }
        Ok(expr)
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_term()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => break,
            };
            self.pos += 1;
            let right = self.parse_term()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_factor()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => BinOp::Mul,
                Some(Token::Slash) => BinOp::Div,
                _ => break,
            };
            self.pos += 1;
            let right = self.parse_factor()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Token::Minus) => {
                self.pos += 1;
                let inner = self.parse_factor()?;
                Ok(Expr::UnaryMinus(Box::new(inner)))
            }
            Some(Token::Number(n)) => {
                self.pos += 1;
                Ok(Expr::Number(n))
            }
            Some(Token::String(s)) => {
                self.pos += 1;
                Ok(Expr::String(s))
            }
            Some(Token::Ident(name)) => {
                self.pos += 1;
                Ok(Expr::Ident(name))
            }
            Some(Token::LParen) => {
                self.pos += 1;
                let expr = self.parse_expr()?;
                if !matches!(self.peek(), Some(Token::RParen)) {
                    return Err("Missing closing ')'".to_string());
                }
                self.pos += 1;
                Ok(expr)
            }
            _ => Err("Expected expression".to_string()),
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
}
