//! The `pkg install` pipeline.
//!
//! Source resolution order: `packages/pending/<name>.yaml` (T12 git record,
//! expanded by clone + whitelist/blacklist) → `packages/<name>.yaml` (local
//! static manifest) → cached `index.json` (T13, fetched via GitHub
//! Releases). Then: already-installed guard → platform check (mismatch is
//! a skip, not an error) → dependency and conflict validation (T15) → file
//! copy with backup/rollback → `config.d/packages/<name>.conf` fragment →
//! manifest cached for later conflict checks → `packages.lock` updated
//! atomically.
//!
//! File copies are protected two ways: every source is validated to exist
//! before the first byte is written, and every destination that already
//! exists is moved aside to a backup directory first — a mid-copy failure
//! restores the backups and removes the freshly copied files.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use pkg::deps;
use pkg::fetch::{self, HttpFetcher};
use pkg::index::Index;
use pkg::manifest::{Manifest, Platform as PkgPlatform};

use super::cache::cache_manifest;
use super::copy::{copy_files, restore_all, strip_root};
use super::error::{LifecycleError, Result};
use super::fragment::write_config_fragment;
use super::lock::{self, LockFile};
use super::pending::{self, ResolvedPending};
use super::unique_suffix;
use core::platform::Platform;

/// Where an installed manifest came from (drives the report wording).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// `packages/<name>.yaml` — local static manifest.
    LocalManifest,
    /// `packages/pending/<name>.yaml` — git source expanded by clone.
    PendingGit,
    /// Cached `index.json` entry fetched from GitHub Releases.
    Index,
}

/// Options for one `pkg install` run.
#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub name: String,
    /// Repository root: `~~/` expands here; files are copied under it and
    /// `packages.lock` lives at its root.
    pub repo_root: PathBuf,
    /// Package cache directory (`index.json`, fetched archives, clones).
    pub cache_dir: PathBuf,
    /// Target platform for the platform-mismatch check.
    pub platform: Platform,
}

/// What happened after an install attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallOutcome {
    Installed(InstallReport),
    /// Platform mismatch: skipped with a notice, not an error.
    SkippedPlatform {
        name: String,
        package_platform: PkgPlatform,
        target: Platform,
    },
}

/// A successful install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReport {
    pub name: String,
    pub version: String,
    pub source: SourceKind,
    pub files: usize,
    pub config_written: bool,
}

/// Run the install pipeline for `opts.name`.
///
/// # Errors
///
/// [`LifecycleError`] on every failure (missing package, invalid manifest,
/// missing dependency, conflict, copy failure, lock failure). Nothing
/// panics; a mid-copy failure rolls back.
pub fn install(opts: &InstallOptions) -> Result<InstallOutcome> {
    let lock = lock::read(&opts.repo_root.join("packages.lock"))?;
    let mut resolved = resolve_source(opts)?;
    let manifest = resolved.manifest().clone();

    if let Some(entry) = lock.find(&opts.name) {
        return Err(LifecycleError::AlreadyInstalled {
            name: opts.name.clone(),
            version: entry.version.clone(),
        });
    }
    if manifest.platform_mismatch(to_pkg_platform(opts.platform)) {
        return Ok(InstallOutcome::SkippedPlatform {
            name: opts.name.clone(),
            package_platform: manifest.platform,
            target: opts.platform,
        });
    }

    let installed_names: HashSet<String> = lock
        .packages
        .iter()
        .map(|entry| entry.name.clone())
        .collect();
    let missing: Vec<String> = manifest
        .requires
        .iter()
        .filter(|required| !installed_names.contains(*required))
        .cloned()
        .collect();
    if !missing.is_empty() {
        return Err(LifecycleError::MissingDeps {
            required_by: opts.name.clone(),
            missing,
        });
    }

    let installed_manifests = load_installed_manifests(&lock, &opts.cache_dir)?;
    let installed_refs: Vec<&Manifest> = installed_manifests.iter().collect();
    let incoming = [&manifest];
    deps::check_conflicts(&installed_refs, &incoming)?;

    deps::check_file_conflicts(&lock.file_map(None), &manifest)?;

    let (source_dir, source_kind) = resolved.source_and_kind(opts);
    let backup_dir = opts
        .repo_root
        .join(format!(".pkg-backup-{}", unique_suffix()));
    copy_files(&source_dir, &manifest, &opts.repo_root, &backup_dir)?;

    let outcome: std::result::Result<bool, LifecycleError> = (|| {
        let config_written = write_config_fragment(&opts.repo_root, &manifest)?;
        cache_manifest(&opts.cache_dir, &manifest)?;
        let mut lock = lock;
        lock.upsert(super::lock::LockEntry {
            name: manifest.name.clone(),
            version: manifest.version.to_string(),
            files: manifest
                .files
                .iter()
                .map(|entry| strip_root(&entry.dest).to_owned())
                .collect(),
            config_d: config_written,
        });
        lock::write(&opts.repo_root.join("packages.lock"), &lock)?;
        Ok(config_written)
    })();

    let config_written = match outcome {
        Ok(config_written) => config_written,
        Err(error) => {
            if let Resolved::Pending(resolved) = &mut resolved {
                let _ = fs::remove_dir_all(&resolved.clone_dir);
            }
            restore_all(&backup_dir, &opts.repo_root, &manifest.files)?;
            return Err(error);
        }
    };

    if let Resolved::Pending(resolved) = &mut resolved {
        let _ = fs::remove_dir_all(&resolved.clone_dir);
    }
    let _ = fs::remove_dir_all(&backup_dir);

    Ok(InstallOutcome::Installed(InstallReport {
        name: manifest.name.clone(),
        version: manifest.version.to_string(),
        source: source_kind,
        files: manifest.files.len(),
        config_written,
    }))
}

/// A resolved installable: owns its manifest and knows its source tree.
enum Resolved {
    Local(Manifest),
    Pending(ResolvedPending),
    Fetched(Manifest),
}

impl Resolved {
    fn manifest(&self) -> &Manifest {
        match self {
            Self::Local(manifest) | Self::Fetched(manifest) => manifest,
            Self::Pending(resolved) => &resolved.manifest,
        }
    }

    /// The directory holding the package's source files, plus its kind.
    fn source_and_kind(&self, opts: &InstallOptions) -> (PathBuf, SourceKind) {
        match self {
            Self::Local(_) => (opts.repo_root.join("packages"), SourceKind::LocalManifest),
            Self::Pending(resolved) => (resolved.clone_dir.clone(), SourceKind::PendingGit),
            Self::Fetched(manifest) => (
                opts.cache_dir
                    .join("packages")
                    .join(format!("{}-{}", manifest.name, manifest.version)),
                SourceKind::Index,
            ),
        }
    }
}

/// Resolve the source for `name`: pending record → local manifest → index.
fn resolve_source(opts: &InstallOptions) -> Result<Resolved> {
    let pending_path = opts
        .repo_root
        .join("packages")
        .join("pending")
        .join(format!("{}.yaml", opts.name));
    if pending_path.is_file() {
        let record = pending::PendingRecord::parse_file(&pending_path)?;
        return pending::resolve(&record, &opts.cache_dir).map(Resolved::Pending);
    }

    let local_path = opts
        .repo_root
        .join("packages")
        .join(format!("{}.yaml", opts.name));
    if local_path.is_file() {
        let manifest =
            Manifest::parse_file(&local_path).map_err(|error| LifecycleError::Manifest {
                name: opts.name.clone(),
                error,
            })?;
        return Ok(Resolved::Local(manifest));
    }

    let index_path = opts.cache_dir.join("index.json");
    if index_path.is_file() {
        let text = fs::read_to_string(&index_path)
            .map_err(|e| LifecycleError::io(format!("读取 {}", index_path.display()), e))?;
        let index = Index::parse(&text)?;
        if let Some(entry) = index.find(&opts.name) {
            let archive = fetch::fetch_package(entry, &HttpFetcher, &opts.cache_dir)?;
            return Ok(Resolved::Fetched(archive.manifest));
        }
    }

    Err(LifecycleError::PackageNotFound {
        name: opts.name.clone(),
    })
}

/// Load cached manifests for every installed package, so conflict checks
/// can see the installed `conflicts` lists. Packages without a cached
/// manifest (e.g. lock files written before caching existed) are skipped —
/// their conflicts are unknowable offline.
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

/// Map the CLI's core platform onto the manifest platform enum.
fn to_pkg_platform(platform: Platform) -> PkgPlatform {
    match platform {
        Platform::Linux => PkgPlatform::Linux,
        Platform::Windows => PkgPlatform::Windows,
        Platform::MacOS => PkgPlatform::MacOS,
    }
}
