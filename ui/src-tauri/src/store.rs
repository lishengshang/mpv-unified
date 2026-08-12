//! Tauri commands for the package store page (T20).
//!
//! Every command calls crate functions directly — no shell, no subprocess,
//! no browser-side network. Sources are read from the repository root
//! (found by [`cli::gen::repo_root`]) and merged by [`pkg::catalog`];
//! install/uninstall/update delegate to the `cli` lifecycle, which owns
//! rollback and shared-file safety.

use std::path::Path;

use mpv_core::platform;
use pkg::catalog::{self, CatalogEntry, PendingMeta};
use pkg::index::Index;
use serde::Serialize;

use cli::pkg_cmds::lifecycle::{
    self, InstallOptions, InstallOutcome, UninstallOptions, UpdateOptions,
};

/// Result of an install/uninstall/update action, shown as a toast.
#[derive(Debug, Serialize)]
pub struct ActionResult {
    pub ok: bool,
    pub message: String,
    pub warnings: Vec<String>,
}

fn repo_root() -> std::path::PathBuf {
    cli::gen::repo_root()
}

fn cache_dir() -> Result<std::path::PathBuf, String> {
    lifecycle::default_cache_dir().map_err(|error| error.to_string())
}

/// The store listing: lock + pending + local manifests + cached index,
/// merged and sorted by [`pkg::catalog::build`]. Parse failures in any
/// optional source (one bad pending record, a corrupt index) degrade to
/// "source skipped", never to a command error — the store must stay
/// browsable.
#[tauri::command]
pub fn list_packages() -> Result<Vec<CatalogEntry>, String> {
    let root = repo_root();

    let lock = pkg::lock::read(&root.join("packages.lock"))
        .map_err(|error| format!("读取 packages.lock 失败:{error}"))?;
    let pending = read_pending(&root);
    let local = read_local(&root);
    let index = read_index();

    Ok(catalog::build(&lock, &pending, &local, &index))
}

/// Fetch and validate the remote `index.json` into the package cache.
/// The readable error covers transport, validation, and cache failures.
#[tauri::command]
pub fn update_index() -> Result<String, String> {
    let cache = cache_dir()?;
    let options = lifecycle::UpdateIndexOptions {
        index_url: pkg::fetch::DEFAULT_INDEX_URL.to_owned(),
        cache_dir: cache.clone(),
    };
    let path = lifecycle::update_index(&options).map_err(|error| error.to_string())?;
    Ok(path.display().to_string())
}

/// Install `name` (pending git record → local manifest → index source
/// priority, as the CLI). Platform mismatch is a skip with a notice, not an
/// error.
#[tauri::command]
pub fn install_package(name: String) -> Result<ActionResult, String> {
    let options = InstallOptions {
        name: name.clone(),
        repo_root: repo_root(),
        cache_dir: cache_dir()?,
        platform: platform::detect(),
    };
    match lifecycle::install(&options).map_err(|error| error.to_string())? {
        InstallOutcome::Installed(report) => Ok(ActionResult {
            ok: true,
            message: report.to_string(),
            warnings: Vec::new(),
        }),
        InstallOutcome::SkippedPlatform { name, .. } => Ok(ActionResult {
            ok: true,
            message: format!("{name}:目标平台不匹配,已跳过安装"),
            warnings: Vec::new(),
        }),
    }
}

/// Uninstall `name`; shared files are kept while another package uses them.
#[tauri::command]
pub fn uninstall_package(name: String) -> Result<ActionResult, String> {
    let options = UninstallOptions {
        name: name.clone(),
        repo_root: repo_root(),
    };
    let report = lifecycle::uninstall(&options).map_err(|error| error.to_string())?;
    Ok(ActionResult {
        ok: true,
        message: report.to_string(),
        warnings: Vec::new(),
    })
}

/// Update `name` to the newest resolvable version; already-current packages
/// are reported as warnings instead of errors.
#[tauri::command]
pub fn update_package(name: String) -> Result<ActionResult, String> {
    let options = UpdateOptions {
        name: Some(name),
        repo_root: repo_root(),
        cache_dir: cache_dir()?,
    };
    let report = lifecycle::update(&options).map_err(|error| error.to_string())?;
    let warnings: Vec<String> = report
        .items
        .iter()
        .filter(|item| !item.changed)
        .map(|item| format!("{} 已是最新版本 v{}", item.name, item.to))
        .collect();
    Ok(ActionResult {
        ok: true,
        message: report.to_string(),
        warnings,
    })
}

/// Parse every `packages/pending/<name>.yaml`; malformed records are
/// skipped (a corrupt record should never hide the rest of the store).
fn read_pending(root: &Path) -> Vec<PendingMeta> {
    let Some(entries) = std::fs::read_dir(root.join("packages").join("pending")).ok() else {
        return Vec::new();
    };
    entries
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "yaml"))
        .filter_map(|path| {
            cli::pkg_cmds::lifecycle::pending::PendingRecord::parse_file(&path).ok()
        })
        .map(|record| PendingMeta {
            name: record.name,
            version: record.version,
            description: record.description,
            git: record.source.git,
        })
        .collect()
}

/// Parse every `packages/<name>.yaml` (local static manifests, T14 source
/// priority 2). Malformed manifests are skipped.
fn read_local(root: &Path) -> Vec<pkg::manifest::Manifest> {
    let Some(entries) = std::fs::read_dir(root.join("packages")).ok() else {
        return Vec::new();
    };
    entries
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "yaml"))
        .filter_map(|path| pkg::manifest::Manifest::parse_file(&path).ok())
        .collect()
}

/// Read the cached remote index (`cache_dir/index.json`); missing or
/// invalid → empty, so the store still lists lock + pending packages.
fn read_index() -> Vec<pkg::index::PackageEntry> {
    let Ok(cache) = cache_dir() else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(cache.join("index.json")) else {
        return Vec::new();
    };
    Index::parse(&text).map_or_else(|_| Vec::new(), |index| index.packages)
}
