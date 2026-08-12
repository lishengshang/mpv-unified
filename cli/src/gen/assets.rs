//! Asset-directory copying for `gen`.
//!
//! The `--out` directory doubles as `mpv --config-dir`, which loads every
//! runtime asset (scripts, shaders, fonts, script-opts, ...) from that
//! single directory. Emitting only the generated conf files left the output
//! without any scripts — uosc and friends never loaded. This module copies
//! the repository-root asset directories verbatim (recursively), skips the
//! ones that do not exist, and reports per-directory file counts so the CLI
//! can print a summary instead of one line per file.
//!
//! Nothing here panics: every failure is a [`crate::gen::GenError`] with a
//! Chinese message, mirroring the rest of the `gen` pipeline.

use crate::gen::{GenError, GenOptions};
use std::fs;
use std::path::Path;

/// One asset directory copied (or planned, in dry-run mode) into the output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopiedAssetDir {
    /// Directory name, e.g. `scripts`.
    pub name: String,
    /// Number of files copied (or counted, in dry-run mode) recursively.
    pub file_count: usize,
}

/// Asset directories at the repository root, copied verbatim to the output.
/// Missing directories are skipped silently. `user/` and `config/` are
/// deliberately absent: they are the layered *sources* — the output
/// `mpv.conf` is the generated product, and `user/` holds private state
/// that must never leak into a shipped config.
const ASSET_DIRS: &[&str] = &[
    "scripts",
    "shaders",
    "fonts",
    "script-opts",
    "icc",
    "osc-style",
    "vs",
    "script-modules",
];

/// Copy every existing asset directory from `root` into `options.out`.
///
/// In dry-run mode nothing is written; the file counts are still computed
/// and reported so the planned output stays visible.
pub fn copy_assets(root: &Path, options: &GenOptions) -> Result<Vec<CopiedAssetDir>, GenError> {
    let mut copied = Vec::new();
    for name in ASSET_DIRS {
        let source = root.join(name);
        if !source.is_dir() {
            continue;
        }
        let target = options.out.join(name);
        let file_count = if options.dry_run {
            count_files(&source)?
        } else {
            copy_dir_recursive(&source, &target)?
        };
        copied.push(CopiedAssetDir {
            name: (*name).to_owned(),
            file_count,
        });
    }
    Ok(copied)
}

/// Recursively copy `source` into `target`, creating directories as needed;
/// returns the number of files copied.
fn copy_dir_recursive(source: &Path, target: &Path) -> Result<usize, GenError> {
    fs::create_dir_all(target).map_err(|source_error| GenError::Io {
        action: "创建输出目录",
        path: target.to_path_buf(),
        source: source_error,
    })?;
    let mut count = 0;
    for entry in read_entries(source)? {
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            count += copy_dir_recursive(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path).map_err(|source_error| GenError::Io {
                action: "复制",
                path: target_path,
                source: source_error,
            })?;
            count += 1;
        }
    }
    Ok(count)
}

/// Count the files under `dir` (recursively) without writing anything.
fn count_files(dir: &Path) -> Result<usize, GenError> {
    let mut count = 0;
    for entry in read_entries(dir)? {
        let path = entry.path();
        if path.is_dir() {
            count += count_files(&path)?;
        } else {
            count += 1;
        }
    }
    Ok(count)
}

/// Read one directory's entries, mapping any failure to a [`GenError`].
fn read_entries(dir: &Path) -> Result<Vec<fs::DirEntry>, GenError> {
    let entries = fs::read_dir(dir).map_err(|source| io_error(dir, source))?;
    let mut result = Vec::new();
    for entry in entries {
        result.push(entry.map_err(|source| io_error(dir, source))?);
    }
    Ok(result)
}

fn io_error(dir: &Path, source: std::io::Error) -> GenError {
    GenError::Io {
        action: "读取",
        path: dir.to_path_buf(),
        source,
    }
}
