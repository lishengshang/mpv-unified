//! `packages.lock` read/write for the package lifecycle (T14 format, T16
//! ownership).
//!
//! Format (T16 contract, see the plan's notes):
//!
//! ```yaml
//! packages:
//!   - name: foo
//!     version: 1.0.0
//!     files:
//!       - scripts/foo.lua
//!     config_d: true
//! ```
//!
//! `files` are repository-relative paths (`~~/scripts/x.lua` becomes
//! `scripts/x.lua`); `config_d` records whether a
//! `config.d/packages/<name>.conf` fragment was written. The file is git
//! tracked (it is the dependency lock: "package manifest is the lock").
//!
//! [`verify`] and [`repair`] live in [`lock::verify`]; this module owns the
//! schema plus atomic read/write (tmp + rename, never a torn document). A
//! missing lock file reads as an empty lock; a *corrupt* file is a typed
//! [`LockError`] — never a panic.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub mod verify;

/// Header prepended to every written lock file (kept comment-only so the
/// document still parses as YAML).
const LOCK_HEADER: &str = "\
# packages.lock - 由 `mpv-config pkg` 管理(T16 verify/repair 可校验一致性)。
# 本文件进入 git 跟踪;删除其中条目后再 `pkg install` 可重建。
";

/// One installed package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockEntry {
    pub name: String,
    pub version: String,
    /// Repository-relative installed file paths (`scripts/foo.lua`).
    pub files: Vec<String>,
    /// Whether a `config.d/packages/<name>.conf` fragment was written.
    #[serde(default)]
    pub config_d: bool,
}

/// The whole lock document.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LockFile {
    pub packages: Vec<LockEntry>,
}

impl LockFile {
    /// Look up an entry by package name.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&LockEntry> {
        self.packages.iter().find(|entry| entry.name == name)
    }

    /// Every package (other than `except`, when given) that records `path`.
    #[must_use]
    pub fn owners_of(&self, path: &str, except: Option<&str>) -> Vec<String> {
        self.packages
            .iter()
            .filter(|entry| Some(entry.name.as_str()) != except)
            .filter(|entry| entry.files.iter().any(|f| f == path))
            .map(|entry| entry.name.clone())
            .collect()
    }

    /// Files recorded by every package except `except`.
    #[must_use]
    pub fn file_map(&self, except: Option<&str>) -> std::collections::HashMap<String, Vec<String>> {
        let mut map = std::collections::HashMap::new();
        for entry in &self.packages {
            if Some(entry.name.as_str()) == except {
                continue;
            }
            map.insert(entry.name.clone(), entry.files.clone());
        }
        map
    }

    /// Replace (or append) the entry for `name` with `entry`; the entry's
    /// name is taken from `entry`, which must equal `name`.
    pub fn upsert(&mut self, entry: LockEntry) {
        if let Some(slot) = self.packages.iter_mut().find(|e| e.name == entry.name) {
            *slot = entry;
        } else {
            self.packages.push(entry);
        }
    }

    /// Remove the entry for `name`; returns whether one was removed.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.packages.len();
        self.packages.retain(|entry| entry.name != name);
        self.packages.len() != before
    }
}

/// Lock file read/write failures.
#[derive(Debug)]
pub enum LockError {
    /// The file exists but is not a valid lock document.
    Corrupt { path: PathBuf, message: String },
    /// Filesystem failure during read/write.
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Corrupt { path, message } => write!(
                f,
                "{} 解析失败:{message}(请运行 `mpv-config pkg repair` 或删除后重装)",
                path.display()
            ),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for LockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Corrupt { .. } => None,
        }
    }
}

/// Read the lock at `path`; a missing file yields an empty lock.
///
/// # Errors
///
/// [`LockError::Corrupt`] on malformed content, [`LockError::Io`] on
/// filesystem failures.
pub fn read(path: &Path) -> Result<LockFile, LockError> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(LockFile::default()),
        Err(source) => {
            return Err(LockError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    serde_yaml::from_str(&raw).map_err(|error| LockError::Corrupt {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

/// Atomically write the lock: serialize, write a temp file next to `path`,
/// then rename over the target (never a torn document).
///
/// # Errors
///
/// [`LockError::Io`] on any filesystem failure.
pub fn write(path: &Path, lock: &LockFile) -> Result<(), LockError> {
    let body = serde_yaml::to_string(lock).map_err(|error| LockError::Io {
        path: path.to_path_buf(),
        source: io::Error::other(format!("serialize lock: {error}")),
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| LockError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let tmp = path.with_extension(format!("lock.tmp{}", std::process::id()));
    fs::write(&tmp, format!("{LOCK_HEADER}{body}")).map_err(|source| LockError::Io {
        path: tmp.clone(),
        source,
    })?;
    fs::rename(&tmp, path).map_err(|source| LockError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "mpv-config-lock-{label}-{}",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn entry(name: &str, files: &[&str]) -> LockEntry {
        LockEntry {
            name: name.to_owned(),
            version: "1.0.0".to_owned(),
            files: files.iter().map(|s| s.to_string()).collect(),
            config_d: true,
        }
    }

    #[test]
    fn missing_lock_reads_as_empty() {
        let path = temp_path("missing");
        let lock = read(&path).expect("missing lock is not an error");
        assert!(lock.packages.is_empty());
    }

    #[test]
    fn write_then_read_roundtrips() {
        let path = temp_path("roundtrip");
        let mut lock = LockFile::default();
        lock.upsert(entry("a", &["scripts/a.lua"]));
        lock.upsert(entry("b", &["scripts/b.lua", "scripts/shared.lua"]));
        write(&path, &lock).expect("write ok");
        let back = read(&path).expect("read ok");
        assert_eq!(back, lock);
    }

    #[test]
    fn corrupt_lock_is_a_typed_error_not_a_panic() {
        let path = temp_path("corrupt");
        fs::write(&path, "packages: [broken").expect("write garbage");
        let error = read(&path).expect_err("corrupt lock fails");
        assert!(matches!(error, LockError::Corrupt { .. }), "{error}");
    }

    #[test]
    fn owners_of_finds_every_package_except_the_excluded() {
        let mut lock = LockFile::default();
        lock.upsert(entry("a", &["scripts/shared.lua"]));
        lock.upsert(entry("b", &["scripts/shared.lua", "scripts/b.lua"]));
        let owners = lock.owners_of("scripts/shared.lua", None);
        assert_eq!(owners, vec!["a".to_owned(), "b".to_owned()]);
        let after_uninstall_b = lock.owners_of("scripts/shared.lua", Some("b"));
        assert_eq!(after_uninstall_b, vec!["a".to_owned()]);
    }

    #[test]
    fn file_map_excludes_one_package() {
        let mut lock = LockFile::default();
        lock.upsert(entry("a", &["scripts/a.lua"]));
        lock.upsert(entry("b", &["scripts/b.lua"]));
        let map = lock.file_map(Some("a"));
        assert!(!map.contains_key("a"));
        assert_eq!(map["b"], vec!["scripts/b.lua".to_owned()]);
    }

    #[test]
    fn upsert_replaces_and_remove_drops() {
        let mut lock = LockFile::default();
        lock.upsert(entry("a", &["scripts/old.lua"]));
        lock.upsert(entry("a", &["scripts/new.lua"]));
        assert_eq!(
            lock.find("a").expect("a present").files,
            vec!["scripts/new.lua"]
        );
        assert!(lock.remove("a"));
        assert!(!lock.remove("a"));
    }
}
