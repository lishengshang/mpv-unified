//! The `pkg uninstall` pipeline.
//!
//! The lock file is the source of truth for what a package owns. Every
//! recorded file is deleted only when it is the last user — a path also
//! recorded by another package is shared and kept (the shared-file rule).
//! The `config.d/packages/<name>.conf` fragment is removed when the lock
//! says one was written, then the lock entry is dropped atomically.

use std::fs;
use std::path::PathBuf;

use super::error::{LifecycleError, Result};
use super::lock;

/// Options for one `pkg uninstall` run.
#[derive(Debug, Clone)]
pub struct UninstallOptions {
    pub name: String,
    /// Repository root holding the installed files and `packages.lock`.
    pub repo_root: PathBuf,
}

/// What was removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UninstallReport {
    pub name: String,
    pub version: String,
    /// Files deleted (owned exclusively by this package).
    pub files_removed: usize,
    /// Files kept because another package still uses them.
    pub files_shared: usize,
    /// Whether the `config.d/packages/<name>.conf` fragment was removed.
    pub config_removed: bool,
}

/// Uninstall `opts.name`: delete its exclusively-owned files, keep shared
/// ones, drop its config fragment and lock entry.
///
/// # Errors
///
/// [`LifecycleError::NotInstalled`] when the package is not in the lock;
/// [`LifecycleError`] on filesystem or lock failures. A file that is
/// already missing on disk counts as removed (the lock said it existed).
pub fn uninstall(opts: &UninstallOptions) -> Result<UninstallReport> {
    let lock_path = opts.repo_root.join("packages.lock");
    let mut lock = lock::read(&lock_path)?;
    let entry = lock
        .find(&opts.name)
        .cloned()
        .ok_or_else(|| LifecycleError::NotInstalled {
            name: opts.name.clone(),
        })?;

    let mut removed = 0;
    let mut shared = 0;
    for file in &entry.files {
        let other_owners = lock.owners_of(file, Some(&opts.name));
        if !other_owners.is_empty() {
            shared += 1;
            continue;
        }
        let path = opts.repo_root.join(file);
        match fs::remove_file(&path) {
            Ok(()) => removed += 1,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => removed += 1,
            Err(source) => {
                return Err(LifecycleError::io(
                    format!("删除 {}", path.display()),
                    source,
                ));
            }
        }
    }

    let config_removed = if entry.config_d {
        let fragment = opts
            .repo_root
            .join("config.d")
            .join("packages")
            .join(format!("{}.conf", opts.name));
        match fs::remove_file(&fragment) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(source) => {
                return Err(LifecycleError::io(
                    format!("删除 {}", fragment.display()),
                    source,
                ));
            }
        }
    } else {
        false
    };

    lock.remove(&opts.name);
    lock::write(&lock_path, &lock)?;

    Ok(UninstallReport {
        name: opts.name.clone(),
        version: entry.version,
        files_removed: removed,
        files_shared: shared,
        config_removed,
    })
}
