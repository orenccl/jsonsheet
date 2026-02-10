use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::state::data_model;
use crate::state::data_model::{Row, TableData};

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct JSheetMeta {
    #[serde(default)]
    pub columns: BTreeMap<String, ColumnConstraint>,
    #[serde(default)]
    pub computed_columns: BTreeMap<String, ComputedColumn>,
    #[serde(default)]
    pub summaries: BTreeMap<String, SummaryKind>,
    #[serde(default)]
    pub styles: BTreeMap<String, ColumnStyle>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputedColumn {
    pub formula: String,
    #[serde(default)]
    pub bake: bool,
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
        all.extend(self.computed_columns.keys().cloned());
        all.into_iter().collect()
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

    pub fn computed_column(&self, column: &str) -> Option<&ComputedColumn> {
        self.computed_columns.get(column)
    }

    pub fn set_computed_column(&mut self, column: String, formula: String, bake: bool) {
        self.computed_columns
            .insert(column, ComputedColumn { formula, bake });
    }

    pub fn remove_computed_column(&mut self, column: &str) {
        self.computed_columns.remove(column);
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

    pub fn style(&self, column: &str) -> Option<&ColumnStyle> {
        self.styles.get(column)
    }

    pub fn set_style(&mut self, column: &str, color: Option<String>, background: Option<String>) {
        let color = normalize_color(color);
        let background = normalize_color(background);

        if color.is_none() && background.is_none() {
            self.styles.remove(column);
        } else {
            self.styles
                .insert(column.to_string(), ColumnStyle { color, background });
        }
    }

    pub fn remove_column_metadata(&mut self, column: &str) {
        self.columns.remove(column);
        self.computed_columns.remove(column);
        self.summaries.remove(column);
        self.styles.remove(column);
    }

    pub fn style_inline(&self, column: &str) -> String {
        let Some(style) = self.style(column) else {
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

    pub fn validate_formula(formula: &str) -> bool {
        Parser::new(formula).parse().is_ok()
    }

    pub fn value_for_column(&self, row: &Row, column: &str) -> Option<Value> {
        self.value_for_column_inner(row, column, &mut BTreeSet::new())
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
            .filter_map(|idx| data.get(*idx))
            .filter_map(|row| self.value_for_column(row, column))
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

    pub fn export_row_with_baked_computed(&self, row: &Row) -> Result<Row, String> {
        let mut out = row.clone();

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

        for (column, computed) in &self.computed_columns {
            if computed.bake {
                let Some(value) = self.value_for_column(&out, column) else {
                    return Err(format!("Failed to evaluate computed column '{column}'"));
                };
                out.insert(column.clone(), value);
            } else {
                out.remove(column);
            }
        }

        Ok(out)
    }

    fn value_for_column_inner(
        &self,
        row: &Row,
        column: &str,
        stack: &mut BTreeSet<String>,
    ) -> Option<Value> {
        if let Some(value) = row.get(column) {
            return Some(value.clone());
        }

        let computed = self.computed_column(column)?;
        if !stack.insert(column.to_string()) {
            return None;
        }

        let parsed = Parser::new(&computed.formula).parse().ok()?;
        let value = self.eval_expr_for_row(&parsed, row, stack);
        stack.remove(column);
        Some(value)
    }

    fn eval_expr_for_row(&self, expr: &Expr, row: &Row, stack: &mut BTreeSet<String>) -> Value {
        match expr {
            Expr::Number(n) => json_number_from_f64(*n)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            Expr::String(s) => Value::String(s.clone()),
            Expr::Ident(name) => self
                .value_for_column_inner(row, name, stack)
                .unwrap_or(Value::Null),
            Expr::UnaryMinus(inner) => value_as_f64(self.eval_expr_for_row(inner, row, stack))
                .and_then(|n| json_number_from_f64(-n))
                .map(Value::Number)
                .unwrap_or(Value::Null),
            Expr::Binary { op, left, right } => {
                let left = self.eval_expr_for_row(left, row, stack);
                let right = self.eval_expr_for_row(right, row, stack);
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
