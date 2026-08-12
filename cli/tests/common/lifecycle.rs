//! Shared fixtures for the lifecycle (install/uninstall/update) integration
//! tests: temp-dir repositories, manifest builders, and a local git
//! repository factory.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

/// A temp-dir repository root, removed on drop.
pub struct TestDir {
    path: PathBuf,
}

impl TestDir {
    pub fn new(label: &str) -> Self {
        let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("mpv-config-lifecycle-{label}-{id}"));
        fs::create_dir_all(&path).expect("create test directory");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(path, content).expect("write fixture file");
}

/// Render a manifest YAML with the given files (`(src, dest)` pairs),
/// `config` lines, and dependency/conflict lists.
pub fn manifest(
    name: &str,
    version: &str,
    files: &[(&str, &str)],
    config: &[&str],
    requires: &[&str],
    conflicts: &[&str],
    platform: Option<&str>,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "name: {name}\nversion: {version}\ndescription: test\n"
    ));
    if let Some(platform) = platform {
        out.push_str(&format!("platform: {platform}\n"));
    }
    if !requires.is_empty() {
        out.push_str("requires:\n");
        for r in requires {
            out.push_str(&format!("  - {r}\n"));
        }
    }
    if !conflicts.is_empty() {
        out.push_str("conflicts:\n");
        for c in conflicts {
            out.push_str(&format!("  - {c}\n"));
        }
    }
    out.push_str("files:\n");
    for (src, dest) in files {
        out.push_str(&format!("  - src: {src}\n    dest: {dest}\n"));
    }
    if !config.is_empty() {
        out.push_str("config:\n");
        for line in config {
            out.push_str(&format!("  - {line}\n"));
        }
    }
    out
}

/// Create a local git repository at `dir` with `files` committed on
/// `branch`, ready to be cloned via its `file://` URL.
pub fn git_repo(dir: &Path, branch: &str, files: &[(&str, &str)]) {
    fs::create_dir_all(dir).expect("create git fixture dir");
    run_git(dir, &["init", "-q", "-b", branch]);
    for (path, content) in files {
        write(&dir.join(path), content);
    }
    run_git(dir, &["add", "-A"]);
    run_git(
        dir,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@test.local",
            "commit",
            "-q",
            "-m",
            "fixture",
        ],
    );
}

pub fn run_git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("git runs in tests");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `file://` URL of a fixture repository.
pub fn file_url(dir: &Path) -> String {
    format!("file://{}", dir.display())
}

pub fn read_string(path: &Path) -> String {
    fs::read_to_string(path).expect("fixture file readable")
}
