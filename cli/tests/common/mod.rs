//! Shared helpers for the integration test binaries.
//!
//! This module is not a test binary itself; the per-suite test files import
//! it with `mod common;`. Every suite links the whole module, so helpers
//! unused by one suite are dead there by construction.
#![allow(dead_code)]

use cli::doctor::{self, DoctorReport, Finding};
use core::platform::Platform;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

pub struct TestDir {
    path: PathBuf,
}

impl TestDir {
    pub fn new(label: &str) -> Self {
        let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("mpv-config-doctor-{label}-{id}"));
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

/// Mock `mpv --list-options` output: headers, flag, plain, choices, a
/// suffix-variant continuation line and the list-group markers.
const MOCK_OPTIONS: &str = "\
Options:

 --osd-bar                        Flag (default: yes)
 --vo                             String (default: gpu)
 --hwdec                          Choices: auto no yes (default: auto)
    --ytdl-raw-options-append
 --ab-loop-count                  Choices: inf (or an integer) (0 to 2147483647) (default: inf)
 --{
 --}
";

pub fn source_ok() -> impl Fn() -> Option<String> {
    || Some(MOCK_OPTIONS.to_owned())
}

pub fn source_none() -> impl Fn() -> Option<String> {
    || None
}

pub fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(path, content).expect("write fixture file");
}

/// A healthy repository: every layer present and parseable; the merged
/// output uses only options known to the mock list (`no-osd-bar` exercises
/// the `no-`-prefix rule).
pub fn healthy_root(root: &Path) {
    write(&root.join("config/base.conf"), "vo=gpu\n# base comment\n");
    write(&root.join("config/linux.conf"), "hwdec=auto\n");
    write(
        &root.join("config/windows.conf"),
        "window-corners=roundsmall\n",
    );
    write(&root.join("config/macos.conf"), "vo=gpu\n");
    write(&root.join("config/input.conf"), "UP cycle sub-visibility\n");
    write(&root.join("user/user.conf"), "no-osd-bar\n");
}

pub fn run(root: &Path) -> DoctorReport {
    doctor::run_with(root, Platform::Linux, &source_ok())
}

/// Findings of the check named `name`.
pub fn findings_of<'a>(report: &'a DoctorReport, name: &str) -> Vec<&'a Finding> {
    report
        .checks
        .iter()
        .find(|check| check.name == name)
        .expect("check exists")
        .findings
        .iter()
        .collect()
}

pub fn finding_messages(report: &DoctorReport, name: &str) -> Vec<String> {
    findings_of(report, name)
        .iter()
        .map(|finding| finding.message.clone())
        .collect()
}

pub mod lifecycle;
