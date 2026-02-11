use serde_json::Value;
use std::collections::BTreeMap;

use jsonsheet::state::jsheet::{
    ColumnStyle, ColumnType, ConditionalFormat, ParsedCondRule, SummaryKind, ValidationRule,
};
use jsonsheet::state::table_state::{CellEdit, CellEditKind, TableState};

fn sample_state() -> TableState {
    TableState::from_data(vec![
        BTreeMap::from([
            ("name".to_string(), Value::String("Alice".to_string())),
            ("age".to_string(), Value::Number(30.into())),
        ]),
        BTreeMap::from([
            ("name".to_string(), Value::String("Bob".to_string())),
            ("age".to_string(), Value::Number(25.into())),
        ]),
    ])
}

#[test]
fn test_type_constraint_blocks_invalid_input() {
    let mut state = sample_state();
    state.set_column_type("age", Some(ColumnType::Number));

    assert!(!state.set_cell_from_input(0, "age", "not_a_number"));
    assert_eq!(state.data()[0]["age"], Value::Number(30.into()));
}

#[test]
fn test_type_constraint_coerces_valid_input() {
    let mut state = sample_state();
    state.set_column_type("age", Some(ColumnType::Number));

    assert!(state.set_cell_from_input(0, "age", "42"));
    assert_eq!(state.data()[0]["age"], Value::Number(42.into()));
}

#[test]
fn test_cell_formula_display_and_export_baked_by_default() {
    let mut state = sample_state();
    assert!(state.add_column("age2"));
    assert!(state.set_cell_formula(0, "age2", "=age * 2".to_string()));
    assert!(state.set_cell_formula(1, "age2", "=age * 2".to_string()));

    assert_eq!(state.cell_display_value(0, "age2"), "60");
    assert_eq!(state.cell_display_value(1, "age2"), "50");

    let exported = state.export_json_data().unwrap();
    assert_eq!(exported[0]["age2"], Value::Number(60.into()));
    assert_eq!(exported[1]["age2"], Value::Number(50.into()));
}

#[test]
fn test_summary_values_for_base_and_formula_columns() {
    let mut state = sample_state();
    assert!(state.add_column("age2"));
    assert!(state.set_cell_formula(0, "age2", "=age * 2".to_string()));
    assert!(state.set_cell_formula(1, "age2", "=age * 2".to_string()));
    state.set_summary_kind("age", Some(SummaryKind::Avg));
    state.set_summary_kind("age2", Some(SummaryKind::Sum));

    assert_eq!(
        state.summary_display_for_column("age"),
        Some("27.5".to_string())
    );
    assert_eq!(
        state.summary_display_for_column("age2"),
        Some("110".to_string())
    );
}

#[test]
fn test_cell_style_inline_output() {
    let mut state = sample_state();
    state.set_cell_style(
        0,
        "age",
        Some("#aa0000".to_string()),
        Some("#f0f0f0".to_string()),
    );

    let style = state.cell_style(0, "age").unwrap_or_default();
    assert_eq!(style.color.as_deref(), Some("#aa0000"));
    assert_eq!(style.background.as_deref(), Some("#f0f0f0"));

    let inline = state.cell_inline_style(0, "age");
    assert!(inline.contains("color: #aa0000;"));
    assert!(inline.contains("background-color: #f0f0f0;"));
}

#[test]
fn test_comment_column_excluded_from_export_json() {
    let mut state = sample_state();
    assert!(state.add_column("note"));
    assert!(state.set_cell_from_input(0, "note", "keep in jsheet only"));
    state.set_comment_column("note", true);

    let exported = state.export_json_data().unwrap();
    assert!(!exported[0].contains_key("note"));
    assert!(!exported[1].contains_key("note"));
}

#[test]
fn test_comment_rows_are_captured_for_sidecar_save() {
    let mut state = sample_state();
    assert!(state.add_column("note"));
    assert!(state.set_cell_from_input(0, "note", "internal"));
    state.set_comment_column("note", true);

    let meta = state.jsheet_meta_for_save();
    assert!(meta.comment_columns.contains("note"));
    assert_eq!(
        meta.comment_rows[0]["note"],
        Value::String("internal".to_string())
    );
}

#[test]
fn test_set_cell_formula_accepts_excel_style_formula_input() {
    let mut state = sample_state();
    assert!(state.add_column("age2"));
    assert!(state.set_cell_formula(0, "age2", "=age * 2".to_string()));
    assert_eq!(state.cell_display_value(0, "age2"), "60");
}

#[test]
fn test_cell_formula_takes_priority_over_existing_json_value() {
    let mut state = sample_state();
    assert!(state.add_column("age2"));
    assert!(state.set_cell_from_input(0, "age2", "1"));
    assert!(state.set_cell_formula(0, "age2", "=age * 2".to_string()));

    assert_eq!(state.cell_display_value(0, "age2"), "60");
    let exported = state.export_json_data().unwrap();
    assert_eq!(exported[0]["age2"], Value::Number(60.into()));
}

#[test]
fn test_sort_keeps_row_bound_formula_metadata() {
    let mut state = sample_state();
    assert!(state.add_column("score"));
    assert!(state.set_cell_formula(0, "score", "=age + 100".to_string()));
    assert!(state.set_cell_formula(1, "score", "=age + 200".to_string()));

    assert!(state.sort_by_column_toggle("age"));

    assert_eq!(state.cell_display_value(0, "name"), "Bob");
    assert_eq!(state.cell_display_value(0, "score"), "225");
    assert_eq!(state.cell_display_value(1, "name"), "Alice");
    assert_eq!(state.cell_display_value(1, "score"), "130");
}

#[test]
fn test_fixture_sidecar_cell_formula_is_loaded() {
    let path = std::path::Path::new("tests/data/types.json");
    let (rows, meta) = jsonsheet::io::jsheet_io::load_json_and_sidecar(path).unwrap();
    let state = TableState::from_data_and_jsheet(rows, meta);

    assert_eq!(state.cell_display_value(0, "age2"), "60");
    assert_eq!(state.cell_display_value(1, "age2"), "50");
}

#[test]
fn test_parsed_cond_rule_numeric_operators() {
    let rule_lt = ParsedCondRule::parse("< 100").unwrap();
    assert!(rule_lt.matches(&Value::Number(50.into())));
    assert!(!rule_lt.matches(&Value::Number(100.into())));
    assert!(!rule_lt.matches(&Value::Number(200.into())));

    let rule_ge = ParsedCondRule::parse(">= 25").unwrap();
    assert!(rule_ge.matches(&Value::Number(25.into())));
    assert!(rule_ge.matches(&Value::Number(30.into())));
    assert!(!rule_ge.matches(&Value::Number(24.into())));

    let rule_eq = ParsedCondRule::parse("== 42").unwrap();
    assert!(rule_eq.matches(&Value::Number(42.into())));
    assert!(!rule_eq.matches(&Value::Number(43.into())));

    let rule_ne = ParsedCondRule::parse("!= 0").unwrap();
    assert!(rule_ne.matches(&Value::Number(1.into())));
    assert!(!rule_ne.matches(&Value::Number(0.into())));
}

#[test]
fn test_parsed_cond_rule_string_comparison() {
    let rule = ParsedCondRule::parse("== legendary").unwrap();
    assert!(rule.matches(&Value::String("legendary".to_string())));
    assert!(rule.matches(&Value::String("Legendary".to_string()))); // case insensitive
    assert!(!rule.matches(&Value::String("rare".to_string())));
}

#[test]
fn test_parsed_cond_rule_invalid_syntax() {
    assert!(ParsedCondRule::parse("").is_none());
    assert!(ParsedCondRule::parse("abc").is_none());
    assert!(ParsedCondRule::parse("< ").is_none());
    assert!(ParsedCondRule::parse("<").is_none());
}

#[test]
fn test_conditional_format_affects_cell_style_inline() {
    let mut state = sample_state();
    state.add_conditional_format(ConditionalFormat {
        column: "age".to_string(),
        rule: "< 30".to_string(),
        style: ColumnStyle {
            color: Some("#ff0000".to_string()),
            background: None,
        },
    });

    // age=30 does not match "< 30"
    let inline_0 = state.cell_inline_style(0, "age");
    assert!(!inline_0.contains("#ff0000"));

    // age=25 matches "< 30"
    let inline_1 = state.cell_inline_style(1, "age");
    assert!(inline_1.contains("color: #ff0000;"));
}

#[test]
fn test_cell_style_overrides_conditional_format() {
    let mut state = sample_state();
    state.add_conditional_format(ConditionalFormat {
        column: "age".to_string(),
        rule: "< 30".to_string(),
        style: ColumnStyle {
            color: Some("#ff0000".to_string()),
            background: None,
        },
    });
    // Set explicit cell style on row 1 (age=25, matches rule)
    state.set_cell_style(1, "age", Some("#00ff00".to_string()), None);

    let inline = state.cell_inline_style(1, "age");
    // Cell style (#00ff00) should override conditional format (#ff0000)
    assert!(inline.contains("color: #00ff00;"));
    assert!(!inline.contains("#ff0000"));
}

#[test]
fn test_remove_conditional_format() {
    let mut state = sample_state();
    state.add_conditional_format(ConditionalFormat {
        column: "age".to_string(),
        rule: "< 30".to_string(),
        style: ColumnStyle {
            color: Some("#ff0000".to_string()),
            background: None,
        },
    });
    assert_eq!(state.conditional_formats().len(), 1);
    assert!(state.remove_conditional_format(0));
    assert_eq!(state.conditional_formats().len(), 0);

    // After removal, no conditional style applied
    let inline = state.cell_inline_style(1, "age");
    assert!(inline.is_empty());
}

#[test]
fn test_conditional_format_roundtrip_via_sidecar() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("data.json");
    let rows = vec![
        BTreeMap::from([
            ("id".to_string(), Value::Number(1.into())),
            ("hp".to_string(), Value::Number(50.into())),
        ]),
        BTreeMap::from([
            ("id".to_string(), Value::Number(2.into())),
            ("hp".to_string(), Value::Number(200.into())),
        ]),
    ];
    jsonsheet::io::json_io::save_json(&json_path, &rows).unwrap();

    let mut meta = jsonsheet::state::jsheet::JSheetMeta::default();
    meta.add_conditional_format(ConditionalFormat {
        column: "hp".to_string(),
        rule: "< 100".to_string(),
        style: ColumnStyle {
            color: Some("#ff0000".to_string()),
            background: None,
        },
    });
    jsonsheet::io::jsheet_io::save_sidecar_for_json(&json_path, &meta, &rows).unwrap();

    let loaded = jsonsheet::io::jsheet_io::load_sidecar_with_data(&json_path, &rows).unwrap();
    assert_eq!(loaded.conditional_formats.len(), 1);
    assert_eq!(loaded.conditional_formats[0].column, "hp");
    assert_eq!(loaded.conditional_formats[0].rule, "< 100");
}

// ── Phase 8: Data Validation + Freeze Panes ──

#[test]
fn test_validation_min_max_blocks_out_of_range() {
    let mut state = sample_state();
    state.set_column_type("age", Some(ColumnType::Number));
    state.set_validation_rule(
        "age",
        Some(ValidationRule {
            min: Some(0.0),
            max: Some(100.0),
            ..Default::default()
        }),
    );

    // Within range: should succeed
    assert!(state.set_cell_from_input(0, "age", "50"));
    assert_eq!(state.data()[0]["age"], Value::Number(50.into()));

    // Below min: should fail
    assert!(!state.set_cell_from_input(0, "age", "-1"));
    assert_eq!(state.data()[0]["age"], Value::Number(50.into()));

    // Above max: should fail
    assert!(!state.set_cell_from_input(0, "age", "101"));
    assert_eq!(state.data()[0]["age"], Value::Number(50.into()));

    // Boundary: min and max should succeed
    assert!(state.set_cell_from_input(0, "age", "0"));
    assert!(state.set_cell_from_input(0, "age", "100"));
}

#[test]
fn test_validation_enum_blocks_invalid_value() {
    let mut state = sample_state();
    state.set_validation_rule(
        "name",
        Some(ValidationRule {
            enum_values: Some(vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ]),
            ..Default::default()
        }),
    );

    // Valid enum value: should succeed
    assert!(state.set_cell_from_input(0, "name", "Charlie"));
    assert_eq!(
        state.data()[0]["name"],
        Value::String("Charlie".to_string())
    );

    // Invalid enum value: should fail
    assert!(!state.set_cell_from_input(0, "name", "Dave"));
    assert_eq!(
        state.data()[0]["name"],
        Value::String("Charlie".to_string())
    );

    // Case insensitive match
    assert!(state.set_cell_from_input(0, "name", "alice"));
}

#[test]
fn test_validation_enum_allows_null() {
    let mut state = sample_state();
    assert!(state.add_column("rarity"));
    state.set_validation_rule(
        "rarity",
        Some(ValidationRule {
            enum_values: Some(vec!["common".to_string(), "rare".to_string()]),
            ..Default::default()
        }),
    );

    // Move away from default null first so the next set operation is a real change.
    assert!(state.set_cell_from_input(0, "rarity", "common"));

    // Null values should be allowed even with enum constraint
    assert!(state.set_cell_from_input(0, "rarity", "null"));
}

#[test]
fn test_validation_rule_removed_with_column() {
    let mut state = sample_state();
    state.set_validation_rule(
        "age",
        Some(ValidationRule {
            min: Some(0.0),
            ..Default::default()
        }),
    );
    assert!(state.validation_rule("age").is_some());
    assert!(state.delete_column("age"));
    assert!(state.validation_rule("age").is_none());
}

#[test]
fn test_validation_rule() {
    let rule = ValidationRule {
        min: Some(10.0),
        max: Some(50.0),
        enum_values: None,
    };
    assert!(rule.validate(&Value::Number(10.into())));
    assert!(rule.validate(&Value::Number(50.into())));
    assert!(rule.validate(&Value::Number(30.into())));
    assert!(!rule.validate(&Value::Number(9.into())));
    assert!(!rule.validate(&Value::Number(51.into())));
}

#[test]
fn test_frozen_columns_getter_setter() {
    let mut state = sample_state();
    assert_eq!(state.frozen_columns(), 0);
    state.set_frozen_columns(Some(2));
    assert_eq!(state.frozen_columns(), 2);
    state.set_frozen_columns(Some(0));
    assert_eq!(state.frozen_columns(), 0);
    state.set_frozen_columns(None);
    assert_eq!(state.frozen_columns(), 0);
}

#[test]
fn test_validation_roundtrip_via_sidecar() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("data.json");
    let rows = vec![BTreeMap::from([
        ("id".to_string(), Value::Number(1.into())),
        ("hp".to_string(), Value::Number(50.into())),
        ("rarity".to_string(), Value::String("common".to_string())),
    ])];
    jsonsheet::io::json_io::save_json(&json_path, &rows).unwrap();

    let mut meta = jsonsheet::state::jsheet::JSheetMeta::default();
    meta.set_validation_rule(
        "hp",
        Some(ValidationRule {
            min: Some(0.0),
            max: Some(9999.0),
            ..Default::default()
        }),
    );
    meta.set_validation_rule(
        "rarity",
        Some(ValidationRule {
            enum_values: Some(vec![
                "common".to_string(),
                "rare".to_string(),
                "epic".to_string(),
            ]),
            ..Default::default()
        }),
    );
    meta.set_frozen_columns(Some(2));

    jsonsheet::io::jsheet_io::save_sidecar_for_json(&json_path, &meta, &rows).unwrap();

    let loaded = jsonsheet::io::jsheet_io::load_sidecar_with_data(&json_path, &rows).unwrap();
    let hp_rule = loaded.validation.get("hp").unwrap();
    assert_eq!(hp_rule.min, Some(0.0));
    assert_eq!(hp_rule.max, Some(9999.0));
    assert!(hp_rule.enum_values.is_none());

    let rarity_rule = loaded.validation.get("rarity").unwrap();
    assert!(rarity_rule.min.is_none());
    assert!(rarity_rule.max.is_none());
    assert_eq!(
        rarity_rule.enum_values.as_ref().unwrap(),
        &vec!["common".to_string(), "rare".to_string(), "epic".to_string()]
    );

    assert_eq!(loaded.frozen_columns, Some(2));
}

#[test]
fn test_apply_cell_edits_batches_into_single_undo_snapshot() {
    let mut state = sample_state();
    let changed = state.apply_cell_edits(vec![
        CellEdit {
            row_index: 0,
            column: "age".to_string(),
            kind: CellEditKind::Value(Value::Number(40.into())),
        },
        CellEdit {
            row_index: 1,
            column: "age".to_string(),
            kind: CellEditKind::Value(Value::Number(35.into())),
        },
    ]);

    assert_eq!(changed, 2);
    assert_eq!(state.data()[0]["age"], Value::Number(40.into()));
    assert_eq!(state.data()[1]["age"], Value::Number(35.into()));

    assert!(state.undo());
    assert_eq!(state.data()[0]["age"], Value::Number(30.into()));
    assert_eq!(state.data()[1]["age"], Value::Number(25.into()));
}

#[test]
fn test_apply_cell_edits_value_replaces_existing_formula() {
    let mut state = sample_state();
    assert!(state.add_column("age2"));
    assert!(state.set_cell_formula(0, "age2", "=age * 2".to_string()));
    assert_eq!(state.cell_display_value(0, "age2"), "60");

    let changed = state.apply_cell_edits(vec![CellEdit {
        row_index: 0,
        column: "age2".to_string(),
        kind: CellEditKind::Value(Value::Number(999.into())),
    }]);
    assert_eq!(changed, 1);
    assert!(state.cell_formula(0, "age2").is_none());
    assert_eq!(state.cell_display_value(0, "age2"), "999");
}

#[test]
fn test_apply_cell_edits_formula_write() {
    let mut state = sample_state();
    assert!(state.add_column("score"));
    let changed = state.apply_cell_edits(vec![CellEdit {
        row_index: 1,
        column: "score".to_string(),
        kind: CellEditKind::Formula("=age + 10".to_string()),
    }]);

    assert_eq!(changed, 1);
    assert_eq!(state.cell_formula(1, "score").as_deref(), Some("age + 10"));
    assert_eq!(state.cell_display_value(1, "score"), "35");
}

#[test]
fn test_apply_cell_edits_respects_validation_rules() {
    let mut state = sample_state();
    state.set_validation_rule(
        "name",
        Some(ValidationRule {
            enum_values: Some(vec!["Alice".to_string(), "Bob".to_string()]),
            ..Default::default()
        }),
    );

    let changed = state.apply_cell_edits(vec![CellEdit {
        row_index: 0,
        column: "name".to_string(),
        kind: CellEditKind::Value(Value::String("Charlie".to_string())),
    }]);

    assert_eq!(changed, 0);
    assert_eq!(state.data()[0]["name"], Value::String("Alice".to_string()));
}

#[test]
fn test_formula_supports_bracket_identifiers_for_non_ascii_columns() {
    let mut state = TableState::from_data(vec![BTreeMap::from([
        ("總分".to_string(), Value::Number(40.into())),
        ("加成 值".to_string(), Value::Number(2.into())),
    ])]);
    assert!(state.add_column("結果"));
    assert!(state.set_cell_formula(0, "結果", "=[總分] + [加成 值]".to_string()));
    assert_eq!(state.cell_display_value(0, "結果"), "42");
}

#[test]
fn test_formula_rejects_unterminated_bracket_identifier() {
    let mut state = sample_state();
    assert!(state.add_column("score"));
    assert!(!state.set_cell_formula(0, "score", "=[age + 1".to_string()));
}
