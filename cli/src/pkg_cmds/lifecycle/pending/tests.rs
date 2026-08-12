//! Unit tests for pending-record parsing and whitelist/blacklist filtering.

use super::regex_filters::compile_filters;
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "mpv-config-pending-{label}-{}",
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

const VALID_RECORD: &str = "\
name: demopkg
version: 0.0.0
description: demo
platform: all
source:
  git: https://example.com/owner/repo
  branch: main
  whitelist: '\\.lua$'
  dest: ~~/scripts
migrated: pending
";

#[test]
fn valid_record_parses() {
    let path = temp_path("valid");
    fs::write(&path, VALID_RECORD).expect("write fixture");
    let record = PendingRecord::parse_file(&path).expect("valid record parses");
    assert_eq!(record.name, "demopkg");
    assert_eq!(record.source.branch.as_deref(), Some("main"));
    assert_eq!(record.platform(), Platform::All);
}

#[test]
fn non_pending_record_is_rejected() {
    let path = temp_path("notpending");
    fs::write(
        &path,
        VALID_RECORD.replace("migrated: pending", "migrated: done"),
    )
    .expect("write fixture");
    let error = PendingRecord::parse_file(&path).expect_err("non-pending record fails");
    match error {
        LifecycleError::Pending { name, message } => {
            assert_eq!(name, "demopkg");
            assert!(message.contains("pending"), "{message}");
        }
        other => panic!("expected Pending error, got {other}"),
    }
}

#[test]
fn invalid_dest_is_rejected() {
    let path = temp_path("baddest");
    fs::write(&path, VALID_RECORD.replace("~~/scripts", "/etc/scripts")).expect("write fixture");
    let error = PendingRecord::parse_file(&path).expect_err("bad dest fails");
    assert!(matches!(error, LifecycleError::Pending { .. }), "{error}");
}

#[test]
fn whitelist_keeps_only_matching_paths() {
    let filters = compile_filters(Some(r"\.lua$"), None).expect("regex compiles");
    assert!(filters.keep("scripts/foo.lua"));
    assert!(!filters.keep("scripts/foo.txt"));
    assert!(!filters.keep("README.md"));
}

#[test]
fn blacklist_drops_matching_paths() {
    let filters = compile_filters(None, Some(r"(?i)readme|\.md$")).expect("regex compiles");
    assert!(filters.keep("scripts/foo.lua"));
    assert!(!filters.keep("README.md"));
    assert!(!filters.keep("docs/notes.md"));
}

#[test]
fn whitelist_and_blacklist_combine() {
    let filters = compile_filters(Some(r"\.lua$"), Some(r"drcbox")).expect("regex compiles");
    assert!(filters.keep("scripts/foo.lua"));
    assert!(!filters.keep("scripts/drcbox.lua"));
    assert!(!filters.keep("scripts/foo.txt"));
}

#[test]
fn invalid_pattern_is_a_typed_error() {
    let error = compile_filters(Some("(unclosed"), None).expect_err("bad regex fails");
    assert!(matches!(error, LifecycleError::Regex { .. }), "{error}");
}

#[test]
fn no_filters_keeps_everything() {
    let filters = compile_filters(None, None).expect("no regex");
    assert!(filters.keep("anything/at-all.lua"));
}
