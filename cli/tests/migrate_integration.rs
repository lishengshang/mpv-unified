//! Integration tests for `pkg migrate-manager` against fixture `manager.json`
//! files: a representative 5-entry fixture (github + branch, gist, blacklist,
//! duplicate repo name) and a corrupted one.

use cli::pkg_cmds::migrate::{self, MigrateError};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(label: &str) -> Self {
        let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("mpv-config-migrate-{label}-{id}"));
        fs::create_dir_all(&path).expect("create test directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn fixture(name: &str) -> PathBuf {
    Path::new(FIXTURES).join(name)
}

/// Parse a generated pending record back into a YAML mapping so field
/// mappings can be asserted on the *contract*, not on serialization details.
fn record_map(path: &Path) -> serde_yaml::Mapping {
    let text = fs::read_to_string(path).expect("read generated record");
    serde_yaml::from_str::<serde_yaml::Value>(&text)
        .expect("generated record parses as yaml")
        .as_mapping()
        .expect("record is a mapping")
        .clone()
}

/// Expected field mapping of one generated pending record.
struct Expected {
    name: &'static str,
    git: &'static str,
    branch: Option<&'static str>,
    whitelist: Option<&'static str>,
    blacklist: Option<&'static str>,
    dest: &'static str,
}

fn assert_record(out: &Path, expected: &Expected) {
    let path = out.join(format!("{}.yaml", expected.name));
    assert!(path.is_file(), "missing {}.yaml", expected.name);
    let map = record_map(&path);
    let source = map
        .get("source")
        .and_then(serde_yaml::Value::as_mapping)
        .expect("source mapping present");

    assert_eq!(
        map.get("name").and_then(serde_yaml::Value::as_str),
        Some(expected.name)
    );
    assert_eq!(
        map.get("version").and_then(serde_yaml::Value::as_str),
        Some("0.0.0"),
        "{}: git sources carry the 0.0.0 placeholder version",
        expected.name
    );
    assert_eq!(
        map.get("platform").and_then(serde_yaml::Value::as_str),
        Some("all"),
        "{}: platform defaults to all",
        expected.name
    );
    assert_eq!(
        map.get("migrated").and_then(serde_yaml::Value::as_str),
        Some("pending"),
        "{}: record is marked pending",
        expected.name
    );
    assert!(
        map.get("description")
            .and_then(serde_yaml::Value::as_str)
            .is_some_and(|d| !d.is_empty()),
        "{}: auto-generated description present",
        expected.name
    );

    assert_eq!(
        source.get("git").and_then(serde_yaml::Value::as_str),
        Some(expected.git)
    );
    let branch = source.get("branch").and_then(serde_yaml::Value::as_str);
    assert_eq!(
        branch, expected.branch,
        "{}: branch preserved",
        expected.name
    );
    let whitelist = source.get("whitelist").and_then(serde_yaml::Value::as_str);
    assert_eq!(
        whitelist, expected.whitelist,
        "{}: whitelist preserved",
        expected.name
    );
    let blacklist = source.get("blacklist").and_then(serde_yaml::Value::as_str);
    assert_eq!(
        blacklist, expected.blacklist,
        "{}: blacklist preserved",
        expected.name
    );
    assert_eq!(
        source.get("dest").and_then(serde_yaml::Value::as_str),
        Some(expected.dest),
        "{}: dest preserved verbatim",
        expected.name
    );
}

#[test]
fn migrates_all_fixture_entries_with_field_mapping() {
    let dir = TestDir::new("happy");
    let out = dir.path().join("pending");
    let report_path = dir.path().join("report.md");
    let report = migrate::run(&fixture("manager.json"), &out, &report_path)
        .expect("fixture migrates cleanly");
    assert_eq!(report.success_count(), 5, "all 5 entries migrate");
    assert_eq!(report.failure_count(), 0);
    assert_eq!(
        report.succeeded[0].name, "evafast",
        "entries keep source order"
    );

    assert_record(
        &out,
        &Expected {
            name: "evafast",
            git: "https://github.com/po5/evafast",
            branch: Some("rewrite"),
            whitelist: Some("%.lua$"),
            blacklist: None,
            dest: "~~/scripts",
        },
    );
    assert_record(
        &out,
        &Expected {
            name: "igv-a015fc88",
            git: "https://gist.github.com/igv/a015fc885d5c22e6891820ad89555637",
            branch: None,
            whitelist: Some("%.glsl$"),
            blacklist: None,
            dest: "~~/shaders/igv",
        },
    );
    assert_record(
        &out,
        &Expected {
            name: "anime4k",
            git: "https://github.com/bloc97/Anime4K",
            branch: None,
            whitelist: Some("%.md$|%.glsl$"),
            blacklist: Some("tensorflow"),
            dest: "~~/shaders/Anime4k",
        },
    );
    assert_record(
        &out,
        &Expected {
            name: "mpv-scripts",
            git: "https://github.com/dyphire/mpv-scripts",
            branch: Some("main"),
            whitelist: None,
            blacklist: Some("LICENSE|README%.md$"),
            dest: "~~/scripts",
        },
    );
    assert_record(
        &out,
        &Expected {
            name: "mpv-scripts-2",
            git: "https://github.com/Eisa01/mpv-scripts",
            branch: Some("master"),
            whitelist: Some("undoredo%.lua$"),
            blacklist: None,
            dest: "~~/scripts",
        },
    );
}

#[test]
fn duplicate_repo_name_gets_suffix_and_report_note() {
    let dir = TestDir::new("dedup");
    let out = dir.path().join("pending");
    let report = migrate::run(&fixture("manager.json"), &out, &dir.path().join("r.md"))
        .expect("fixture migrates");
    let note = report
        .notes
        .iter()
        .find(|n| n.contains("mpv-scripts-2"))
        .expect("rename note recorded");
    assert!(note.contains("Eisa01/mpv-scripts"), "{note}");
}

#[test]
fn corrupt_json_fails_without_writing_anything() {
    let dir = TestDir::new("corrupt");
    let out = dir.path().join("pending");
    let error = migrate::run(
        &fixture("manager-corrupt.json"),
        &out,
        &dir.path().join("r.md"),
    )
    .expect_err("corrupt json must fail");
    assert!(matches!(error, MigrateError::Json(_)), "{error:?}");
    assert!(
        !out.exists(),
        "no half-products: output directory must not be created on parse failure"
    );
    assert!(
        !dir.path().join("r.md").exists(),
        "no report written on parse failure"
    );
}

#[test]
fn missing_input_file_fails_with_io_error() {
    let dir = TestDir::new("missing");
    let error = migrate::run(
        &dir.path().join("nope.json"),
        &dir.path().join("pending"),
        &dir.path().join("r.md"),
    )
    .expect_err("missing input must fail");
    assert!(matches!(error, MigrateError::Io(_)), "{error:?}");
}

#[test]
fn report_file_records_counts_and_entries() {
    let dir = TestDir::new("report");
    let out = dir.path().join("pending");
    let report_path = dir.path().join("migration-report.md");
    migrate::run(&fixture("manager.json"), &out, &report_path).expect("fixture migrates");
    let markdown = fs::read_to_string(&report_path).expect("report file written");
    assert!(markdown.contains("成功 5 / 失败 0"), "{markdown}");
    assert!(
        markdown.contains("| 1 | evafast | https://github.com/po5/evafast | ~~/scripts |"),
        "{markdown}"
    );
    assert!(markdown.contains("| 3 | anime4k |"), "{markdown}");
}

#[test]
fn stdout_report_is_human_readable() {
    let dir = TestDir::new("stdout");
    let out = dir.path().join("pending");
    let report = migrate::run(&fixture("manager.json"), &out, &dir.path().join("r.md"))
        .expect("fixture migrates");
    let text = report.to_string();
    assert!(text.contains("成功 5 / 失败 0"), "{text}");
    assert!(text.contains("[ok] igv-a015fc88"), "{text}");
}
