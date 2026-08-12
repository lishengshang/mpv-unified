//! `pkg install` / `pkg uninstall` / `pkg update` / `pkg update-index`:
//! the package lifecycle commands (T14).
//!
//! Install resolves a source in priority order — `packages/pending/<name>.yaml`
//! (git record expanded by clone + whitelist/blacklist), `packages/<name>.yaml`
//! (local manifest), cached `index.json` (fetched from GitHub Releases) —
//! validates platform/dependencies/conflicts, copies files with backup and
//! rollback, writes the `config.d/packages/<name>.conf` fragment, and
//! records the result in `packages.lock` (minimal T14 format; T16 takes
//! over the file). Uninstall removes exclusively-owned files (shared paths
//! are kept while another package still lists them) and drops the lock
//! entry. Update version-compares and atomically replaces with a backup in
//! `~/.cache/mpv-config/backup/`.
//!
//! Everything is offline-testable: roots and cache dirs are parameters,
//! never globals.

pub mod cache;
pub mod copy;
pub mod error;
pub mod fragment;
pub mod guard;
pub mod install;
pub mod lock;
pub mod pending;
pub mod uninstall;
pub mod update;

use std::fmt;
use std::path::PathBuf;

use pkg::fetch::{self, HttpFetcher};

pub use error::{default_cache_dir, LifecycleError};
pub use install::{install, InstallOptions, InstallOutcome, InstallReport, SourceKind};
pub use uninstall::{uninstall, UninstallOptions, UninstallReport};
pub use update::{update, UpdateItem, UpdateOptions, UpdateReport};

pub(crate) use pending::unique_suffix;

impl fmt::Display for InstallReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let source = match self.source {
            SourceKind::LocalManifest => "本地 manifest (packages/)",
            SourceKind::PendingGit => "git 源 (packages/pending/)",
            SourceKind::Index => "索引 (index.json + GitHub Releases)",
        };
        write!(
            f,
            "已安装 {} v{}(来源:{source};{} 个文件{})",
            self.name,
            self.version,
            self.files,
            if self.config_written {
                ";已写入 config.d 片段"
            } else {
                ""
            }
        )
    }
}

impl fmt::Display for UninstallReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "已卸载 {} v{}(删除 {} 个文件,保留 {} 个共享文件{})",
            self.name,
            self.version,
            self.files_removed,
            self.files_shared,
            if self.config_removed {
                ";已移除 config.d 片段"
            } else {
                ""
            }
        )
    }
}

impl fmt::Display for UpdateReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut lines = Vec::new();
        for item in &self.items {
            if item.changed {
                let backup = item
                    .backup
                    .as_ref()
                    .map(|path| format!("(旧版备份于 {path})"))
                    .unwrap_or_default();
                lines.push(format!(
                    "已更新 {} v{} → v{}{}",
                    item.name, item.from, item.to, backup
                ));
            } else {
                lines.push(format!("{} 已是最新版本 v{}", item.name, item.to));
            }
        }
        if lines.is_empty() {
            write!(f, "没有已安装的包可更新")?;
        } else {
            for line in &lines {
                writeln!(f, "{line}")?;
            }
        }
        Ok(())
    }
}

/// Options for `pkg update-index`.
#[derive(Debug, Clone)]
pub struct UpdateIndexOptions {
    /// Remote index URL (defaults to the official index repository).
    pub index_url: String,
    /// Cache directory to store `index.json` in.
    pub cache_dir: PathBuf,
}

/// Fetch and validate the remote `index.json` into the cache directory.
/// The cached document is only replaced after the new one validates.
///
/// # Errors
///
/// [`LifecycleError`] on transport, validation, or filesystem failures.
pub fn update_index(opts: &UpdateIndexOptions) -> Result<PathBuf, LifecycleError> {
    let path = fetch::update_index(&HttpFetcher, &opts.index_url, &opts.cache_dir)?;
    Ok(path)
}
