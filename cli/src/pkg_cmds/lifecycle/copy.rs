//! File-copy machinery for `pkg install` with backup and rollback.
//!
//! Every source is validated to exist before the first byte is written, and
//! every destination that already exists is moved aside to the backup
//! directory first — a mid-copy failure restores the backups and removes
//! the freshly created files.

use std::fs;
use std::path::Path;

use pkg::manifest::{FileEntry, Manifest};

use super::error::{LifecycleError, Result};

/// Copy every file entry from `source_dir` into the repo root, moving
/// pre-existing destinations aside into `backup_dir` first so a mid-copy
/// failure can restore them.
pub(crate) fn copy_files(
    source_dir: &Path,
    manifest: &Manifest,
    root: &Path,
    backup_dir: &Path,
) -> Result<()> {
    for entry in &manifest.files {
        if !source_dir.join(&entry.src).exists() {
            return Err(LifecycleError::io(
                format!("包 \"{}\" 的源文件 {} 不存在", manifest.name, entry.src),
                std::io::Error::other("source missing"),
            ));
        }
    }

    fs::create_dir_all(backup_dir)
        .map_err(|e| LifecycleError::io(format!("创建 {}", backup_dir.display()), e))?;
    for (index, entry) in manifest.files.iter().enumerate() {
        let dest = root.join(strip_root(&entry.dest));
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| LifecycleError::io(format!("创建 {}", parent.display()), e))?;
        }
        let backup_path = backup_dir.join(index.to_string());
        if dest.exists() {
            fs::rename(&dest, &backup_path)
                .map_err(|e| LifecycleError::io(format!("备份 {}", dest.display()), e))?;
        }
        if let Err(error) = copy_one(&source_dir.join(&entry.src), &dest) {
            restore_all(backup_dir, root, &manifest.files)?;
            return Err(error);
        }
    }
    Ok(())
}

/// Restore every destination touched by the copy of `files`: a backup file
/// moves back over its destination, a freshly created destination is
/// removed. Idempotent for already-restored entries.
pub(crate) fn restore_all(backup_dir: &Path, root: &Path, files: &[FileEntry]) -> Result<()> {
    for (index, entry) in files.iter().enumerate() {
        let dest = root.join(strip_root(&entry.dest));
        let backup = backup_dir.join(index.to_string());
        let result = if backup.exists() {
            let _ = fs::remove_file(&dest);
            fs::rename(&backup, &dest)
        } else if dest.exists() {
            fs::remove_file(&dest)
        } else {
            continue;
        };
        if let Err(source) = result {
            return Err(LifecycleError::RollbackFailed {
                detail: format!("{}: {source}", dest.display()),
            });
        }
    }
    Ok(())
}

/// Copy a file or, when `src` is a directory, its whole subtree.
fn copy_one(src: &Path, dest: &Path) -> Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dest)
            .map_err(|e| LifecycleError::io(format!("创建 {}", dest.display()), e))?;
        for item in fs::read_dir(src).map_err(|e| LifecycleError::io("读取目录", e))? {
            let item = item.map_err(|e| LifecycleError::io("读取目录条目", e))?;
            let child_dest = dest.join(item.file_name());
            copy_one(&item.path(), &child_dest)?;
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
pub(crate) fn strip_root(dest: &str) -> &str {
    dest.strip_prefix("~~/").unwrap_or(dest)
}
