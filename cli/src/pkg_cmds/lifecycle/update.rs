//! The `pkg update` pipeline.
//!
//! For each requested installed package, the newest version is resolved from
//! the same source priority as install (pending git record → local
//! manifest → cached index). When the version differs (pending git sources
//! carry the `0.0.0` placeholder and always update), the replacement is
//! atomic: old files move to `cache_dir/backup/<name>-<version>-<n>/`, new
//! files are copied in, stale files the new version dropped are removed
//! (shared files never), and the lock entry is rewritten. Any failure
//! restores the backup — the package is left exactly as it was.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use pkg::deps;
use pkg::fetch::{self, HttpFetcher};
use pkg::index::Index;
use pkg::manifest::Manifest;

use super::cache::cache_manifest;
use super::copy::strip_root;
use super::error::{LifecycleError, Result};
use super::fragment::write_config_fragment;
use super::guard::ReplaceGuard;
use super::install::SourceKind;
use super::lock::{self, LockEntry, LockFile};
use super::pending;
use super::unique_suffix;

/// Options for one `pkg update` run.
#[derive(Debug, Clone)]
pub struct UpdateOptions {
    /// Package to update; `None` updates every installed package that has a
    /// resolvable source.
    pub name: Option<String>,
    pub repo_root: PathBuf,
    pub cache_dir: PathBuf,
}

/// One updated (or already-current) package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateItem {
    pub name: String,
    pub from: String,
    pub to: String,
    /// `true` when files actually changed; `false` when already current.
    pub changed: bool,
    /// Where the old version's files were preserved (cache backup dir, or
    /// the in-repo swap dir when persisting to the cache failed).
    pub backup: Option<String>,
}

/// Outcome of an update run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateReport {
    pub items: Vec<UpdateItem>,
}

/// Update `opts.name` (or every installed package when `None`).
///
/// # Errors
///
/// [`LifecycleError::NotInstalled`] when a named package is not in the
/// lock; [`LifecycleError`] on source resolution, validation, copy, or
/// lock failures. A failed replacement rolls back to the old version.
pub fn update(opts: &UpdateOptions) -> Result<UpdateReport> {
    let lock = lock::read(&opts.repo_root.join("packages.lock"))?;
    let targets = match &opts.name {
        Some(name) => {
            if lock.find(name).is_none() {
                return Err(LifecycleError::NotInstalled { name: name.clone() });
            }
            vec![name.clone()]
        }
        None => lock
            .packages
            .iter()
            .map(|entry| entry.name.clone())
            .collect(),
    };

    let mut report = UpdateReport::default();
    for name in targets {
        report.items.push(update_one(opts, &lock, &name)?);
    }
    Ok(report)
}

/// A resolved next version for one package.
enum Next {
    /// Version differs from the installed one (or a git source, which is
    /// always treated as newer): the manifest to install.
    Newer(Box<Manifest>, PathBuf, SourceKind),
    /// Same version: nothing to do.
    Current,
}

fn update_one(opts: &UpdateOptions, lock: &LockFile, name: &str) -> Result<UpdateItem> {
    let installed = lock
        .find(name)
        .ok_or_else(|| LifecycleError::NotInstalled {
            name: name.to_owned(),
        })?;
    let installed_version = installed.version.clone();

    let next = resolve_next(opts, installed, name)?;
    let Next::Newer(new_manifest, source_dir, source_kind) = next else {
        return Ok(UpdateItem {
            name: name.to_owned(),
            from: installed_version.clone(),
            to: installed_version,
            changed: false,
            backup: None,
        });
    };

    let installed_manifests = load_installed_manifests(lock, &opts.cache_dir)?;
    let installed_refs: Vec<&Manifest> = installed_manifests.iter().collect();
    let incoming = [new_manifest.as_ref()];
    deps::check_conflicts(&installed_refs, &incoming)?;
    // `check_file_conflicts` compares `~~/`-prefixed dests (as the manifest
    // stores them); the lock records stripped paths, so re-prefix here.
    let installed_paths: std::collections::HashMap<String, Vec<String>> = lock
        .file_map(Some(name))
        .into_iter()
        .map(|(pkg, paths)| {
            let prefixed = paths
                .iter()
                .map(|path| format!("~~/{path}"))
                .collect::<Vec<_>>();
            (pkg, prefixed)
        })
        .collect();
    deps::check_file_conflicts(&installed_paths, &new_manifest)?;

    let swap_dir = opts
        .repo_root
        .join(format!(".pkg-backup-{}", unique_suffix()));
    let mut guard = ReplaceGuard::new(&swap_dir, &new_manifest, &opts.repo_root);
    guard.backup_existing(installed)?;

    let outcome = (|| -> Result<()> {
        guard.copy_new(&source_dir)?;
        let _ = write_config_fragment(&opts.repo_root, &new_manifest)?;
        cache_manifest(&opts.cache_dir, &new_manifest)?;
        drop_stale_files(lock, name, &new_manifest, &opts.repo_root)?;

        let mut lock = lock.clone();
        lock.upsert(LockEntry {
            name: name.to_owned(),
            version: new_manifest.version.to_string(),
            files: new_manifest
                .files
                .iter()
                .map(|entry| strip_root(&entry.dest).to_owned())
                .collect(),
            config_d: !new_manifest.config.is_empty(),
        });
        lock::write(&opts.repo_root.join("packages.lock"), &lock)?;
        Ok(())
    })();

    if let Err(error) = outcome {
        let rollback = guard.restore();
        let _ = fs::remove_dir_all(&swap_dir);
        if source_kind == SourceKind::PendingGit {
            let _ = fs::remove_dir_all(&source_dir);
        }
        rollback?;
        return Err(error);
    }

    let backup = persist_backup(&swap_dir, &opts.cache_dir, name, &installed_version);
    if source_kind == SourceKind::PendingGit {
        let _ = fs::remove_dir_all(&source_dir);
    }

    Ok(UpdateItem {
        name: name.to_owned(),
        from: installed_version,
        to: new_manifest.version.to_string(),
        changed: true,
        backup,
    })
}

/// Move the in-repo swap backup into the cache's backup area
/// (`cache_dir/backup/<name>-<old-version>-<n>/`). The rename works on the
/// same filesystem; a cross-device cache falls back to a recursive copy.
/// When neither works the swap dir stays in the repo and its path is
/// returned — the old version is never silently destroyed.
fn persist_backup(swap_dir: &Path, cache_dir: &Path, name: &str, version: &str) -> Option<String> {
    let target = cache_dir
        .join("backup")
        .join(format!("{name}-{version}-{}", unique_suffix()));
    if fs::rename(swap_dir, &target).is_ok() {
        return Some(target.display().to_string());
    }
    if copy_tree(swap_dir, &target).is_ok() {
        let _ = fs::remove_dir_all(swap_dir);
        return Some(target.display().to_string());
    }
    Some(swap_dir.display().to_string())
}

/// Recursively copy `src` into `dest` (used for cross-device backups).
fn copy_tree(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)
        .map_err(|e| LifecycleError::io(format!("创建 {}", dest.display()), e))?;
    for item in fs::read_dir(src).map_err(|e| LifecycleError::io("读取目录", e))? {
        let item = item.map_err(|e| LifecycleError::io("读取目录条目", e))?;
        let child_dest = dest.join(item.file_name());
        if item
            .file_type()
            .map_err(|e| LifecycleError::io("读取条目类型", e))?
            .is_dir()
        {
            copy_tree(&item.path(), &child_dest)?;
        } else {
            fs::copy(item.path(), &child_dest).map_err(|e| {
                LifecycleError::io(
                    format!("拷贝 {} → {}", item.path().display(), child_dest.display()),
                    e,
                )
            })?;
        }
    }
    Ok(())
}

/// Resolve the next version: pending git record (always newer), local
/// manifest or index entry (version-compared against the installed one).
fn resolve_next(opts: &UpdateOptions, installed: &LockEntry, name: &str) -> Result<Next> {
    let pending_path = opts
        .repo_root
        .join("packages")
        .join("pending")
        .join(format!("{name}.yaml"));
    if pending_path.is_file() {
        let record = pending::PendingRecord::parse_file(&pending_path)?;
        let resolved = pending::resolve(&record, &opts.cache_dir)?;
        return Ok(Next::Newer(
            Box::new(resolved.manifest),
            resolved.clone_dir,
            SourceKind::PendingGit,
        ));
    }

    let local_path = opts.repo_root.join("packages").join(format!("{name}.yaml"));
    if local_path.is_file() {
        let manifest =
            Manifest::parse_file(&local_path).map_err(|error| LifecycleError::Manifest {
                name: name.to_owned(),
                error,
            })?;
        return versioned(
            installed,
            Box::new(manifest),
            opts.repo_root.join("packages"),
            SourceKind::LocalManifest,
        );
    }

    let index_path = opts.cache_dir.join("index.json");
    if index_path.is_file() {
        let text = fs::read_to_string(&index_path)
            .map_err(|e| LifecycleError::io(format!("读取 {}", index_path.display()), e))?;
        let index = Index::parse(&text)?;
        if let Some(entry) = index.find(name) {
            let archive = fetch::fetch_package(entry, &HttpFetcher, &opts.cache_dir)?;
            let source_dir = opts.cache_dir.join("packages").join(format!(
                "{}-{}",
                archive.manifest.name, archive.manifest.version
            ));
            return versioned(
                installed,
                Box::new(archive.manifest),
                source_dir,
                SourceKind::Index,
            );
        }
    }

    Err(LifecycleError::PackageNotFound {
        name: name.to_owned(),
    })
}

/// Version-compare: same version → `Current`, else `Newer`.
fn versioned(
    installed: &LockEntry,
    manifest: Box<Manifest>,
    source_dir: PathBuf,
    kind: SourceKind,
) -> Result<Next> {
    let same = semver::Version::parse(&installed.version).is_ok_and(|v| v == manifest.version);
    if same {
        Ok(Next::Current)
    } else {
        Ok(Next::Newer(manifest, source_dir, kind))
    }
}

/// Load cached manifests for every installed package (conflict checks).
fn load_installed_manifests(lock: &LockFile, cache_dir: &Path) -> Result<Vec<Manifest>> {
    let mut out = Vec::new();
    for entry in &lock.packages {
        let path = cache_dir
            .join("packages")
            .join(format!("{}-{}", entry.name, entry.version))
            .join("package.yaml");
        if let Ok(manifest) = Manifest::parse_file(&path) {
            out.push(manifest);
        }
    }
    Ok(out)
}

/// Delete files the old version owned that the new version no longer lists,
/// unless another package shares them.
fn drop_stale_files(
    lock: &LockFile,
    name: &str,
    new_manifest: &Manifest,
    root: &Path,
) -> Result<()> {
    let new_set: HashSet<String> = new_manifest
        .files
        .iter()
        .map(|entry| strip_root(&entry.dest).to_owned())
        .collect();
    let Some(installed) = lock.find(name) else {
        return Ok(());
    };
    for file in &installed.files {
        if new_set.contains(file) || !lock.owners_of(file, Some(name)).is_empty() {
            continue;
        }
        let path = root.join(file);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(LifecycleError::io(
                    format!("删除 {}", path.display()),
                    source,
                ));
            }
        }
    }
    Ok(())
}
