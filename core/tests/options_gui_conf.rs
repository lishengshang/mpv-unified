//! Integration tests for `core::options_gui`: the `user/gui.conf` fragment
//! read/write round-trip, plus the shipped curated table (evidence demo).

use core::options_gui::{self, OptionsTable};
use std::fs;
use std::path::PathBuf;

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
";

fn valid_table() -> OptionsTable {
    options_gui::parse_yaml(VALID).expect("fixture parses")
}

fn user_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mpv-config-options-{label}"));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn missing_gui_conf_reads_as_empty() {
    let dir = user_dir("missing");
    assert_eq!(
        options_gui::read_gui_conf(&dir).expect("read succeeds"),
        Vec::<(String, String)>::new()
    );
    assert!(!dir.exists(), "read must not create the directory");
}

#[test]
fn corrupt_gui_conf_is_an_error() {
    let dir = user_dir("corrupt");
    fs::create_dir_all(&dir).expect("create dir");
    fs::write(
        dir.join(options_gui::GUI_CONF_FILE),
        "sub-font-size=\"unterminated\n",
    )
    .expect("write corrupt fragment");
    let error = options_gui::read_gui_conf(&dir).expect_err("corrupt fragment must error");
    assert!(error.to_string().contains("line"), "{error}");
}

#[test]
fn read_gui_conf_returns_pairs_and_skips_comments_and_profiles() {
    let dir = user_dir("read");
    fs::create_dir_all(&dir).expect("create dir");
    fs::write(
        dir.join(options_gui::GUI_CONF_FILE),
        "# 由 mpv-config GUI 管理\n\nsub-font-size=44\n\n[x]\nscale=ewa_lanczossharp\n",
    )
    .expect("write fragment");
    assert_eq!(
        options_gui::read_gui_conf(&dir).expect("read succeeds"),
        vec![("sub-font-size".to_owned(), "44".to_owned())]
    );
}

#[test]
fn write_only_persists_non_default_values_with_header() {
    let table = valid_table();
    let dir = user_dir("write-non-default");
    let values = vec![
        ("sub-font-size".to_owned(), "55".to_owned()), // default → skipped
        ("sub-font-size".to_owned(), "44".to_owned()),  // later occurrence wins
        ("deband".to_owned(), "no".to_owned()),         // default → skipped
        ("hwdec".to_owned(), "no".to_owned()),          // non-default → written
    ];
    options_gui::write_gui_conf(&table, &dir, &values).expect("write succeeds");
    let text = fs::read_to_string(dir.join(options_gui::GUI_CONF_FILE)).expect("read fragment");
    assert!(text.starts_with("# 由 mpv-config GUI 管理\n\n"), "{text}");
    assert!(text.contains("sub-font-size=44"), "{text}");
    assert!(text.contains("hwdec=no"), "{text}");
    assert!(!text.contains("deband="), "default switch not written: {text}");
    let roundtrip = core::conf::parse(&text).expect("fragment parses");
    assert_eq!(core::conf::serialize(&roundtrip), text);
    let entries: Vec<String> = fs::read_dir(&dir)
        .expect("list dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!entries.iter().any(|n| n.contains(".tmp")), "atomic: {entries:?}");
}

#[test]
fn write_preserves_comments_across_rewrites() {
    let table = valid_table();
    let dir = user_dir("roundtrip");
    options_gui::write_gui_conf(&table, &dir, &[("sub-font-size".to_owned(), "44".to_owned())])
        .expect("first write");
    fs::write(
        dir.join(options_gui::GUI_CONF_FILE),
        "# 由 mpv-config GUI 管理\n\n# 我的注释\nsub-font-size=44 # 行内注释\n",
    )
    .expect("seed comments");
    options_gui::write_gui_conf(&table, &dir, &[("sub-font-size".to_owned(), "48".to_owned())])
        .expect("second write");
    let text = fs::read_to_string(dir.join(options_gui::GUI_CONF_FILE)).expect("read fragment");
    assert!(text.contains("# 我的注释"), "comments preserved: {text}");
    assert!(text.contains("sub-font-size=48 # 行内注释"), "{text}");
    assert_eq!(
        options_gui::read_gui_conf(&dir).expect("read"),
        vec![("sub-font-size".to_owned(), "48".to_owned())]
    );
}

#[test]
fn write_aborts_on_invalid_value_without_touching_the_file() {
    let table = valid_table();
    let dir = user_dir("abort");
    fs::create_dir_all(&dir).expect("create dir");
    let before = "# 已有内容\nsub-font-size=44\n";
    fs::write(dir.join(options_gui::GUI_CONF_FILE), before).expect("seed fragment");
    let error = options_gui::write_gui_conf(
        &table,
        &dir,
        &[("sub-font-size".to_owned(), "999".to_owned())],
    )
    .expect_err("out-of-range value must abort");
    assert!(error.to_string().contains("范围"), "{error}");
    assert_eq!(
        fs::read_to_string(dir.join(options_gui::GUI_CONF_FILE)).expect("file intact"),
        before,
        "nothing may be written on validation failure"
    );
}

#[test]
fn write_removes_reset_keys_and_preserves_unknown_manual_keys() {
    let table = valid_table();
    let dir = user_dir("reset");
    fs::create_dir_all(&dir).expect("create dir");
    fs::write(
        dir.join(options_gui::GUI_CONF_FILE),
        "# 由 mpv-config GUI 管理\n\nsub-font-size=44\nmanual-key=keep-me\n",
    )
    .expect("seed fragment");
    // sub-font-size reset to its default → dropped; manual key kept.
    options_gui::write_gui_conf(&table, &dir, &[("sub-font-size".to_owned(), "55".to_owned())])
        .expect("write succeeds");
    let text = fs::read_to_string(dir.join(options_gui::GUI_CONF_FILE)).expect("read fragment");
    assert!(!text.contains("sub-font-size"), "{text}");
    assert!(text.contains("manual-key=keep-me"), "{text}");
}

#[test]
fn is_default_compares_numbers_numerically() {
    let table = valid_table();
    let option = table.find("sub-font-size").expect("option exists");
    assert!(table.is_default(option, "55"));
    assert!(table.is_default(option, "55.0"), "numeric equality");
    assert!(!table.is_default(option, "44"));
    let option = table.find("deband").expect("option exists");
    assert!(table.is_default(option, "  no  "), "trimmed comparison");
    assert!(!table.is_default(option, "yes"));
}

#[test]
fn write_ignores_unknown_keys_in_the_values_argument() {
    let table = valid_table();
    let dir = user_dir("unknown-value");
    options_gui::write_gui_conf(
        &table,
        &dir,
        &[
            ("ghost-option".to_owned(), "42".to_owned()),
            ("deband".to_owned(), "no".to_owned()),
        ],
    )
    .expect("unknown values are skipped, known defaults are too");
    let text = fs::read_to_string(dir.join(options_gui::GUI_CONF_FILE)).expect("read fragment");
    assert!(!text.contains("ghost-option"), "{text}");
    assert!(!text.contains("deband="), "{text}");
}

// ------------------------------------------------- real curated table

/// Evidence demo (task 19): the shipped `config/options-gui.yaml` loads,
/// `sub-font-size=44` validates, and saving writes exactly one non-default
/// line into `user/gui.conf` with the managed header comment.
#[test]
fn real_table_save_sub_font_size_44_writes_single_line() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("core crate sits inside the workspace")
        .to_path_buf();
    let table =
        OptionsTable::load(&root.join("config/options-gui.yaml")).expect("real table parses");
    assert!(
        table.options.len() >= 50,
        "curated table must hold >=50 options, got {}",
        table.options.len()
    );
    let categories: Vec<&str> = options_gui::CATEGORIES
        .iter()
        .copied()
        .filter(|category| table.options.iter().any(|o| o.category == *category))
        .collect();
    assert!(
        categories.len() >= 6,
        "curated table must cover >=6 chapters, got {categories:?}"
    );

    let dir = user_dir("real-save");
    let normalized = options_gui::validate_value(&table, "sub-font-size", "44")
        .expect("in-range value validates");
    assert_eq!(normalized, "44");
    options_gui::write_gui_conf(
        &table,
        &dir,
        &[
            ("sub-font-size".to_owned(), "44".to_owned()),
            // a couple of untouched defaults must NOT be written
            ("volume".to_owned(), "100".to_owned()),
            ("deband".to_owned(), "no".to_owned()),
        ],
    )
    .expect("save succeeds");

    let text = fs::read_to_string(dir.join(options_gui::GUI_CONF_FILE)).expect("read gui.conf");
    assert!(text.starts_with("# 由 mpv-config GUI 管理\n\n"), "{text}");
    assert!(text.contains("sub-font-size=44"), "{text}");
    assert!(!text.contains("volume="), "default volume must not be written: {text}");
    assert!(!text.contains("deband="), "default deband must not be written: {text}");
    assert_eq!(text.lines().count(), 3, "header + blank + one option: {text}");
    // The fragment survives the lossless parser round-trip and reads back.
    assert_eq!(
        options_gui::read_gui_conf(&dir).expect("read back"),
        vec![("sub-font-size".to_owned(), "44".to_owned())]
    );
    eprintln!("--- user/gui.conf (real table, sub-font-size=44) ---\n{text}");
}

#[test]
fn real_table_rejects_out_of_range_values() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("core crate sits inside the workspace")
        .to_path_buf();
    let table =
        OptionsTable::load(&root.join("config/options-gui.yaml")).expect("real table parses");
    let error = options_gui::validate_value(&table, "sub-font-size", "200")
        .expect_err("out of range must fail");
    assert!(error.contains("范围"), "{error}");
    let error = options_gui::validate_value(&table, "hwdec", "magic")
        .expect_err("unknown choice must fail");
    assert!(error.contains("可选值"), "{error}");
    let dir = user_dir("real-reject");
    options_gui::write_gui_conf(
        &table,
        &dir,
        &[("sub-font-size".to_owned(), "200".to_owned())],
    )
    .expect_err("write must abort on out-of-range value");
    assert!(
        !dir.join(options_gui::GUI_CONF_FILE).exists(),
        "no file may be created by an aborted write"
    );
}
