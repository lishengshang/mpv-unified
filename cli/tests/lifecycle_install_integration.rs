//! Integration tests for `pkg install` against temp-dir repository
//! fixtures: file placement, config fragments, lock updates, and
//! duplicate/missing-dependency/conflict/platform errors.

mod common;

use cli::pkg_cmds::lifecycle::lock;
use cli::pkg_cmds::lifecycle::{self, InstallOptions, InstallOutcome};
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
    let outcome = lifecycle::install(&install_opts(root, cache, name)).expect("install succeeds");
    assert!(
        matches!(outcome, InstallOutcome::Installed(_)),
        "{outcome:?}"
    );
}

#[test]
fn install_two_local_packages_lands_files_fragment_and_lock() {
    let dir = TestDir::new("two-packages");
    let root = dir.path();
    let cache = root.join("cache");
    write(
        &root.join("packages/alpha.yaml"),
        &manifest(
            "alpha",
            "1.0.0",
            &[("alpha.lua", "~~/scripts/alpha.lua")],
            &["osd-font-size=30"],
            &[],
            &[],
            None,
        ),
    );
    write(&root.join("packages/alpha.lua"), "return 1\n");
    write(
        &root.join("packages/beta.yaml"),
        &manifest(
            "beta",
            "2.0.0",
            &[("beta.lua", "~~/script-opts/beta.lua")],
            &[],
            &[],
            &[],
            None,
        ),
    );
    write(&root.join("packages/beta.lua"), "return 2\n");

    install_local(root, &cache, "alpha");
    install_local(root, &cache, "beta");

    assert_eq!(read_string(&root.join("scripts/alpha.lua")), "return 1\n");
    assert_eq!(
        read_string(&root.join("script-opts/beta.lua")),
        "return 2\n"
    );
    let fragment = read_string(&root.join("config.d/packages/alpha.conf"));
    assert!(fragment.contains("osd-font-size=30"), "{fragment}");
    assert!(!root.join("config.d/packages/beta.conf").exists());

    let lock = lock::read(&root.join("packages.lock")).expect("lock readable");
    assert_eq!(lock.packages.len(), 2);
    let alpha = lock.find("alpha").expect("alpha in lock");
    assert_eq!(alpha.version, "1.0.0");
    assert_eq!(alpha.files, vec!["scripts/alpha.lua"]);
    assert!(alpha.config_d);
    let beta = lock.find("beta").expect("beta in lock");
    assert!(!beta.config_d);
}

#[test]
fn install_existing_package_suggests_update() {
    let dir = TestDir::new("duplicate");
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
    write(&root.join("packages/alpha.lua"), "x");
    install_local(root, &cache, "alpha");

    let error =
        lifecycle::install(&install_opts(root, &cache, "alpha")).expect_err("duplicate fails");
    let message = error.to_string();
    assert!(message.contains("已安装"), "{message}");
    assert!(message.contains("update"), "{message}");
}

#[test]
fn install_missing_dependency_reports_and_suggests_install() {
    let dir = TestDir::new("missing-dep");
    let root = dir.path();
    let cache = root.join("cache");
    write(
        &root.join("packages/gamma.yaml"),
        &manifest(
            "gamma",
            "1.0.0",
            &[("gamma.lua", "~~/scripts/gamma.lua")],
            &[],
            &["zeta"],
            &[],
            None,
        ),
    );
    write(&root.join("packages/gamma.lua"), "x");

    let error =
        lifecycle::install(&install_opts(root, &cache, "gamma")).expect_err("missing dep fails");
    let message = error.to_string();
    assert!(message.contains("缺少依赖"), "{message}");
    assert!(message.contains("pkg install zeta"), "{message}");
    assert!(!root.join("scripts/gamma.lua").exists());
}

#[test]
fn install_with_satisfied_dependency_succeeds() {
    let dir = TestDir::new("satisfied-dep");
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
    write(&root.join("packages/alpha.lua"), "a");
    write(
        &root.join("packages/gamma.yaml"),
        &manifest(
            "gamma",
            "1.0.0",
            &[("gamma.lua", "~~/scripts/gamma.lua")],
            &[],
            &["alpha"],
            &[],
            None,
        ),
    );
    write(&root.join("packages/gamma.lua"), "g");
    install_local(root, &cache, "alpha");

    install_local(root, &cache, "gamma");

    assert!(root.join("scripts/gamma.lua").exists());
}

#[test]
fn install_conflicting_package_is_rejected() {
    let dir = TestDir::new("conflict");
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
    write(&root.join("packages/alpha.lua"), "a");
    write(
        &root.join("packages/delta.yaml"),
        &manifest(
            "delta",
            "1.0.0",
            &[("delta.lua", "~~/scripts/delta.lua")],
            &[],
            &[],
            &["alpha"],
            None,
        ),
    );
    write(&root.join("packages/delta.lua"), "d");
    install_local(root, &cache, "alpha");

    let error =
        lifecycle::install(&install_opts(root, &cache, "delta")).expect_err("conflict fails");
    let message = error.to_string();
    assert!(message.contains("冲突"), "{message}");
    assert!(!root.join("scripts/delta.lua").exists());
    let lock = lock::read(&root.join("packages.lock")).expect("lock readable");
    assert!(lock.find("delta").is_none());
}

#[test]
fn install_platform_mismatch_skips_without_side_effects() {
    let dir = TestDir::new("platform-skip");
    let root = dir.path();
    let cache = root.join("cache");
    write(
        &root.join("packages/winonly.yaml"),
        &manifest(
            "winonly",
            "1.0.0",
            &[("w.lua", "~~/scripts/w.lua")],
            &[],
            &[],
            &[],
            Some("windows"),
        ),
    );
    write(&root.join("packages/winonly.lua"), "w");

    let outcome =
        lifecycle::install(&install_opts(root, &cache, "winonly")).expect("skip is not an error");
    match outcome {
        InstallOutcome::SkippedPlatform { name, .. } => assert_eq!(name, "winonly"),
        other => panic!("expected skip, got {other:?}"),
    }
    assert!(!root.join("scripts/w.lua").exists());
    let lock = lock::read(&root.join("packages.lock")).expect("lock readable");
    assert!(lock.find("winonly").is_none());
}

#[test]
fn install_missing_package_is_a_clear_error() {
    let dir = TestDir::new("missing-package");
    let root = dir.path();
    let cache = root.join("cache");
    let error = lifecycle::install(&install_opts(root, &cache, "nope")).expect_err("missing fails");
    assert!(error.to_string().contains("找不到包"), "{error}");
}
