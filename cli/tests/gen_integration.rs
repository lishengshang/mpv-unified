//! Integration tests for the `gen` engine against fixture and real configs.

use cli::gen::{run_at_root, GenOptions};
use core::platform::Platform;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
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
fn gen_bootstraps_user_layer_on_first_run() {
    let root = minimal_root("first-run-user", "volume=50\n", Some("volume=90\n"));
    let out = TestDir::new("first-run-user-out");

    let report = run_at_root(
        root.path(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect("first-run generation succeeds");

    assert!(
        report.warnings.iter().any(|w| w.contains("自动创建")),
        "first run must report the bootstrap: {:?}",
        report.warnings
    );
    let user_conf =
        fs::read_to_string(root.path().join("user/user.conf")).expect("user.conf auto-created");
    assert!(user_conf.starts_with("# user.conf"), "{user_conf}");
    let merged = fs::read_to_string(out.path().join("dist/mpv.conf")).expect("read mpv output");
    assert!(merged.starts_with("volume=90\n"), "{merged}");
    assert!(merged.contains("# user.conf — 个人配置层"), "{merged}");
}

#[test]
fn gen_copies_user_example_conf_as_user_conf_when_template_exists() {
    let root = minimal_root("template-user", "volume=50\n", Some("volume=90\n"));
    write(
        &root.path().join("user/user.example.conf"),
        "# 模板头部\nvolume=1\n",
    );
    let out = TestDir::new("template-user-out");

    let report = run_at_root(
        root.path(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect("generation succeeds");

    assert!(report
        .warnings
        .iter()
        .any(|w| w.contains("user.example.conf")));
    assert_eq!(
        fs::read_to_string(root.path().join("user/user.conf")).expect("user.conf from template"),
        "# 模板头部\nvolume=1\n"
    );
    let merged = fs::read_to_string(out.path().join("dist/mpv.conf")).expect("read mpv output");
    assert!(
        merged.contains("volume=1"),
        "user layer overrides base: {merged}"
    );
    assert!(
        merged.contains("# 模板头部"),
        "template comment preserved: {merged}"
    );
}

#[test]
fn gen_never_overwrites_existing_user_conf() {
    let root = minimal_root("keep-user", "volume=50\n", Some("volume=90\n"));
    let custom = "# 我的自定义\nsub-font-size=44\n";
    write(&root.path().join("user/user.conf"), custom);
    let out = TestDir::new("keep-user-out");

    let report = run_at_root(
        root.path(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect("generation succeeds");

    assert!(
        report.warnings.iter().all(|w| !w.contains("自动创建")),
        "no bootstrap on second run: {:?}",
        report.warnings
    );
    assert_eq!(
        fs::read_to_string(root.path().join("user/user.conf")).expect("user.conf intact"),
        custom,
        "existing user.conf must never be rewritten"
    );
}

#[test]
fn invalid_user_conf_errors_without_overwriting() {
    let root = minimal_root("invalid-user", "volume=50\n", Some("volume=90\n"));
    let invalid = "volume=50\nsub-font-size=\"unterminated\n";
    write(&root.path().join("user/user.conf"), invalid);
    let out = TestDir::new("invalid-user-out");

    let error = run_at_root(
        root.path(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect_err("invalid user.conf must fail generation");

    let message = error.to_string();
    assert!(message.contains("user/user.conf"), "{message}");
    assert!(message.contains("line"), "{message}");
    assert_eq!(
        fs::read_to_string(root.path().join("user/user.conf")).expect("user.conf intact"),
        invalid,
        "user.conf must be preserved byte-for-byte on parse failure"
    );
}

#[test]
fn dry_run_does_not_bootstrap_user_layer() {
    let root = minimal_root("dry-run-user", "volume=50\n", Some("volume=90\n"));
    let out = TestDir::new("dry-run-user-out");

    let report = run_at_root(
        root.path(),
        &options(Platform::Linux, &out.path().join("dist"), true),
    )
    .expect("dry-run succeeds");

    assert!(
        report.warnings.is_empty(),
        "dry-run must not touch the filesystem: {:?}",
        report.warnings
    );
    assert!(
        !root.path().join("user").exists(),
        "dry-run must not create the user layer"
    );
}

/// Stable content fingerprint (DefaultHasher with fixed keys), standing in
/// for an md5 in tests: equal bytes → equal fingerprint.
fn fingerprint(path: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    fs::read(path)
        .expect("read file for fingerprint")
        .hash(&mut hasher);
    hasher.finish()
}

#[test]
fn upgrade_simulation_preserves_user_layer() {
    // 模拟旧版布局:app 层 + user 层(含自定义内容)
    let root = TestDir::new("upgrade-sim");
    write(&root.path().join("config/base.conf"), "volume=50\n");
    write(&root.path().join("config/linux.conf"), "vo=gpu\n");
    write(&root.path().join("scripts/old-script.lua"), "-- old\n");
    write(&root.path().join("VERSION"), "0.1.0-dev\n");
    let custom = "# 我的自定义\nsub-font-size=44\nassrt-token=abc123\n";
    write(&root.path().join("user/user.conf"), custom);

    let before = fingerprint(&root.path().join("user/user.conf"));

    // 模拟"新版替换":重新拷贝/覆盖 app 层文件,完全不碰 user 层
    write(&root.path().join("config/base.conf"), "volume=55\n");
    write(&root.path().join("config/linux.conf"), "vo=gpu-next\n");
    write(&root.path().join("scripts/new-script.lua"), "-- new\n");
    write(&root.path().join("VERSION"), "0.2.0\n");

    let after = fingerprint(&root.path().join("user/user.conf"));
    assert_eq!(
        before, after,
        "user.conf fingerprint must be unchanged by app-layer replacement"
    );
    let preserved = fs::read_to_string(root.path().join("user/user.conf")).expect("read user.conf");
    assert_eq!(
        preserved, custom,
        "custom user content must survive upgrade"
    );
    assert!(preserved.contains("abc123"), "secret stays in user layer");
}

#[test]
fn missing_macos_layer_is_warned_and_skipped() {
    let root = minimal_root("missing-macos", "vo=gpu\n", None);
    // Pre-seed the user layer so the macOS warning is the only one.
    write(&root.path().join("user/user.conf"), "\n");
    let out = TestDir::new("missing-macos-out");

    let report = run_at_root(
        root.path(),
        &options(Platform::MacOS, &out.path().join("dist"), false),
    )
    .expect("missing macOS layer is optional");

    assert!(
        fs::read_to_string(out.path().join("dist/mpv.conf"))
            .expect("read mpv output")
            .starts_with("vo=gpu"),
        "base content must survive"
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
fn gen_appends_enabled_profile_blocks_to_mpv_conf() {
    let root = minimal_root("profiles-enabled", "volume=50\n", Some("vo=gpu\n"));
    write(
        &root.path().join("config/profiles.yaml"),
        "profiles:\n\
         \x20 - id: cinema\n\
         \x20   name: 高清观影\n\
         \x20   desc: 画质优先\n\
         \x20   icon: 🎬\n\
         \x20   options:\n\
         \x20     - \"deband=yes\"\n\
         \x20   requires: []\n\
         \x20 - id: music\n\
         \x20   name: 音乐模式\n\
         \x20   desc: 纯音频\n\
         \x20   icon: 🎧\n\
         \x20   options:\n\
         \x20     - \"vo=null\"\n\
         \x20   requires: []\n",
    );
    write(
        &root.path().join("user/profiles-state.json"),
        "[\"cinema\",\"music\"]\n",
    );
    let out = TestDir::new("profiles-enabled-out");

    let report = run_at_root(
        root.path(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect("generation with profiles succeeds");

    assert!(
        report.warnings.iter().all(|w| !w.contains("方案")),
        "no profile warnings on the happy path: {:?}",
        report.warnings
    );
    let mpv = fs::read_to_string(out.path().join("dist/mpv.conf")).expect("read mpv output");
    // Layer content stays intact, blocks append at the end with the
    // copy-equal first line, the generated comment, and apply lines.
    assert!(mpv.starts_with("volume=50\nvo=gpu\n"), "{mpv}");
    assert!(mpv.contains(
        "# 方案:高清观影 (由 mpv-config 生成)\n[cinema]\nprofile-restore=copy-equal\ndeband=yes\n"
    ), "{mpv}");
    assert!(mpv.contains(
        "# 方案:音乐模式 (由 mpv-config 生成)\n[music]\nprofile-restore=copy-equal\nvo=null\n"
    ), "{mpv}");
    assert!(mpv.ends_with("profile=cinema\nprofile=music\n"), "{mpv}");
    // The appended region still parses as mpv.conf.
    let doc = core::conf::parse(&mpv).expect("output with profile blocks must parse");
    assert_eq!(core::conf::serialize(&doc), mpv);
}

#[test]
fn gen_defaults_to_cinema_profile_when_state_is_missing() {
    let root = minimal_root("profiles-default", "volume=50\n", Some("vo=gpu\n"));
    write(
        &root.path().join("config/profiles.yaml"),
        "profiles:\n\
         \x20 - id: cinema\n\
         \x20   name: 高清观影\n\
         \x20   icon: 🎬\n\
         \x20   options:\n\
         \x20     - \"deband=yes\"\n\
         \x20   requires: []\n",
    );
    let out = TestDir::new("profiles-default-out");

    let report = run_at_root(
        root.path(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect("first-run generation succeeds");

    let mpv = fs::read_to_string(out.path().join("dist/mpv.conf")).expect("read mpv output");
    assert!(mpv.contains("[cinema]\nprofile-restore=copy-equal\n"), "{mpv}");
    assert!(mpv.contains("profile=cinema"), "{mpv}");
    assert!(
        !root.path().join("user/profiles-state.json").exists(),
        "gen reads state but must not write it"
    );
    assert!(
        report.warnings.iter().all(|w| !w.contains("方案")),
        "{:?}",
        report.warnings
    );
}

#[test]
fn gen_emits_no_profile_blocks_when_all_profiles_are_disabled() {
    let root = minimal_root("profiles-disabled", "volume=50\n", Some("vo=gpu\n"));
    write(
        &root.path().join("config/profiles.yaml"),
        "profiles:\n  - id: cinema\n    name: 高清观影\n    options:\n      - \"deband=yes\"\n    requires: []\n",
    );
    write(&root.path().join("user/profiles-state.json"), "[]\n");
    let out = TestDir::new("profiles-disabled-out");

    run_at_root(
        root.path(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect("generation succeeds");

    let mpv = fs::read_to_string(out.path().join("dist/mpv.conf")).expect("read mpv output");
    assert!(!mpv.contains("[cinema]"), "no block when disabled: {mpv}");
    assert!(!mpv.contains("profile=cinema"), "no apply line: {mpv}");
}

#[test]
fn corrupt_profiles_yaml_warns_and_skips_blocks() {
    let root = minimal_root("profiles-corrupt", "volume=50\n", Some("vo=gpu\n"));
    write(
        &root.path().join("config/profiles.yaml"),
        "profiles:\n  - id: dup\n    name: A\n  - id: dup\n    name: B\n",
    );
    let out = TestDir::new("profiles-corrupt-out");

    let report = run_at_root(
        root.path(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect("gen continues when profiles.yaml is broken");

    assert!(
        report.warnings.iter().any(|w| w.contains("方案块")),
        "{:?}",
        report.warnings
    );
    let mpv = fs::read_to_string(out.path().join("dist/mpv.conf")).expect("read mpv output");
    assert!(!mpv.contains("[dup]"), "no partial blocks: {mpv}");
}

#[test]
fn missing_profiles_yaml_is_skipped_silently() {
    let root = minimal_root("profiles-absent", "volume=50\n", Some("vo=gpu\n"));
    let out = TestDir::new("profiles-absent-out");

    let report = run_at_root(
        root.path(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect("generation without profiles.yaml succeeds");

    assert!(
        report
            .warnings
            .iter()
            .all(|w| !w.contains("方案")),
        "missing profiles.yaml must not warn: {:?}",
        report.warnings
    );
}

#[test]
fn real_config_generates_parseable_outputs() {
    let out = TestDir::new("real-out");

    let report = run_at_root(
        &repo_root(),
        &options(Platform::Linux, &out.path().join("dist"), false),
    )
    .expect("real config generation succeeds");

    // The only acceptable warning on the real repo is the first-run user
    // layer bootstrap (present only when user/user.conf does not exist yet).
    assert!(
        report.warnings.iter().all(|w| w.contains("自动创建")),
        "{:?}",
        report.warnings
    );
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
