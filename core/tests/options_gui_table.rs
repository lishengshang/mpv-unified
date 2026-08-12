//! Integration tests for `core::options_gui`: table parsing/validation and
//! per-value validation.

use core::options_gui::{self, OptionType, OptionsTable};

const VALID: &str = "\
options:
  - key: sub-font-size
    category: subtitle
    type: number
    min: 1
    max: 100
    default: 55
    desc_zh: 字幕字号
    desc_en: Subtitle font size
    doc_url: https://mpv.io/manual/master/#options-sub-font-size
  - key: hwdec
    category: video
    type: select
    choices: [auto, no]
    default: auto
    desc_zh: 硬解
    desc_en: HW decode
    doc_url: https://mpv.io/manual/master/#options-hwdec
  - key: deband
    category: video
    type: switch
    default: no
    desc_zh: 去色带
    desc_en: Deband
    doc_url: https://mpv.io/manual/master/#options-deband
  - key: log-file
    category: other
    type: path
    default: \"~~/files/mpv.log\"
    desc_zh: 日志
    desc_en: Log file
    doc_url: https://mpv.io/manual/master/#options-log-file
  - key: osc
    category: general
    type: select
    choices: [auto, yes, no]
    default: \"yes\"
    desc_zh: OSC
    desc_en: OSC
    doc_url: https://mpv.io/manual/master/#options-osc
";

fn valid_table() -> OptionsTable {
    options_gui::parse_yaml(VALID).expect("fixture parses")
}

fn table_with(item: &str) -> Result<OptionsTable, options_gui::OptionsGuiError> {
    options_gui::parse_yaml(&format!("options:\n{item}"))
}

#[test]
fn parses_valid_yaml_with_field_defaults() {
    let table = valid_table();
    assert_eq!(table.options.len(), 5);
    let number = &table.options[0];
    assert_eq!(number.key, "sub-font-size");
    assert_eq!(number.category, "subtitle");
    assert_eq!(number.type_, OptionType::Number);
    assert_eq!(number.min, Some(1.0));
    assert_eq!(number.max, Some(100.0));
    assert_eq!(number.default, "55");
    assert_eq!(number.desc_zh, "字幕字号");
    assert_eq!(number.desc_en, "Subtitle font size");
    assert!(number.doc_url.starts_with("https://mpv.io"));
    assert!(number.choices.is_empty(), "number has no choices");
    let hwdec = &table.options[1];
    assert_eq!(hwdec.choices, ["auto", "no"]);
    assert_eq!(table.options[2].type_, OptionType::Switch);
    assert_eq!(table.options[3].type_, OptionType::Path);
}

#[test]
fn duplicate_keys_are_rejected() {
    let error = options_gui::parse_yaml(
        "options:\n\
         \x20 - key: dup\n    category: general\n    type: switch\n    default: no\n    desc_zh: a\n    desc_en: b\n    doc_url: c\n\
         \x20 - key: dup\n    category: general\n    type: switch\n    default: no\n    desc_zh: a\n    desc_en: b\n    doc_url: c\n",
    )
    .expect_err("duplicate key rejected");
    assert!(error.to_string().contains("重复"), "{error}");
}

#[test]
fn unknown_type_is_rejected() {
    let error = table_with(
        "  - key: a\n    category: general\n    type: color\n    default: no\n    desc_zh: a\n    desc_en: b\n    doc_url: c\n",
    )
    .expect_err("unknown type rejected");
    assert!(error.to_string().contains("type"), "{error}");
}

#[test]
fn number_without_min_or_max_is_rejected() {
    let error = table_with(
        "  - key: a\n    category: general\n    type: number\n    max: 10\n    default: 5\n    desc_zh: a\n    desc_en: b\n    doc_url: c\n",
    )
    .expect_err("missing min rejected");
    assert!(error.to_string().contains("min"), "{error}");
    let error = table_with(
        "  - key: a\n    category: general\n    type: number\n    min: 0\n    default: 5\n    desc_zh: a\n    desc_en: b\n    doc_url: c\n",
    )
    .expect_err("missing max rejected");
    assert!(error.to_string().contains("max"), "{error}");
}

#[test]
fn number_default_out_of_range_is_rejected() {
    let error = table_with(
        "  - key: a\n    category: general\n    type: number\n    min: 1\n    max: 10\n    default: 42\n    desc_zh: a\n    desc_en: b\n    doc_url: c\n",
    )
    .expect_err("out-of-range default rejected");
    assert!(error.to_string().contains("范围"), "{error}");
}

#[test]
fn number_default_of_wrong_type_is_rejected() {
    let error = table_with(
        "  - key: a\n    category: general\n    type: number\n    min: 1\n    max: 10\n    default: big\n    desc_zh: a\n    desc_en: b\n    doc_url: c\n",
    )
    .expect_err("non-numeric default rejected");
    assert!(error.to_string().contains("不是数字"), "{error}");
}

#[test]
fn select_without_choices_or_bad_default_is_rejected() {
    let error = table_with(
        "  - key: a\n    category: general\n    type: select\n    default: x\n    desc_zh: a\n    desc_en: b\n    doc_url: c\n",
    )
    .expect_err("missing choices rejected");
    assert!(error.to_string().contains("choices"), "{error}");
    let error = table_with(
        "  - key: a\n    category: general\n    type: select\n    choices: [x, y]\n    default: z\n    desc_zh: a\n    desc_en: b\n    doc_url: c\n",
    )
    .expect_err("default outside choices rejected");
    assert!(error.to_string().contains("不在 choices"), "{error}");
}

#[test]
fn switch_default_must_be_yes_or_no() {
    let error = table_with(
        "  - key: a\n    category: general\n    type: switch\n    default: maybe\n    desc_zh: a\n    desc_en: b\n    doc_url: c\n",
    )
    .expect_err("non-boolean switch default rejected");
    assert!(error.to_string().contains("yes 或 no"), "{error}");
    // YAML 1.1 boolean spellings normalize to yes/no.
    let table = table_with(
        "  - key: a\n    category: general\n    type: switch\n    default: true\n    desc_zh: a\n    desc_en: b\n    doc_url: c\n",
    )
    .expect("parses");
    assert_eq!(table.options[0].default, "yes");
}

#[test]
fn unknown_category_and_invalid_key_are_rejected() {
    let error = table_with(
        "  - key: a\n    category: color\n    type: switch\n    default: no\n    desc_zh: a\n    desc_en: b\n    doc_url: c\n",
    )
    .expect_err("unknown category rejected");
    assert!(error.to_string().contains("category"), "{error}");
    let error = table_with(
        "  - key: \"bad key\"\n    category: general\n    type: switch\n    default: no\n    desc_zh: a\n    desc_en: b\n    doc_url: c\n",
    )
    .expect_err("whitespace key rejected");
    assert!(error.to_string().contains("key"), "{error}");
}

// ----------------------------------------------------- value validation

#[test]
fn validate_value_checks_number_range() {
    let table = valid_table();
    assert_eq!(
        options_gui::validate_value(&table, "sub-font-size", "44").expect("in range"),
        "44"
    );
    let error = options_gui::validate_value(&table, "sub-font-size", "200")
        .expect_err("out of range rejected");
    assert!(error.contains("范围"), "{error}");
    let error = options_gui::validate_value(&table, "sub-font-size", "huge")
        .expect_err("non-number rejected");
    assert!(error.contains("数字"), "{error}");
}

#[test]
fn validate_value_checks_select_switch_and_unknown_key() {
    let table = valid_table();
    assert_eq!(
        options_gui::validate_value(&table, "hwdec", "no").expect("choice"),
        "no"
    );
    let error = options_gui::validate_value(&table, "hwdec", "magic")
        .expect_err("unknown choice rejected");
    assert!(error.contains("可选值"), "{error}");
    assert_eq!(
        options_gui::validate_value(&table, "deband", "yes").expect("switch"),
        "yes"
    );
    let error = options_gui::validate_value(&table, "deband", "on")
        .expect_err("non-switch value rejected");
    assert!(error.contains("yes 或 no"), "{error}");
    let error = options_gui::validate_value(&table, "ghost", "1")
        .expect_err("unknown key rejected");
    assert!(error.contains("未知选项"), "{error}");
}

#[test]
fn validate_value_rejects_unbalanced_quotes_and_comment_marker() {
    let table = valid_table();
    let error = options_gui::validate_value(&table, "log-file", "~~/files/a #b")
        .expect_err("inline comment marker rejected");
    assert!(error.contains("注释"), "{error}");
    let error = options_gui::validate_value(&table, "log-file", "\"unterminated")
        .expect_err("unbalanced quote rejected");
    assert!(error.contains("引号"), "{error}");
}
