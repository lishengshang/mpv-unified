//! Integration tests for pending (git-source) records: whitelist/blacklist
//! filtering against a local git repository fixture, clone cleanup, and
//! git-source updates.

mod common;

use cli::pkg_cmds::lifecycle::lock;
use cli::pkg_cmds::lifecycle::{self, InstallOptions, UpdateOptions};
use common::lifecycle::{file_url, git_repo, read_string, write, TestDir};
use core::platform::Platform;

fn install_opts(root: &std::path::Path, cache: &std::path::Path, name: &str) -> InstallOptions {
    InstallOptions {
        name: name.to_owned(),
        repo_root: root.to_path_buf(),
        cache_dir: cache.to_path_buf(),
        platform: Platform::Linux,
    }
}

#[test]
fn pending_git_install_clones_and_applies_whitelist() {
    let dir = TestDir::new("pending-whitelist");
    let root = dir.path();
    let cache = root.join("cache");
    let repo = root.join("src-repo");
    git_repo(
        &repo,
        "rewrite",
        &[
            ("foo.lua", "local foo = true\n"),
            ("bar.txt", "not lua\n"),
            ("sub/baz.lua", "local baz = true\n"),
        ],
    );
    write(
        &root.join("packages/pending/fromgit.yaml"),
        &format!(
            "name: fromgit\nversion: 0.0.0\ndescription: git fixture\nplatform: all\nsource:\n  git: {}\n  branch: rewrite\n  whitelist: '\\.lua$'\n  dest: ~~/scripts\nmigrated: pending\n",
            file_url(&repo)
        ),
    );

    lifecycle::install(&install_opts(root, &cache, "fromgit")).expect("pending install succeeds");

    // Normalize CRLF (Windows git autocrlf) so the fixture stays byte-stable
    // on every runner.
    assert_eq!(
        read_string(&root.join("scripts/foo.lua")).replace("\r\n", "\n"),
        "local foo = true\n"
    );
    assert_eq!(
        read_string(&root.join("scripts/sub/baz.lua")).replace("\r\n", "\n"),
        "local baz = true\n"
    );
    assert!(!root.join("scripts/bar.txt").exists());

    let lock = lock::read(&root.join("packages.lock")).expect("lock readable");
    let entry = lock.find("fromgit").expect("fromgit in lock");
    assert_eq!(entry.version, "0.0.0");
    assert_eq!(
        entry.files,
        vec![
            "scripts/foo.lua".to_owned(),
            "scripts/sub/baz.lua".to_owned()
        ]
    );
    let leftovers: Vec<_> = std::fs::read_dir(&cache)
        .expect("cache exists")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("fromgit-"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "clone dir must be cleaned up: {leftovers:?}"
    );
}

#[test]
fn pending_git_install_applies_blacklist() {
    let dir = TestDir::new("pending-blacklist");
    let root = dir.path();
    let cache = root.join("cache");
    let repo = root.join("src-repo");
    git_repo(
        &repo,
        "main",
        &[
            ("keep.lua", "local k = true\n"),
            ("skip.lua", "local s = true\n"),
        ],
    );
    write(
        &root.join("packages/pending/bl.yaml"),
        &format!(
            "name: bl\nversion: 0.0.0\ndescription: git fixture\nplatform: all\nsource:\n  git: {}\n  blacklist: '^skip'\n  dest: ~~/scripts\nmigrated: pending\n",
            file_url(&repo)
        ),
    );

    lifecycle::install(&install_opts(root, &cache, "bl")).expect("pending install succeeds");

    assert!(root.join("scripts/keep.lua").exists());
    assert!(!root.join("scripts/skip.lua").exists());
}

#[test]
fn pending_git_update_reclones_and_replaces() {
    let dir = TestDir::new("pending-update");
    let root = dir.path();
    let cache = root.join("cache");
    let repo = root.join("src-repo");
    git_repo(&repo, "main", &[("script.lua", "v1\n")]);
    write(
        &root.join("packages/pending/gitpkg.yaml"),
        &format!(
            "name: gitpkg\nversion: 0.0.0\ndescription: git fixture\nplatform: all\nsource:\n  git: {}\n  whitelist: '\\.lua$'\n  dest: ~~/scripts\nmigrated: pending\n",
            file_url(&repo)
        ),
    );
    lifecycle::install(&install_opts(root, &cache, "gitpkg")).expect("install succeeds");
    assert_eq!(
        read_string(&root.join("scripts/script.lua")).replace("\r\n", "\n"),
        "v1\n"
    );

    git_repo(
        &repo,
        "main",
        &[("script.lua", "v2\n"), ("extra.lua", "x\n")],
    );

    let report = lifecycle::update(&UpdateOptions {
        name: Some("gitpkg".to_owned()),
        repo_root: root.to_path_buf(),
        cache_dir: cache.clone(),
    })
    .expect("git update succeeds");
    assert!(report.items[0].changed, "{report:?}");
    assert_eq!(
        read_string(&root.join("scripts/script.lua")).replace("\r\n", "\n"),
        "v2\n"
    );
    assert!(root.join("scripts/extra.lua").exists());
}
