//! Package catalog: merge the four sources a store page shows — installed
//! (`packages.lock`), pending git records (`packages/pending/*.yaml`), local
//! manifests (`packages/*.yaml`), and the cached remote `index.json` — into
//! one sorted, display-ready list (T20).
//!
//! This module is pure: callers parse their sources (the `cli` crate owns
//! pending-record parsing; the Tauri shell owns the filesystem layout) and
//! hand the raw values in. The `Status` derivation is the contract the UI
//! renders badges from:
//!
//! - installed + pending record → [`Status::Updatable`] (a git source is
//!   always treated as newer)
//! - installed + local manifest with a different version → [`Status::Updatable`]
//! - installed → [`Status::Installed`]
//! - pending only → [`Status::Pending`]
//! - index/local only → [`Status::Available`]
//!
//! Rows are sorted by name; rows without any source (an installed package
//! whose index entry disappeared) still appear so the user can uninstall.

use serde::Serialize;

use crate::index::PackageEntry;
use crate::manifest::Manifest;

/// Where a catalog row's installable version comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RowSource {
    /// `packages/pending/<name>.yaml` — git source, install = clone.
    PendingGit,
    /// `packages/<name>.yaml` — local static manifest.
    LocalManifest,
    /// Cached `index.json` entry (GitHub Releases).
    Index,
}

/// Store status of one catalog row (the UI badge contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Status {
    /// Not installed, an installable version exists.
    Available,
    /// Installed; no known newer version.
    Installed,
    /// Installed, a newer version is known (git source or version bump).
    Updatable,
    /// A pending git record exists but the package is not installed.
    Pending,
}

/// A pending git record reduced to the fields the catalog needs.
#[derive(Debug, Clone)]
pub struct PendingMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    pub git: String,
}

/// One displayable package row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogEntry {
    pub name: String,
    pub description: String,
    /// Version recorded in `packages.lock`, when installed.
    pub installed_version: Option<String>,
    /// Newest version the catalog knows about (`"git"` for pending sources).
    pub available_version: Option<String>,
    /// Whether a pending git record exists for this package.
    pub pending: bool,
    pub status: Status,
    /// Installed file count from the lock (a simple size metric).
    pub file_count: usize,
    /// The install source that drives this row.
    pub source: RowSource,
    /// GitHub `owner/repo` for index entries; the clone URL for pending git
    /// records (both render as the row's 来源 line).
    pub repo: Option<String>,
}

/// Merge the four sources into a sorted catalog.
///
/// Later sources never overwrite an earlier row's identity: a pending
/// record shadows a local manifest shadows an index entry (matching the
/// install source priority). Installed-only packages with no source at all
/// keep their row so uninstall stays possible.
pub fn build(
    lock: &crate::lock::LockFile,
    pending: &[PendingMeta],
    local: &[Manifest],
    index: &[PackageEntry],
) -> Vec<CatalogEntry> {
    let mut rows: Vec<CatalogEntry> = Vec::new();

    for entry in &lock.packages {
        rows.push(CatalogEntry {
            name: entry.name.clone(),
            description: String::new(),
            installed_version: Some(entry.version.clone()),
            available_version: None,
            pending: false,
            status: Status::Installed,
            file_count: entry.files.len(),
            source: RowSource::Index,
            repo: None,
        });
    }

    for record in pending {
        push_or_merge(
            &mut rows,
            CatalogEntry {
                name: record.name.clone(),
                description: record.description.clone(),
                installed_version: None,
                available_version: Some("git".to_owned()),
                pending: true,
                status: Status::Pending,
                file_count: 0,
                source: RowSource::PendingGit,
                repo: Some(record.git.clone()),
            },
        );
    }

    for manifest in local {
        let row = CatalogEntry {
            name: manifest.name.clone(),
            description: manifest.description.clone(),
            installed_version: None,
            available_version: Some(manifest.version.to_string()),
            pending: false,
            status: Status::Available,
            file_count: 0,
            source: RowSource::LocalManifest,
            repo: None,
        };
        push_or_merge(&mut rows, row);
    }

    for entry in index {
        push_or_merge(
            &mut rows,
            CatalogEntry {
                name: entry.name.clone(),
                description: entry.repo.clone(),
                installed_version: None,
                available_version: None,
                pending: false,
                status: Status::Available,
                file_count: 0,
                source: RowSource::Index,
                repo: Some(entry.repo.clone()),
            },
        );
    }

    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

/// Add a source row, or merge its metadata into the row already present.
fn push_or_merge(rows: &mut Vec<CatalogEntry>, row: CatalogEntry) {
    if let Some(slot) = rows.iter().position(|existing| existing.name == row.name) {
        merge(rows, slot, row);
        return;
    }
    rows.push(row);
}

/// Merge `incoming` into the existing row at `slot`.
///
/// The existing row keeps `installed_version`/`file_count` (lock-owned) and
/// any description it already has. The source fields only move when
/// `incoming` is a strictly stronger source, so a local manifest or index
/// entry never erases a pending record's `"git"` version.
fn merge(rows: &mut [CatalogEntry], slot: usize, incoming: CatalogEntry) {
    let existing = &mut rows[slot];
    if !incoming.description.is_empty() && existing.description.is_empty() {
        existing.description = incoming.description;
    }
    if stronger_source(incoming.source, existing.source) {
        existing.source = incoming.source;
        existing.available_version = incoming.available_version;
        existing.repo = incoming.repo;
    }
    existing.pending = existing.pending || incoming.pending;
    existing.status = derive_status(
        existing.installed_version.as_deref(),
        existing.pending,
        existing.available_version.as_deref(),
    );
}

/// The badge contract: see the module docs.
fn derive_status(installed: Option<&str>, pending: bool, available: Option<&str>) -> Status {
    match installed {
        Some(installed_version) if pending || available.is_some_and(|v| v != installed_version) => {
            Status::Updatable
        }
        Some(_) => Status::Installed,
        None if pending => Status::Pending,
        None => Status::Available,
    }
}

fn stronger_source(candidate: RowSource, current: RowSource) -> bool {
    fn rank(source: RowSource) -> u8 {
        match source {
            RowSource::PendingGit => 3,
            RowSource::LocalManifest => 2,
            RowSource::Index => 1,
        }
    }
    rank(candidate) > rank(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::{LockEntry, LockFile};

    fn entry(name: &str, version: &str, files: usize) -> LockEntry {
        LockEntry {
            name: name.to_owned(),
            version: version.to_owned(),
            files: (0..files)
                .map(|i| format!("scripts/{name}-{i}.lua"))
                .collect(),
            config_d: false,
        }
    }

    fn pending(name: &str, desc: &str) -> PendingMeta {
        PendingMeta {
            name: name.to_owned(),
            version: "0.0.0".to_owned(),
            description: desc.to_owned(),
            git: format!("https://github.com/po5/{name}"),
        }
    }

    fn local(name: &str, version: &str) -> Manifest {
        Manifest::parse(&format!(
            "name: {name}\nversion: {version}\ndescription: {name} 本地包\nplatform: all\nfiles:\n  - src: x.lua\n    dest: ~~/scripts/x.lua\n"
        ))
        .expect("fixture manifest parses")
    }

    fn index_entry(name: &str) -> PackageEntry {
        PackageEntry {
            name: name.to_owned(),
            repo: format!("owner/{name}"),
            release_tag: "latest".to_owned(),
            homepage: None,
        }
    }

    fn lock_with(entries: Vec<LockEntry>) -> LockFile {
        LockFile { packages: entries }
    }

    #[test]
    fn merge_lock_pending_index_builds_one_row_per_package() {
        let lock = lock_with(vec![
            entry("alpha", "1.2.0", 3),
            entry("installed-only", "0.9.0", 1),
        ]);
        let pending = vec![pending("beta", "beta 描述")];
        let local = vec![];
        let index = vec![index_entry("alpha"), index_entry("gamma")];

        let catalog = build(&lock, &pending, &local, &index);

        // Sorted by name.
        let names: Vec<&str> = catalog.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "beta", "gamma", "installed-only"]);

        // Installed row keeps lock data; the index entry fills description.
        let alpha = &catalog[0];
        assert_eq!(alpha.status, Status::Installed);
        assert_eq!(alpha.installed_version.as_deref(), Some("1.2.0"));
        assert_eq!(alpha.file_count, 3);
        assert_eq!(alpha.description, "owner/alpha");
        assert_eq!(alpha.source, RowSource::Index);
        assert!(!alpha.pending);

        // Pending-only row.
        let beta = &catalog[1];
        assert_eq!(beta.status, Status::Pending);
        assert_eq!(beta.available_version.as_deref(), Some("git"));
        assert!(beta.pending);
        assert_eq!(beta.description, "beta 描述");
        assert_eq!(beta.source, RowSource::PendingGit);
        assert_eq!(beta.repo.as_deref(), Some("https://github.com/po5/beta"));

        // Index-only row.
        let gamma = &catalog[2];
        assert_eq!(gamma.status, Status::Available);
        assert_eq!(gamma.repo.as_deref(), Some("owner/gamma"));
        assert_eq!(gamma.source, RowSource::Index);

        // Installed package whose source disappeared still listed.
        let orphan = &catalog[3];
        assert_eq!(orphan.status, Status::Installed);
        assert_eq!(orphan.installed_version.as_deref(), Some("0.9.0"));
    }

    #[test]
    fn installed_pending_git_is_updatable() {
        let lock = lock_with(vec![entry("evafast", "1.0.0", 2)]);
        let pending = vec![pending("evafast", "evafast 描述")];

        let catalog = build(&lock, &pending, &[], &[]);

        let row = &catalog[0];
        assert_eq!(row.status, Status::Updatable);
        assert!(row.pending);
        assert_eq!(row.available_version.as_deref(), Some("git"));
        assert_eq!(row.installed_version.as_deref(), Some("1.0.0"));
        assert_eq!(row.description, "evafast 描述");
        assert_eq!(row.source, RowSource::PendingGit);
    }

    #[test]
    fn installed_with_newer_local_manifest_is_updatable() {
        let lock = lock_with(vec![entry("local-pkg", "1.0.0", 1)]);
        let local = vec![local("local-pkg", "2.0.0")];

        let catalog = build(&lock, &[], &local, &[]);

        let row = &catalog[0];
        assert_eq!(row.status, Status::Updatable);
        assert_eq!(row.available_version.as_deref(), Some("2.0.0"));
        assert_eq!(row.installed_version.as_deref(), Some("1.0.0"));
        assert_eq!(row.source, RowSource::LocalManifest);
        assert_eq!(row.description, "local-pkg 本地包");
    }

    #[test]
    fn installed_with_same_local_version_stays_installed() {
        let lock = lock_with(vec![entry("local-pkg", "1.0.0", 1)]);
        let local = vec![local("local-pkg", "1.0.0")];

        let catalog = build(&lock, &[], &local, &[]);

        assert_eq!(catalog[0].status, Status::Installed);
        assert_eq!(catalog[0].available_version.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn pending_shadows_local_and_index_for_same_name() {
        let pending = vec![pending("shared", "git 描述")];
        let local = vec![local("shared", "2.0.0")];
        let index = vec![index_entry("shared")];

        let catalog = build(&lock_with(vec![]), &pending, &local, &index);

        let row = &catalog[0];
        assert_eq!(row.status, Status::Pending);
        assert_eq!(row.description, "git 描述");
        assert_eq!(row.source, RowSource::PendingGit);
        assert!(row.pending);
    }

    #[test]
    fn local_manifest_overrides_index_for_same_name() {
        let local = vec![local("local-pkg", "2.0.0")];
        let index = vec![index_entry("local-pkg")];

        let catalog = build(&lock_with(vec![]), &[], &local, &index);

        let row = &catalog[0];
        assert_eq!(row.status, Status::Available);
        assert_eq!(row.source, RowSource::LocalManifest);
        assert_eq!(row.description, "local-pkg 本地包");
        assert_eq!(row.available_version.as_deref(), Some("2.0.0"));
        assert_eq!(row.repo, None);
    }

    #[test]
    fn empty_sources_yield_empty_catalog() {
        assert!(build(&LockFile::default(), &[], &[], &[]).is_empty());
    }

    #[test]
    fn sort_is_alphabetic_by_name() {
        let index = vec![
            index_entry("zebra"),
            index_entry("alpha"),
            index_entry("mango"),
        ];
        let catalog = build(&lock_with(vec![]), &[], &[], &index);
        let names: Vec<&str> = catalog.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mango", "zebra"]);
    }
}
