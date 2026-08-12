//! `pkg verify` / `pkg repair` logic (T16): cross-check `packages.lock`
//! against the repository filesystem.
//!
//! [`verify`] reports two problem classes:
//! - `missing`: paths the lock records that no longer exist under the repo
//!   root (file was deleted or the repo was re-cloned without the lock);
//! - `orphan`: files under a managed directory (`scripts/`, `shaders/`,
//!   ...) that no lock entry records — they were either installed by hand,
//!   left behind by a failed uninstall, or the lock lost their entry.
//!
//! [`repair`] is the destructive half: with `yes: true` it deletes orphan
//! files (never files the lock records, never `config.d/packages/`
//! fragments owned by a `config_d` entry). Missing files are only reported
//! — the MVP does not auto-restore them from backups. Without `--yes`,
//! repair refuses to touch anything and returns [`RepairError::ConfirmRequired`].

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use crate::manifest::KNOWN_DEST_DIRS;

use super::{LockError, LockFile};

/// Result of a consistency check; both lists are sorted, deduplicated
/// repository-relative paths.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VerifyReport {
    /// Locked paths that do not exist in the repo.
    pub missing: Vec<String>,
    /// Files in managed directories not recorded by any lock entry.
    pub orphan: Vec<String>,
}

impl VerifyReport {
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.missing.is_empty() && self.orphan.is_empty()
    }
}

impl fmt::Display for VerifyReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_healthy() {
            return write!(f, "packages.lock 与实际文件一致(健康)");
        }
        if !self.missing.is_empty() {
            writeln!(
                f,
                "缺失 {} 个 lock 记录的文件(可能被删除或仓库未完整恢复,请重装对应包):",
                self.missing.len()
            )?;
            for path in &self.missing {
                writeln!(f, "  - {path}")?;
            }
        }
        if !self.orphan.is_empty() {
            writeln!(f, "发现 {} 个 lock 未记录的多余文件:", self.orphan.len())?;
            for path in &self.orphan {
                writeln!(f, "  - {path}")?;
            }
            write!(f, "可用 `mpv-config pkg repair --yes` 清理")?;
        }
        Ok(())
    }
}

/// Outcome of a repair run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairReport {
    /// Locked paths still missing after repair (never auto-restored in MVP).
    pub missing: Vec<String>,
    /// Orphan files deleted during this run.
    pub removed: Vec<String>,
}

impl fmt::Display for RepairReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.missing.is_empty() && self.removed.is_empty() {
            return write!(f, "没有需要修复的问题");
        }
        if !self.removed.is_empty() {
            writeln!(f, "已删除 {} 个多余文件:", self.removed.len())?;
            for path in &self.removed {
                writeln!(f, "  - {path}")?;
            }
        }
        if !self.missing.is_empty() {
            write!(
                f,
                "仍有 {} 个缺失文件无法自动恢复(MVP 不恢复备份,请重装对应包)",
                self.missing.len()
            )?;
        }
        Ok(())
    }
}

/// Repair failures.
#[derive(Debug)]
pub enum RepairError {
    /// Orphan files exist but `--yes` was not given; nothing was touched.
    ConfirmRequired { orphans: Vec<String> },
    /// The consistency scan itself failed (e.g. unreadable directory).
    Lock(LockError),
}

impl fmt::Display for RepairError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfirmRequired { orphans } => {
                writeln!(f, "发现 {} 个多余文件,删除需确认:", orphans.len())?;
                for path in orphans {
                    writeln!(f, "  - {path}")?;
                }
                write!(f, "请先核对清单,再运行 `mpv-config pkg repair --yes`")
            }
            Self::Lock(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for RepairError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Lock(error) => Some(error),
            Self::ConfirmRequired { .. } => None,
        }
    }
}

impl From<LockError> for RepairError {
    fn from(error: LockError) -> Self {
        Self::Lock(error)
    }
}

/// Cross-check `lock` against the files under `repo_root`.
///
/// A recorded path covers itself and, when it names a directory, everything
/// under it (install records `strip_root(dest)` per manifest entry, which
/// may be a directory such as `scripts` or a file such as
/// `scripts/foo.lua`).
///
/// # Errors
///
/// [`LockError::Io`] when a managed directory cannot be read.
pub fn verify(lock: &LockFile, repo_root: &Path) -> Result<VerifyReport, LockError> {
    let mut report = VerifyReport::default();

    for entry in &lock.packages {
        for path in &entry.files {
            if !repo_root.join(path).exists() {
                report.missing.push(path.clone());
            }
        }
    }

    let mut recorded: Vec<String> = lock
        .packages
        .iter()
        .flat_map(|entry| entry.files.iter().cloned())
        .collect();
    recorded.extend(
        lock.packages
            .iter()
            .filter(|entry| entry.config_d)
            .map(|entry| format!("config.d/packages/{}.conf", entry.name)),
    );

    let mut discovered = Vec::new();
    for dir in KNOWN_DEST_DIRS {
        walk_files(repo_root, dir, &mut discovered)?;
    }

    for path in discovered {
        let covered = recorded
            .iter()
            .any(|recorded| recorded == &path || path.starts_with(&format!("{recorded}/")));
        if !covered {
            report.orphan.push(path);
        }
    }

    report.missing.sort();
    report.missing.dedup();
    report.orphan.sort();
    report.orphan.dedup();
    Ok(report)
}

/// Fix what [`verify`] found: with `yes` delete orphan files. Missing files
/// are reported but never auto-restored (MVP).
///
/// # Errors
///
/// [`RepairError::ConfirmRequired`] when orphans exist and `yes` is false;
/// [`RepairError::Lock`] on filesystem failures.
pub fn repair(lock: &LockFile, repo_root: &Path, yes: bool) -> Result<RepairReport, RepairError> {
    let report = verify(lock, repo_root)?;
    if !report.orphan.is_empty() && !yes {
        return Err(RepairError::ConfirmRequired {
            orphans: report.orphan,
        });
    }

    let mut removed = Vec::new();
    for path in &report.orphan {
        let target = repo_root.join(path);
        fs::remove_file(&target).map_err(|source| {
            RepairError::Lock(LockError::Io {
                path: target.clone(),
                source,
            })
        })?;
        removed.push(path.clone());
    }
    Ok(RepairReport {
        missing: report.missing,
        removed,
    })
}

/// Recursively collect files under `repo_root/<dir>` as repo-relative paths.
fn walk_files(repo_root: &Path, dir: &str, out: &mut Vec<String>) -> Result<(), LockError> {
    let base = repo_root.join(dir);
    let entries = match fs::read_dir(&base) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(LockError::Io { path: base, source });
        }
    };
    for entry in entries {
        let entry = entry.map_err(|source| LockError::Io {
            path: base.clone(),
            source,
        })?;
        let path = entry.path();
        let rel = path
            .strip_prefix(repo_root)
            .map_err(|_| LockError::Io {
                path: path.clone(),
                source: io::Error::other("path escapes repo root"),
            })?
            .components()
            .map(|c| c.as_os_str())
            .collect::<Vec<_>>()
            .join(std::ffi::OsStr::new("/"))
            .to_string_lossy()
            .into_owned();
        if entry.file_type().map_or(true, |kind| kind.is_dir()) {
            walk_files(repo_root, &rel, out)?;
        } else {
            out.push(rel);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::LockEntry;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "mpv-config-verify-{label}-{}",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp repo");
        path
    }

    fn entry(name: &str, files: &[&str], config_d: bool) -> LockEntry {
        LockEntry {
            name: name.to_owned(),
            version: "1.0.0".to_owned(),
            files: files.iter().map(|s| s.to_string()).collect(),
            config_d,
        }
    }

    fn write_file(repo: &std::path::Path, rel: &str) {
        let path = repo.join(rel);
        fs::create_dir_all(path.parent().expect("rel has parent")).expect("mkdir");
        fs::write(path, "x").expect("write");
    }

    #[test]
    fn verify_is_healthy_when_everything_matches() {
        let repo = temp_dir("healthy");
        let mut lock = LockFile::default();
        lock.upsert(entry("a", &["scripts/a.lua"], true));
        lock.upsert(entry("b", &["shaders/b.glsl"], false));
        write_file(&repo, "scripts/a.lua");
        write_file(&repo, "shaders/b.glsl");
        write_file(&repo, "config.d/packages/a.conf");
        let report = verify(&lock, &repo).expect("verify ok");
        assert!(report.is_healthy(), "{report:?}");
    }

    #[test]
    fn verify_reports_missing_when_recorded_file_is_deleted() {
        let repo = temp_dir("missing");
        let mut lock = LockFile::default();
        lock.upsert(entry("a", &["scripts/gone.lua"], false));
        let report = verify(&lock, &repo).expect("verify ok");
        assert_eq!(report.missing, vec!["scripts/gone.lua".to_owned()]);
        assert!(report.orphan.is_empty());
    }

    #[test]
    fn verify_reports_orphan_for_extra_file() {
        let repo = temp_dir("orphan");
        write_file(&repo, "scripts/stray.lua");
        write_file(&repo, "scripts/sub/also-stray.lua");
        let report = verify(&LockFile::default(), &repo).expect("verify ok");
        assert_eq!(
            report.orphan,
            vec![
                "scripts/stray.lua".to_owned(),
                "scripts/sub/also-stray.lua".to_owned()
            ]
        );
        assert!(report.missing.is_empty());
    }

    #[test]
    fn verify_counts_config_d_fragment_as_owned() {
        let repo = temp_dir("fragment");
        write_file(&repo, "config.d/packages/a.conf");
        let mut lock = LockFile::default();
        lock.upsert(entry("a", &[], true));
        let report = verify(&lock, &repo).expect("verify ok");
        assert!(report.is_healthy(), "{report:?}");
    }

    #[test]
    fn verify_flags_fragment_without_lock_entry() {
        let repo = temp_dir("fragment-orphan");
        write_file(&repo, "config.d/packages/a.conf");
        let report = verify(&LockFile::default(), &repo).expect("verify ok");
        assert_eq!(report.orphan, vec!["config.d/packages/a.conf".to_owned()]);
    }

    #[test]
    fn dir_record_covers_the_whole_subtree() {
        let repo = temp_dir("dir-record");
        let mut lock = LockFile::default();
        lock.upsert(entry("a", &["scripts"], false));
        write_file(&repo, "scripts/a.lua");
        write_file(&repo, "scripts/sub/b.lua");
        let report = verify(&lock, &repo).expect("verify ok");
        assert!(report.is_healthy(), "{report:?}");
    }

    #[test]
    fn missing_dir_record_is_reported() {
        let repo = temp_dir("missing-dir");
        let mut lock = LockFile::default();
        lock.upsert(entry("a", &["shaders"], false));
        let report = verify(&lock, &repo).expect("verify ok");
        assert_eq!(report.missing, vec!["shaders".to_owned()]);
    }

    #[test]
    fn repair_refuses_without_yes_and_touches_nothing() {
        let repo = temp_dir("refuse");
        write_file(&repo, "scripts/stray.lua");
        let error = repair(&LockFile::default(), &repo, false).expect_err("refuses");
        assert!(
            matches!(error, RepairError::ConfirmRequired { .. }),
            "{error}"
        );
        assert!(repo.join("scripts/stray.lua").exists());
    }

    #[test]
    fn repair_with_yes_removes_only_orphans() {
        let repo = temp_dir("repair");
        let mut lock = LockFile::default();
        lock.upsert(entry("a", &["scripts/kept.lua"], false));
        write_file(&repo, "scripts/kept.lua");
        write_file(&repo, "scripts/stray.lua");
        write_file(&repo, "shaders/stray.glsl");

        let report = repair(&lock, &repo, true).expect("repair ok");
        assert_eq!(
            report.removed,
            vec![
                "scripts/stray.lua".to_owned(),
                "shaders/stray.glsl".to_owned()
            ]
        );
        assert!(report.missing.is_empty());
        assert!(repo.join("scripts/kept.lua").exists());
        assert!(!repo.join("scripts/stray.lua").exists());
        assert!(verify(&lock, &repo).expect("verify ok").is_healthy());
    }

    #[test]
    fn repair_reports_missing_files_without_deleting_them() {
        let repo = temp_dir("repair-missing");
        write_file(&repo, "scripts/stray.lua");
        let mut lock = LockFile::default();
        lock.upsert(entry("a", &["scripts/gone.lua"], false));
        let report = repair(&lock, &repo, true).expect("repair ok");
        assert_eq!(report.missing, vec!["scripts/gone.lua".to_owned()]);
        assert!(!repo.join("scripts/gone.lua").exists());
    }
}
