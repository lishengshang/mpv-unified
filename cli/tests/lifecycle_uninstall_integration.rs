//! Integration tests for `pkg uninstall`: shared-file safety (a path used
//! by another package is kept), config-fragment removal, and lock updates.

mod common;

use cli::pkg_cmds::lifecycle::lock;
use cli::pkg_cmds::lifecycle::{self, InstallOptions, UninstallOptions};
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
fn uninstall_keeps_shared_files_and_removes_own() {
    let dir = TestDir::new("uninstall-shared");
    let root = dir.path();
    let cache = root.join("cache");
    write(
        &root.join("packages/alpha.yaml"),
        &manifest(
            "alpha",
            "1.0.0",
            &[
                ("alpha.lua", "~~/scripts/alpha.lua"),
                ("shared.lua", "~~/scripts/shared.lua"),
            ],
            &["osd-font-size=30"],
            &[],
            &[],
            None,
        ),
    );
    write(&root.join("packages/alpha.lua"), "a");
    write(&root.join("packages/shared.lua"), "s");
    write(
        &root.join("packages/beta.yaml"),
        &manifest(
            "beta",
            "2.0.0",
            &[
                ("beta.lua", "~~/scripts/beta.lua"),
                ("shared.lua", "~~/scripts/shared.lua"),
            ],
            &[],
            &[],
            &[],
            None,
        ),
    );
    write(&root.join("packages/beta.lua"), "b");
    write(&root.join("packages/shared.lua"), "s");
    install_local(root, &cache, "alpha");
    install_local(root, &cache, "beta");

    let report = lifecycle::uninstall(&UninstallOptions {
        name: "alpha".to_owned(),
        repo_root: root.to_path_buf(),
    })
    .expect("uninstall succeeds");

    assert_eq!(report.files_removed, 1);
    assert_eq!(report.files_shared, 1);
    assert!(report.config_removed);
    assert!(!root.join("scripts/alpha.lua").exists());
    assert_eq!(read_string(&root.join("scripts/shared.lua")), "s");
    assert!(!root.join("config.d/packages/alpha.conf").exists());

    let lock = lock::read(&root.join("packages.lock")).expect("lock readable");
    assert!(lock.find("alpha").is_none());
    assert!(lock.find("beta").is_some());
}

#[test]
fn uninstall_not_installed_is_a_friendly_error() {
    let dir = TestDir::new("uninstall-missing");
    let root = dir.path();
    let error = lifecycle::uninstall(&UninstallOptions {
        name: "ghost".to_owned(),
        repo_root: root.to_path_buf(),
    })
    .expect_err("uninstall of unknown package fails");
    assert!(error.to_string().contains("未安装"), "{error}");
}

#[test]
fn uninstall_removes_all_files_when_last_user() {
    let dir = TestDir::new("uninstall-last-user");
    let root = dir.path();
    let cache = root.join("cache");
    write(
        &root.join("packages/beta.yaml"),
        &manifest(
            "beta",
            "2.0.0",
            &[("beta.lua", "~~/scripts/beta.lua")],
            &[],
            &[],
            &[],
            None,
        ),
    );
    write(&root.join("packages/beta.lua"), "b");
    install_local(root, &cache, "beta");

    let report = lifecycle::uninstall(&UninstallOptions {
        name: "beta".to_owned(),
        repo_root: root.to_path_buf(),
    })
    .expect("uninstall succeeds");
    assert_eq!(report.files_removed, 1);
    assert!(!root.join("scripts/beta.lua").exists());
    let lock = lock::read(&root.join("packages.lock")).expect("lock readable");
    assert!(lock.packages.is_empty());
}
