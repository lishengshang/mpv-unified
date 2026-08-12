//! Upgrade simulation tests: user-layer preservation, corrupt-zip rejection
//! with rollback, no-update abort, and network-failure reporting.

use super::*;
use crate::fetch::testutil::{zip_bytes, TempDir};
use crate::fetch::MockFetcher;

const INDEX_URL: &str = "https://example.com/index.json";
const ZIP_URL: &str = "https://example.com/mpv-config-0.2.0.zip";

fn index_json(latest: &str, zip_url: &str) -> Vec<u8> {
    format!(
        r#"{{"latest_version":"{latest}","upgrade_zip_url":"{zip_url}","changelog_url":"https://example.com/releases","packages":[]}}"#
    )
    .into_bytes()
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().expect("fixture path has a parent")).expect("mkdir");
    fs::write(path, content).expect("write");
}

/// A simulated installed app: old app layer + a user layer with custom files.
fn fixture_root() -> TempDir {
    let root = TempDir::new("upgrade-root");
    write(&root.path().join("config/base.conf"), "base v1");
    write(&root.path().join("scripts/a.lua"), "script a v1");
    write(&root.path().join("VERSION"), "0.1.0");
    write(&root.path().join("user/user.conf"), "user-conf-secret");
    write(&root.path().join("user/custom.dat"), "custom-user-data");
    root
}

/// A valid new release zip: app layer only, no user/.
fn new_release_zip() -> Vec<u8> {
    zip_bytes(&[
        ("config/base.conf", b"base v2"),
        ("scripts/a.lua", b"script a v2"),
        ("scripts/b.lua", b"script b new"),
        ("VERSION", b"0.2.0"),
    ])
}

#[test]
fn check_update_reports_new_version() {
    let root = TempDir::new("check-root");
    write(&root.path().join("VERSION"), "0.1.0-dev");
    let fetcher = MockFetcher::new().with(INDEX_URL, index_json("0.2.0", ZIP_URL));

    let info = check_update(&fetcher, INDEX_URL, root.path());

    assert_eq!(info.current, "0.1.0-dev");
    assert_eq!(info.latest.as_deref(), Some("0.2.0"));
    assert!(info.has_update);
    assert_eq!(
        info.changelog_url.as_deref(),
        Some("https://example.com/releases")
    );
    assert_eq!(info.error, None);
}

#[test]
fn check_update_reports_no_update_information_without_latest_version() {
    let root = TempDir::new("check-root");
    write(&root.path().join("VERSION"), "0.1.0");
    let fetcher = MockFetcher::new().with(INDEX_URL, br#"{"packages":[]}"#.to_vec());

    let info = check_update(&fetcher, INDEX_URL, root.path());

    assert_eq!(info.latest, None);
    assert!(!info.has_update);
    let error = info.error.expect("no-metadata index is reported");
    assert!(error.contains("latest_version"), "{error}");
    assert!(error.contains("无更新信息"), "{error}");
}

#[test]
fn check_update_reports_readable_error_on_network_failure() {
    let root = TempDir::new("check-root");
    write(&root.path().join("VERSION"), "0.1.0");

    let info = check_update(&MockFetcher::new(), INDEX_URL, root.path());

    assert!(!info.has_update);
    let error = info.error.expect("network failure is reported");
    assert!(error.contains("索引"), "{error}");
    assert!(error.contains("404"), "{error}");
}

#[test]
fn check_update_reports_missing_version_file() {
    let root = TempDir::new("check-root");

    let info = check_update(&MockFetcher::new(), INDEX_URL, root.path());

    assert_eq!(info.current, "");
    let error = info.error.expect("missing VERSION is reported");
    assert!(error.contains("VERSION"), "{error}");
    assert!(!error.contains("0.1.0-dev"), "no phantom version");
}

#[test]
fn check_update_reports_no_update_when_current_is_latest() {
    let root = TempDir::new("check-root");
    write(&root.path().join("VERSION"), "0.1.0");
    let fetcher = MockFetcher::new().with(INDEX_URL, index_json("0.1.0", ZIP_URL));

    let info = check_update(&fetcher, INDEX_URL, root.path());

    assert!(!info.has_update);
    assert_eq!(info.latest.as_deref(), Some("0.1.0"));
    assert_eq!(info.error, None);
}

#[test]
fn upgrade_replaces_app_layer_and_preserves_user_layer() {
    let root = fixture_root();
    let cache = TempDir::new("upgrade-cache");
    let fetcher = MockFetcher::new()
        .with(INDEX_URL, index_json("0.2.0", ZIP_URL))
        .with(ZIP_URL, new_release_zip());

    let result = perform_upgrade(&fetcher, INDEX_URL, cache.path(), root.path());

    assert_eq!(result.applied_version.as_deref(), Some("0.2.0"));
    assert!(!result.rolled_back);
    assert_eq!(result.files_replaced, 4);
    assert_eq!(result.backup_dir.as_deref().map(|d| d.exists()), Some(true));
    assert!(result.message.contains("升级成功"), "{}", result.message);

    let app = root.path();
    assert_eq!(
        fs::read_to_string(app.join("config/base.conf")).expect("base"),
        "base v2"
    );
    assert_eq!(
        fs::read_to_string(app.join("scripts/a.lua")).expect("a"),
        "script a v2"
    );
    assert_eq!(
        fs::read_to_string(app.join("scripts/b.lua")).expect("b"),
        "script b new"
    );
    assert_eq!(
        fs::read_to_string(app.join("VERSION")).expect("version"),
        "0.2.0"
    );

    assert_eq!(
        fs::read_to_string(app.join("user/user.conf")).expect("user conf"),
        "user-conf-secret",
        "user 层 md5 必须不变"
    );
    assert_eq!(
        fs::read_to_string(app.join("user/custom.dat")).expect("custom"),
        "custom-user-data",
        "user 层自定义文件 md5 必须不变"
    );

    let backup = result.backup_dir.expect("backup recorded");
    assert_eq!(
        fs::read_to_string(backup.join("config/base.conf")).expect("backup base"),
        "base v1"
    );
    assert_eq!(
        fs::read_to_string(backup.join("VERSION")).expect("backup version"),
        "0.1.0"
    );
    assert!(cache.path().join("upgrade/mpv-config-0.2.0.zip").is_file());
}

#[test]
fn upgrade_rejects_corrupt_zip_and_rolls_back() {
    let root = fixture_root();
    let cache = TempDir::new("upgrade-cache");
    let fetcher = MockFetcher::new()
        .with(INDEX_URL, index_json("0.2.0", ZIP_URL))
        .with(ZIP_URL, b"this is not a zip archive".to_vec());

    let result = perform_upgrade(&fetcher, INDEX_URL, cache.path(), root.path());

    assert_eq!(result.applied_version, None);
    assert!(result.rolled_back, "corrupt zip must roll back");
    assert!(result.message.contains("解压"), "{}", result.message);
    assert!(result.message.contains("还原"), "{}", result.message);

    let app = root.path();
    assert_eq!(
        fs::read_to_string(app.join("config/base.conf")).expect("base"),
        "base v1"
    );
    assert_eq!(
        fs::read_to_string(app.join("scripts/a.lua")).expect("a"),
        "script a v1"
    );
    assert_eq!(
        fs::read_to_string(app.join("user/user.conf")).expect("user conf"),
        "user-conf-secret"
    );
}

#[test]
fn upgrade_aborts_before_touching_files_when_download_fails() {
    let root = fixture_root();
    let cache = TempDir::new("upgrade-cache");
    let fetcher = MockFetcher::new().with(INDEX_URL, index_json("0.2.0", ZIP_URL));

    let result = perform_upgrade(&fetcher, INDEX_URL, cache.path(), root.path());

    assert_eq!(result.applied_version, None);
    assert!(!result.rolled_back);
    assert!(result.message.contains("下载"), "{}", result.message);
    assert_eq!(result.backup_dir, None);
    assert_eq!(
        fs::read_to_string(root.path().join("config/base.conf")).expect("base"),
        "base v1"
    );
}

#[test]
fn upgrade_aborts_when_version_is_already_latest() {
    let root = fixture_root();
    let cache = TempDir::new("upgrade-cache");
    let fetcher = MockFetcher::new()
        .with(INDEX_URL, index_json("0.1.0", ZIP_URL))
        .with(ZIP_URL, new_release_zip());

    let result = perform_upgrade(&fetcher, INDEX_URL, cache.path(), root.path());

    assert_eq!(result.applied_version, None);
    assert!(result.message.contains("已是最新"), "{}", result.message);
    assert_eq!(result.backup_dir, None);
    assert_eq!(
        fs::read_to_string(root.path().join("config/base.conf")).expect("base"),
        "base v1"
    );
}

#[test]
fn upgrade_accepts_zip_with_single_wrapper_directory() {
    let root = fixture_root();
    let cache = TempDir::new("upgrade-cache");
    let wrapped = zip_bytes(&[
        ("mpv-config/config/base.conf", b"base v2"),
        ("mpv-config/VERSION", b"0.2.0"),
    ]);
    let fetcher = MockFetcher::new()
        .with(INDEX_URL, index_json("0.2.0", ZIP_URL))
        .with(ZIP_URL, wrapped);

    let result = perform_upgrade(&fetcher, INDEX_URL, cache.path(), root.path());

    assert_eq!(result.applied_version.as_deref(), Some("0.2.0"));
    assert_eq!(
        fs::read_to_string(root.path().join("config/base.conf")).expect("base"),
        "base v2"
    );
    assert_eq!(
        fs::read_to_string(root.path().join("user/user.conf")).expect("user conf"),
        "user-conf-secret"
    );
}
