//! Integration tests for `pkg update`: version changes, stale-file
//! removal, no-op on same version, and rollback on failure.

mod common;

use cli::pkg_cmds::lifecycle::lock;
use cli::pkg_cmds::lifecycle::{self, InstallOptions, UpdateOptions};
use common::lifecycle::{manifest, read_string, write, TestDir};
use core::platform::Platform;

fn install_opts(root: &std::path::Path, cache: &std::path::Path, name: &str) -> InstallOptions {
    InstallOptions {
        name: name.to_owned(),
        repo_root: root.to_path_buf(),
        cache_dir: cache.to_path_buf(),
        platform: Platform::Linux,
    }
}

fn install_local(root: &std::path::Path, cache: &std::path::Path, name: &str) {
    lifecycle::install(&install_opts(root, cache, name)).expect("install succeeds");
}

#[test]
fn update_changes_version_removes_stale_files() {
    let dir = TestDir::new("update-version");
    let root = dir.path();
    let cache = root.join("cache");
    write(
        &root.join("packages/alpha.yaml"),
        &manifest(
            "alpha",
            "1.0.0",
            &[
                ("alpha.lua", "~~/scripts/alpha.lua"),
                ("old.lua", "~~/scripts/old.lua"),
            ],
            &["osd-font-size=30"],
            &[],
            &[],
            None,
        ),
    );
    write(&root.join("packages/alpha.lua"), "v1");
    write(&root.join("packages/old.lua"), "old");
    install_local(root, &cache, "alpha");

    write(
        &root.join("packages/alpha.yaml"),
        &manifest(
            "alpha",
            "1.1.0",
            &[
                ("alpha.lua", "~~/scripts/alpha.lua"),
                ("extra.lua", "~~/scripts/extra.lua"),
            ],
            &["osd-font-size=44"],
            &[],
            &[],
            None,
        ),
    );
    write(&root.join("packages/alpha.lua"), "v1.1");
    write(&root.join("packages/extra.lua"), "new");

    let report = lifecycle::update(&UpdateOptions {
        name: Some("alpha".to_owned()),
        repo_root: root.to_path_buf(),
        cache_dir: cache.clone(),
    })
    .expect("update succeeds");
    let item = &report.items[0];
    assert!(item.changed);
    assert_eq!(item.from, "1.0.0");
    assert_eq!(item.to, "1.1.0");
    let cache_prefix = cache.display().to_string();
    assert!(
        item.backup
            .as_ref()
            .is_some_and(|path| path.starts_with(cache_prefix.as_str())),
        "old version must be backed up under the cache: {item:?}"
    );
    let backups: Vec<_> = std::fs::read_dir(cache.join("backup"))
        .expect("cache backup dir exists")
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(backups.len(), 1, "one backup per update: {backups:?}");

    assert_eq!(read_string(&root.join("scripts/alpha.lua")), "v1.1");
    assert_eq!(read_string(&root.join("scripts/extra.lua")), "new");
    assert!(!root.join("scripts/old.lua").exists());
    assert!(read_string(&root.join("config.d/packages/alpha.conf")).contains("osd-font-size=44"));

    let lock = lock::read(&root.join("packages.lock")).expect("lock readable");
    let alpha = lock.find("alpha").expect("alpha in lock");
    assert_eq!(alpha.version, "1.1.0");
    assert_eq!(alpha.files, vec!["scripts/alpha.lua", "scripts/extra.lua"]);
}

#[test]
fn update_to_same_version_is_a_noop() {
    let dir = TestDir::new("update-noop");
    let root = dir.path();
    let cache = root.join("cache");
    write(
        &root.join("packages/alpha.yaml"),
        &manifest(
            "alpha",
            "1.0.0",
            &[("alpha.lua", "~~/scripts/alpha.lua")],
            &[],
            &[],
            &[],
            None,
        ),
    );
    write(&root.join("packages/alpha.lua"), "v1");
    install_local(root, &cache, "alpha");

    let report = lifecycle::update(&UpdateOptions {
        name: Some("alpha".to_owned()),
        repo_root: root.to_path_buf(),
        cache_dir: cache.clone(),
    })
    .expect("update succeeds");
    assert!(!report.items[0].changed);

    let lock = lock::read(&root.join("packages.lock")).expect("lock readable");
    assert_eq!(lock.find("alpha").expect("alpha").version, "1.0.0");
}

#[test]
fn update_failure_rolls_back_to_old_version() {
    let dir = TestDir::new("update-rollback");
    let root = dir.path();
    let cache = root.join("cache");
    write(
        &root.join("packages/alpha.yaml"),
        &manifest(
            "alpha",
            "1.0.0",
            &[("alpha.lua", "~~/scripts/alpha.lua")],
            &[],
            &[],
            &[],
            None,
        ),
    );
    write(&root.join("packages/alpha.lua"), "v1");
    install_local(root, &cache, "alpha");

    write(
        &root.join("packages/alpha.yaml"),
        &manifest(
            "alpha",
            "1.1.0",
            &[
                ("alpha.lua", "~~/scripts/alpha.lua"),
                ("missing.lua", "~~/scripts/missing.lua"),
            ],
            &[],
            &[],
            &[],
            None,
        ),
    );
    write(&root.join("packages/alpha.lua"), "v1.1");

    let error = lifecycle::update(&UpdateOptions {
        name: Some("alpha".to_owned()),
        repo_root: root.to_path_buf(),
        cache_dir: cache.clone(),
    })
    .expect_err("update with missing source fails");

    assert!(error.to_string().contains("源文件"), "{error}");
    assert_eq!(read_string(&root.join("scripts/alpha.lua")), "v1");
    assert!(!root.join("scripts/missing.lua").exists());
    let lock = lock::read(&root.join("packages.lock")).expect("lock readable");
    assert_eq!(lock.find("alpha").expect("alpha").version, "1.0.0");
}

#[test]
fn update_not_installed_is_a_friendly_error() {
    let dir = TestDir::new("update-missing");
    let root = dir.path();
    let error = lifecycle::update(&UpdateOptions {
        name: Some("ghost".to_owned()),
        repo_root: root.to_path_buf(),
        cache_dir: root.join("cache"),
    })
    .expect_err("update of unknown package fails");
    assert!(error.to_string().contains("未安装"), "{error}");
}
