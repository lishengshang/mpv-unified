//! Integration tests for the `gen` engine against fixture and real configs.

use cli::gen::{run_at_root, GenOptions};
use core::platform::Platform;
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
        let path = std::env::temp_dir().join(format!("mpv-config-cli-{label}-{id}"));
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

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gen-basic")
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/cli; the repository root is its parent.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("cli crate sits inside the workspace")
        .to_path_buf()
}

fn options(platform: Platform, out: &Path, dry_run: bool) -> GenOptions {
    GenOptions {
        platform: Some(platform),
        out: out.to_owned(),
        dry_run,
    }
}

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(path, content).expect("write fixture file");
}

fn minimal_root(label: &str, base: &str, platform: Option<&str>) -> TestDir {
    let root = TestDir::new(label);
    write(&root.path().join("config/base.conf"), base);
    if let Some(platform) = platform {
        write(&root.path().join("config/linux.conf"), platform);
    }
    root
}

#[test]
fn gen_merges_all_layers_and_appends_input_variant() {
    let out = TestDir::new("all-layers-out");

    let report = run_at_root(
        &fixture_root(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect("fixture generation succeeds");

    assert_eq!(report.files.len(), 2);
    assert_eq!(
        fs::read_to_string(out.path().join("dist/mpv.conf")).expect("read mpv output"),
        include_str!("../tests/fixtures/gen-basic/expected/mpv.conf")
    );
    assert_eq!(
        fs::read_to_string(out.path().join("dist/input.conf")).expect("read input output"),
        include_str!("../tests/fixtures/gen-basic/expected/input.conf")
    );
}

#[test]
fn dry_run_reports_outputs_without_creating_directory() {
    let out = TestDir::new("dry-run-parent");
    let output_path = out.path().join("not-created");

    let report = run_at_root(
        &fixture_root(),
        &options(Platform::Linux, &output_path, true),
    )
    .expect("dry-run succeeds");

    assert_eq!(report.files.len(), 2);
    assert!(!output_path.exists());
    assert!(report.files[0].line_count > 0);
}

#[test]
fn missing_base_file_is_a_clear_error() {
    let root = TestDir::new("missing-base");
    write(&root.path().join("config/linux.conf"), "vo=gpu\n");
    let out = TestDir::new("missing-base-out");

    let error = run_at_root(
        root.path(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect_err("missing base must fail");

    assert!(error.to_string().contains("base.conf"));
    assert!(error.to_string().contains("必须"));
}

#[test]
fn missing_user_file_is_skipped() {
    let root = minimal_root("missing-user", "volume=50\n", Some("volume=90\n"));
    let out = TestDir::new("missing-user-out");

    let report = run_at_root(
        root.path(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect("missing user is optional");

    assert!(report.warnings.is_empty());
    assert_eq!(
        fs::read_to_string(out.path().join("dist/mpv.conf")).expect("read mpv output"),
        "volume=90\n"
    );
}

#[test]
fn missing_macos_layer_is_warned_and_skipped() {
    let root = minimal_root("missing-macos", "vo=gpu\n", None);
    let out = TestDir::new("missing-macos-out");

    let report = run_at_root(
        root.path(),
        &options(Platform::MacOS, &out.path().join("dist"), false),
    )
    .expect("missing macOS layer is optional");

    assert_eq!(
        fs::read_to_string(out.path().join("dist/mpv.conf")).expect("read mpv output"),
        "vo=gpu\n"
    );
    assert_eq!(report.warnings.len(), 1);
    assert!(report.warnings[0].contains("macOS"));
}

#[test]
fn missing_platform_layer_for_linux_is_an_error() {
    let root = minimal_root("missing-linux-layer", "vo=gpu\n", None);
    let out = TestDir::new("missing-linux-layer-out");

    let error = run_at_root(
        root.path(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect_err("linux target without linux.conf must fail");

    let message = error.to_string();
    assert!(message.contains("linux.conf"), "{message}");
}

#[test]
fn unwritable_output_path_is_an_error() {
    let out = TestDir::new("unwritable-out");
    let output_path = out.path().join("output-file");
    write(&output_path, "not a directory");

    let error = run_at_root(
        &fixture_root(),
        &options(Platform::Linux, &output_path, false),
    )
    .expect_err("file output path must fail");

    assert!(error.to_string().contains("output-file"));
}

#[test]
fn real_config_generates_parseable_outputs() {
    let out = TestDir::new("real-out");

    let report = run_at_root(
        &repo_root(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect("real config generation succeeds");

    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert!(report.files.iter().any(|f| f.path.ends_with("mpv.conf")));
    assert!(report.files.iter().any(|f| f.path.ends_with("input.conf")));

    let mpv = fs::read_to_string(out.path().join("dist/mpv.conf")).expect("read mpv output");
    // The generated document must round-trip through the core parser.
    let doc = core::conf::parse(&mpv).expect("generated mpv.conf must parse");
    assert_eq!(core::conf::serialize(&doc), mpv);
    // Real content survived the pipeline.
    assert!(mpv.contains("gpu-api"), "platform layer content missing");
    assert!(mpv.contains("#"), "comments must be preserved");

    let input = fs::read_to_string(out.path().join("dist/input.conf")).expect("read input output");
    // input.conf keys like `[`/`]` are not mpv.conf syntax, so it cannot be
    // parsed by the core parser; assert its platform-branch invariants:
    // directive lines are gone and the linux else-branch survived while the
    // windows branch was pruned.
    assert!(
        !input
            .lines()
            .any(|line| line.trim_start().starts_with("#@")),
        "directive leaked into input.conf"
    );
    assert!(input.contains("MEMC_RIFE_STD.vpy"), "linux branch missing");
    assert!(
        !input.contains("MEMC_RIFE_DML.vpy"),
        "windows branch leaked"
    );
}
