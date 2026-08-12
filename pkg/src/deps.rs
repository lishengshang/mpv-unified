//! Dependency resolution and conflict detection between package manifests.
//!
//! Resolution is name-level only (MVP; no version ranges): [`resolve_install_order`]
//! builds a graph over the `requires` lists of a candidate set, topologically
//! sorts it so every dependency is installed before its dependents, and reports
//! cycles with a human-readable path (`a -> b -> a`) and missing dependencies
//! with the package that requires them.
//!
//! [`check_conflicts`] rejects installing a package whose `conflicts` list
//! names a package that is already installed (or another candidate);
//! [`check_file_conflicts`] rejects dest paths that overlap (nest) with paths
//! already installed by other packages — an *identical* path is a shared file
//! and is allowed. Detection only reports; it never resolves automatically.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::manifest::Manifest;

/// A dependency-resolution or conflict-detection failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepError {
    /// A required package is not present in the candidate set.
    Missing {
        package: String,
        required_by: String,
    },
    /// A dependency cycle, `path` is `a -> b -> a`-style and starts and ends
    /// with the same package.
    Cycle { path: String },
    /// Two packages are mutually exclusive via their `conflicts` lists.
    Conflict {
        package_a: String,
        package_b: String,
    },
    /// An incoming dest path overlaps (nests inside) a path already installed
    /// by another package. Identical paths are shared files and never reported.
    FileConflict {
        path: String,
        package_a: String,
        package_b: String,
    },
    /// An internal invariant broke (e.g. the DFS walk stack emptied
    /// unexpectedly). Unreachable given the resolution pre-checks, but
    /// reported as an error instead of panicking.
    Internal { message: String },
}

impl fmt::Display for DepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing {
                package,
                required_by,
            } => write!(
                f,
                "依赖缺失: 包 \"{package}\" 未被提供 (被 \"{required_by}\" 依赖)"
            ),
            Self::Cycle { path } => write!(f, "依赖环: {path}"),
            Self::Conflict {
                package_a,
                package_b,
            } => write!(
                f,
                "冲突: 包 \"{package_a}\" 与 \"{package_b}\" 互斥,不能同时安装"
            ),
            Self::FileConflict {
                path,
                package_a,
                package_b,
            } => write!(
                f,
                "文件冲突: 包 \"{package_a}\" 的文件 \"{path}\" 与包 \"{package_b}\" 已安装路径重叠"
            ),
            Self::Internal { message } => write!(f, "内部错误: {message}"),
        }
    }
}

impl std::error::Error for DepError {}

/// Topologically sort `candidates` by their `requires` lists so every
/// dependency is installed before its dependents.
///
/// Returns the package names in install order. Fails with [`DepError::Missing`]
/// when a required package is absent from the candidate set, or
/// [`DepError::Cycle`] with a readable cycle path otherwise.
///
/// # Errors
///
/// Returns [`DepError::Missing`] for the first missing dependency (in
/// candidate order) and [`DepError::Cycle`] for the first cycle found by the
/// depth-first walk.
pub fn resolve_install_order(candidates: &[&Manifest]) -> Result<Vec<String>, DepError> {
    let mut by_name: HashMap<&str, &Manifest> = HashMap::new();
    for m in candidates {
        by_name.insert(&m.name, m);
    }

    // Missing dependencies first: a package must never be silently skipped.
    for m in candidates {
        for required in &m.requires {
            if !by_name.contains_key(required.as_str()) {
                return Err(DepError::Missing {
                    package: required.clone(),
                    required_by: m.name.clone(),
                });
            }
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Color {
        White,
        Gray,
        Black,
    }

    let mut colors: HashMap<&str, Color> =
        by_name.keys().map(|&name| (name, Color::White)).collect();
    let mut stack: Vec<&str> = Vec::new();
    let mut stack_pos: HashMap<&str, usize> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    fn visit<'a>(
        name: &'a str,
        manifest: &Manifest,
        by_name: &HashMap<&'a str, &'a Manifest>,
        colors: &mut HashMap<&'a str, Color>,
        stack: &mut Vec<&'a str>,
        stack_pos: &mut HashMap<&'a str, usize>,
        order: &mut Vec<String>,
    ) -> Result<(), DepError> {
        colors.insert(name, Color::Gray);
        stack.push(name);
        stack_pos.insert(name, stack.len() - 1);

        for required in &manifest.requires {
            let dep = by_name
                .get(required.as_str())
                .ok_or_else(|| DepError::Missing {
                    package: required.clone(),
                    required_by: name.to_string(),
                })?;
            match colors[dep.name.as_str()] {
                Color::Black => {}
                Color::Gray => {
                    // Back edge: the required package is already on the walk
                    // stack, so `stack[start..]` followed by that package is
                    // a cycle starting and ending with the same name.
                    let start = stack_pos[dep.name.as_str()];
                    let path = stack[start..]
                        .iter()
                        .map(|s| (*s).to_string())
                        .chain([dep.name.clone()])
                        .collect::<Vec<_>>()
                        .join(" -> ");
                    return Err(DepError::Cycle { path });
                }
                Color::White => visit(&dep.name, dep, by_name, colors, stack, stack_pos, order)?,
            }
        }

        // `name` was pushed at the top of this visit and every recursive
        // visit pops what it pushed, so the stack is never empty here;
        // the unreachable miss is reported as an error instead of panicking.
        let finished = stack.pop().ok_or_else(|| DepError::Internal {
            message: format!("依赖解析栈在遍历 {name} 时意外为空"),
        })?;
        stack_pos.remove(finished);
        colors.insert(name, Color::Black);
        // Dependencies finish before their dependents, so finish order IS the
        // install order (dependency first).
        order.push(name.to_string());
        Ok(())
    }

    for m in candidates {
        if colors[m.name.as_str()] == Color::White {
            visit(
                &m.name,
                m,
                &by_name,
                &mut colors,
                &mut stack,
                &mut stack_pos,
                &mut order,
            )?;
        }
    }

    Ok(order)
}

/// Check mutual-exclusion (`conflicts`) between already-installed packages and
/// the incoming candidates.
///
/// A conflict fires when an incoming package's `conflicts` list names a
/// package that is installed or another candidate, or when an installed
/// package's `conflicts` list names an incoming package.
///
/// # Errors
///
/// Returns [`DepError::Conflict`] naming both packages on the first clash.
pub fn check_conflicts(existing: &[&Manifest], incoming: &[&Manifest]) -> Result<(), DepError> {
    let existing_names: HashSet<&str> = existing.iter().map(|m| m.name.as_str()).collect();
    let incoming_names: HashSet<&str> = incoming.iter().map(|m| m.name.as_str()).collect();

    // Incoming package declares a conflict with something installed or queued.
    for a in incoming {
        for b in &a.conflicts {
            if existing_names.contains(b.as_str()) || incoming_names.contains(b.as_str()) {
                return Err(DepError::Conflict {
                    package_a: a.name.clone(),
                    package_b: b.clone(),
                });
            }
        }
    }
    // Installed package declares a conflict with an incoming package
    // (asymmetric `conflicts` lists).
    for a in existing {
        for b in &a.conflicts {
            if incoming_names.contains(b.as_str()) {
                return Err(DepError::Conflict {
                    package_a: a.name.clone(),
                    package_b: b.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Check that the dest paths of `incoming` do not overlap paths already
/// installed by other packages (`existing_files`: package name → its dest
/// paths).
///
/// An identical path is a shared file and is allowed; only *nesting* overlap
/// (one path is a strict prefix directory of the other) is a conflict.
///
/// # Errors
///
/// Returns [`DepError::FileConflict`] with the overlapping path and both
/// package names on the first overlap.
pub fn check_file_conflicts(
    existing_files: &HashMap<String, Vec<String>>,
    incoming: &Manifest,
) -> Result<(), DepError> {
    for entry in &incoming.files {
        for (pkg, paths) in existing_files {
            for installed in paths {
                if entry.dest == *installed {
                    continue; // identical path: shared file, not a conflict
                }
                let nests_under = |dir: &str, file: &str| {
                    file.strip_prefix(dir)
                        .is_some_and(|rest| rest.starts_with('/'))
                };
                if nests_under(installed, &entry.dest) || nests_under(&entry.dest, installed) {
                    return Err(DepError::FileConflict {
                        path: entry.dest.clone(),
                        package_a: incoming.name.clone(),
                        package_b: pkg.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
