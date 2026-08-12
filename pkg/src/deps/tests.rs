use std::collections::HashMap;

use crate::deps::{check_conflicts, check_file_conflicts, resolve_install_order, DepError};
use crate::manifest::{FileEntry, Manifest, Platform};

fn manifest(name: &str, requires: &[&str], conflicts: &[&str], files: &[&str]) -> Manifest {
    Manifest {
        name: name.to_string(),
        version: semver::Version::parse("1.0.0").expect("valid version"),
        description: String::new(),
        author: None,
        homepage: None,
        license: None,
        platform: Platform::All,
        requires: requires.iter().map(|s| (*s).to_string()).collect(),
        conflicts: conflicts.iter().map(|s| (*s).to_string()).collect(),
        files: files
            .iter()
            .map(|dest| FileEntry {
                src: "pkg-file".to_string(),
                dest: (*dest).to_string(),
            })
            .collect(),
        config: Vec::new(),
    }
}

fn refs(ms: &[Manifest]) -> Vec<&Manifest> {
    ms.iter().collect()
}

#[test]
fn resolve_empty_candidates_gives_empty_order() {
    let candidates: Vec<&Manifest> = Vec::new();
    assert_eq!(resolve_install_order(&candidates), Ok(Vec::<String>::new()));
}

#[test]
fn resolve_single_package_without_requires_returns_itself() {
    let a = manifest("a", &[], &[], &["~~/scripts/a.lua"]);
    assert_eq!(
        resolve_install_order(&refs(&[a])),
        Ok(vec!["a".to_string()])
    );
}

#[test]
fn resolve_linear_chain_orders_dependencies_first() {
    let a = manifest("a", &["b"], &[], &["~~/scripts/a.lua"]);
    let b = manifest("b", &["c"], &[], &["~~/scripts/b.lua"]);
    let c = manifest("c", &[], &[], &["~~/scripts/c.lua"]);
    // a -> b -> c: c must be installed before b, b before a.
    assert_eq!(
        resolve_install_order(&refs(&[a, b, c])),
        Ok(vec!["c".to_string(), "b".to_string(), "a".to_string()])
    );
}

#[test]
fn resolve_missing_dependency_reports_package_and_requirer() {
    let a = manifest("a", &["ghost"], &[], &["~~/scripts/a.lua"]);
    let err = resolve_install_order(&refs(&[a])).expect_err("missing dep must fail");
    assert_eq!(
        err,
        DepError::Missing {
            package: "ghost".to_string(),
            required_by: "a".to_string(),
        }
    );
    assert!(err.to_string().contains("ghost"));
}

#[test]
fn resolve_two_node_cycle_reports_readable_path() {
    let a = manifest("a", &["b"], &[], &["~~/scripts/a.lua"]);
    let b = manifest("b", &["a"], &[], &["~~/scripts/b.lua"]);
    let err = resolve_install_order(&refs(&[a, b])).expect_err("cycle must fail");
    assert_eq!(
        err,
        DepError::Cycle {
            path: "a -> b -> a".to_string(),
        }
    );
}

#[test]
fn resolve_three_node_cycle_reports_full_path() {
    let a = manifest("a", &["b"], &[], &["~~/scripts/a.lua"]);
    let b = manifest("b", &["c"], &[], &["~~/scripts/b.lua"]);
    let c = manifest("c", &["a"], &[], &["~~/scripts/c.lua"]);
    let err = resolve_install_order(&refs(&[a, b, c])).expect_err("cycle must fail");
    assert_eq!(
        err,
        DepError::Cycle {
            path: "a -> b -> c -> a".to_string(),
        }
    );
    assert!(err.to_string().contains("a -> b -> c -> a"));
}

#[test]
fn conflicts_rejects_incoming_vs_installed() {
    let a = manifest("a", &[], &["b"], &["~~/scripts/a.lua"]);
    let b = manifest("b", &[], &[], &["~~/scripts/b.lua"]);
    let err = check_conflicts(&refs(&[b]), &refs(&[a])).expect_err("conflict must fail");
    assert_eq!(
        err,
        DepError::Conflict {
            package_a: "a".to_string(),
            package_b: "b".to_string(),
        }
    );
    assert!(err.to_string().contains("\"a\""));
    assert!(err.to_string().contains("\"b\""));
}

#[test]
fn conflicts_rejects_installed_vs_incoming_asymmetric() {
    let a = manifest("a", &[], &[], &["~~/scripts/a.lua"]);
    let b = manifest("b", &[], &["a"], &["~~/scripts/b.lua"]);
    let err = check_conflicts(&refs(&[b]), &refs(&[a])).expect_err("conflict must fail");
    assert_eq!(
        err,
        DepError::Conflict {
            package_a: "b".to_string(),
            package_b: "a".to_string(),
        }
    );
}

#[test]
fn conflicts_rejects_two_incoming_packages() {
    let a = manifest("a", &[], &["b"], &["~~/scripts/a.lua"]);
    let b = manifest("b", &[], &[], &["~~/scripts/b.lua"]);
    let err = check_conflicts(&[], &refs(&[a, b])).expect_err("conflict must fail");
    assert_eq!(
        err,
        DepError::Conflict {
            package_a: "a".to_string(),
            package_b: "b".to_string(),
        }
    );
}

#[test]
fn conflicts_allow_uninstalled_conflict_name() {
    let a = manifest("a", &[], &["ghost"], &["~~/scripts/a.lua"]);
    // `ghost` is not installed and not being installed: no clash.
    assert_eq!(check_conflicts(&[], &refs(&[a])), Ok(()));
}

#[test]
fn file_conflicts_reports_overlapping_path() {
    let incoming = manifest("a", &[], &[], &["~~/scripts/tools/util.lua"]);
    let existing_files = HashMap::from([("b".to_string(), vec!["~~/scripts/tools".to_string()])]);
    let err = check_file_conflicts(&existing_files, &incoming).expect_err("overlap must fail");
    assert_eq!(
        err,
        DepError::FileConflict {
            path: "~~/scripts/tools/util.lua".to_string(),
            package_a: "a".to_string(),
            package_b: "b".to_string(),
        }
    );
    assert!(err.to_string().contains("~~/scripts/tools/util.lua"));
}

#[test]
fn file_conflicts_identical_path_is_shared_and_allowed() {
    let incoming = manifest("a", &[], &[], &["~~/scripts/shared.lua"]);
    let existing_files =
        HashMap::from([("b".to_string(), vec!["~~/scripts/shared.lua".to_string()])]);
    assert_eq!(check_file_conflicts(&existing_files, &incoming), Ok(()));
}

#[test]
fn file_conflicts_sibling_paths_do_not_clash() {
    let incoming = manifest("a", &[], &[], &["~~/scripts/a.lua"]);
    let existing_files = HashMap::from([("b".to_string(), vec!["~~/scripts/b.lua".to_string()])]);
    assert_eq!(check_file_conflicts(&existing_files, &incoming), Ok(()));
}

#[test]
fn resolve_multi_entry_dependency_orders_all_dependencies_first() {
    // a requires b and c; d requires a and c.
    let a = manifest("a", &["b", "c"], &[], &["~~/scripts/a.lua"]);
    let b = manifest("b", &[], &[], &["~~/scripts/b.lua"]);
    let c = manifest("c", &[], &[], &["~~/scripts/c.lua"]);
    let d = manifest("d", &["a", "c"], &[], &["~~/scripts/d.lua"]);
    let order = resolve_install_order(&refs(&[a, b, c, d])).expect("acyclic graph sorts");
    assert_eq!(order, vec!["b", "c", "a", "d"]);
    for (dep, dependent) in [("b", "a"), ("c", "a"), ("a", "d"), ("c", "d")] {
        let at = |name: &str| {
            order
                .iter()
                .position(|n| n == name)
                .expect("every package appears in order")
        };
        assert!(
            at(dep) < at(dependent),
            "{dep} must precede {dependent} in {order:?}"
        );
    }
}
