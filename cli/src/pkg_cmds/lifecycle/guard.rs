//! The atomic-replacement guard for `pkg update`.
//!
//! The guard owns the old-file backup and the new-file copy, so a failure
//! anywhere in the replacement can be unwound completely: new files are
//! removed, every backed-up file (old files and pre-update content of
//! shared files) is restored.

use std::fs;
use std::path::{Path, PathBuf};

use pkg::manifest::Manifest;

use super::error::{LifecycleError, Result};
use super::lock::LockEntry;

/// Owns the old-file backup and the new-file copy, so a failure anywhere in
/// the replacement can be unwound completely.
pub(crate) struct ReplaceGuard {
    /// Backup directory; entries are prefixed `old-<i>` / `new-<i>` so the
    /// two sides never collide.
    backup_dir: PathBuf,
    old_files: Vec<String>,
    new_files: Vec<(String, String)>,
    repo_root: PathBuf,
}

impl ReplaceGuard {
    pub(crate) fn new(backup_root: &Path, new_manifest: &Manifest, repo_root: &Path) -> Self {
        Self {
            backup_dir: backup_root.join("files"),
            old_files: Vec::new(),
            new_files: new_manifest
                .files
                .iter()
                .map(|entry| (strip_root(&entry.dest).to_owned(), entry.src.clone()))
                .collect(),
            repo_root: repo_root.to_path_buf(),
        }
    }

    /// Move every existing old file into the backup directory.
    pub(crate) fn backup_existing(&mut self, installed: &LockEntry) -> Result<()> {
        fs::create_dir_all(&self.backup_dir)
            .map_err(|e| LifecycleError::io(format!("创建 {}", self.backup_dir.display()), e))?;
        for (index, file) in installed.files.iter().enumerate() {
            let path = self.repo_root.join(file);
            if !path.exists() {
                continue;
            }
            let backup = self.backup_dir.join(format!("old-{index}"));
            fs::rename(&path, &backup)
                .map_err(|e| LifecycleError::io(format!("备份 {}", path.display()), e))?;
            self.old_files.push(file.clone());
        }
        Ok(())
    }

    /// Copy the new files into place, backing up any destination that
    /// exists (shared files). Sources are validated first, so a failure
    /// never happens halfway through; a failure is unwound by the caller
    /// via [`Self::restore`].
    pub(crate) fn copy_new(&mut self, source_dir: &Path) -> Result<()> {
        for (_, src_rel) in &self.new_files {
            if !source_dir.join(src_rel).exists() {
                return Err(LifecycleError::io(
                    format!("源文件 {src_rel} 不存在"),
                    std::io::Error::other("source missing"),
                ));
            }
        }
        for (index, (file, src_rel)) in self.new_files.iter().enumerate() {
            let src = source_dir.join(src_rel);
            let dest = self.repo_root.join(file);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| LifecycleError::io(format!("创建 {}", parent.display()), e))?;
            }
            let backup = self.backup_dir.join(format!("new-{index}"));
            if dest.exists() {
                fs::rename(&dest, &backup)
                    .map_err(|e| LifecycleError::io(format!("备份 {}", dest.display()), e))?;
            }
            copy_one(&src, &dest)?;
        }
        Ok(())
    }

    /// Unwind: remove the new files, restore every backed-up file (new-side
    /// backups put the pre-update content of shared files back).
    pub(crate) fn restore(&self) -> Result<()> {
        let mut detail = String::new();
        for (index, (file, _)) in self.new_files.iter().enumerate() {
            let new_backup = self.backup_dir.join(format!("new-{index}"));
            let dest = self.repo_root.join(file);
            if new_backup.exists() {
                let _ = fs::remove_file(&dest);
                if let Err(source) = fs::rename(&new_backup, &dest) {
                    detail.push_str(&format!("{}: {source};", dest.display()));
                }
            } else if dest.exists() {
                if let Err(source) = fs::remove_file(&dest) {
                    detail.push_str(&format!("{}: {source};", dest.display()));
                }
            }
        }
        for (index, file) in self.old_files.iter().enumerate() {
            let old_backup = self.backup_dir.join(format!("old-{index}"));
            if !old_backup.exists() {
                continue;
            }
            let dest = self.repo_root.join(file);
            if let Err(source) = fs::rename(&old_backup, &dest) {
                detail.push_str(&format!("{}: {source};", dest.display()));
            }
        }
        if detail.is_empty() {
            Ok(())
        } else {
            Err(LifecycleError::RollbackFailed { detail })
        }
    }
}

fn copy_one(src: &Path, dest: &Path) -> Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dest)
            .map_err(|e| LifecycleError::io(format!("创建 {}", dest.display()), e))?;
        for item in fs::read_dir(src).map_err(|e| LifecycleError::io("读取目录", e))? {
            let item = item.map_err(|e| LifecycleError::io("读取目录条目", e))?;
            copy_one(&item.path(), &dest.join(item.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(src, dest).map_err(|e| {
            LifecycleError::io(format!("拷贝 {} → {}", src.display(), dest.display()), e)
        })?;
        Ok(())
    }
}

/// Strip the `~~/` prefix from a manifest dest path.
fn strip_root(dest: &str) -> &str {
    dest.strip_prefix("~~/").unwrap_or(dest)
}
