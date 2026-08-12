//! App update checking and guided upgrade (T22).
//!
//! The update check compares the local `VERSION` file against the optional
//! `latest_version` field of the remote index (fetched through the
//! [`crate::fetch`] abstraction, `curl` subprocess in production).
//!
//! `perform_upgrade` is the single entry point for the destructive action:
//! download the new zip → extract + structure-validate → back up the current
//! app layer → overlay the new app layer (user layer untouched) → report.
//! Any failure after the backup restores the backup (rollback). It never
//! applies changes automatically on its own — confirmation is enforced at
//! the caller (CLI `--yes`, UI dialog, per the D17 decision).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::fetch::{extract, fetch_index, unique_suffix, Fetcher};
use crate::index::Index;

/// Result of an update check. Never fails: every failure mode is reported
/// in [`UpdateInfo::error`] as a readable message.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct UpdateInfo {
    /// Local version from the root `VERSION` file (empty when missing).
    pub current: String,
    /// Remote `latest_version` when the index carries one.
    pub latest: Option<String>,
    /// True when `latest` is a newer version than `current`.
    pub has_update: bool,
    /// Changelog / release page link from the index, if provided.
    pub changelog_url: Option<String>,
    /// Readable failure message (network error, missing VERSION, index
    /// without update metadata); `None` when the check succeeded.
    pub error: Option<String>,
}

/// Outcome of one upgrade run, including rollback information.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct UpgradeResult {
    /// The version applied; `None` when nothing was applied.
    pub applied_version: Option<String>,
    /// Where the pre-upgrade app layer was backed up.
    pub backup_dir: Option<PathBuf>,
    /// Number of files replaced into the app layer.
    pub files_replaced: usize,
    /// True when a failure after backup occurred and the app layer was
    /// restored from the backup.
    pub rolled_back: bool,
    /// Human-readable outcome message (Chinese).
    pub message: String,
}

/// Top-level entries excluded from the app-layer backup: user data, VCS,
/// planning artifacts, and regenerable build output.
const BACKUP_EXCLUDE: &[&str] = &["user", ".git", ".omo", "target", "dist"];

/// Entries never copied from a downloaded zip into the repository root:
/// the user layer is sacred and survives every upgrade.
const OVERLAY_EXCLUDE: &[&str] = &["user", ".git", ".omo"];

/// Remote metadata extracted from the index for an upgrade run.
struct RemoteMeta {
    latest: String,
    zip_url: String,
    changelog_url: Option<String>,
}

/// Read and trim the root `VERSION` file.
fn read_version(root: &Path) -> Result<String, String> {
    let path = root.join("VERSION");
    fs::read_to_string(&path)
        .map(|text| text.trim().to_owned())
        .map_err(|source| {
            if source.kind() == io::ErrorKind::NotFound {
                format!("缺少 VERSION 文件 {}:无法确认当前版本", path.display())
            } else {
                format!("读取 {} 失败:{source}", path.display())
            }
        })
}

/// Fetch and parse the remote index; every failure is a readable message.
fn fetch_remote_meta(fetcher: &dyn Fetcher, index_url: &str) -> Result<RemoteMeta, String> {
    let index = fetch_index(fetcher, index_url)
        .map_err(|error| format!("获取远程索引失败:{error}"))
        .and_then(|text| Index::parse(&text).map_err(|error| format!("索引校验失败:{error}")))?;
    let Some(latest) = index.latest_version else {
        return Err("索引未提供 latest_version 字段,无更新信息".to_owned());
    };
    let Some(zip_url) = index.upgrade_zip_url else {
        return Err("索引未提供 upgrade_zip_url 字段,无法定位升级包".to_owned());
    };
    Ok(RemoteMeta {
        latest,
        zip_url,
        changelog_url: index.changelog_url,
    })
}

/// True when `latest` is newer than `current`. Versions are compared with
/// semver when both parse; otherwise falls back to string inequality.
fn newer_than(current: &str, latest: &str) -> bool {
    match (
        semver::Version::parse(current),
        semver::Version::parse(latest),
    ) {
        (Ok(current), Ok(latest)) => latest > current,
        _ => current != latest,
    }
}

/// Changelog link fallback derived from the index URL (the releases page of
/// the index repository) when the index itself does not provide one.
fn fallback_changelog_url(index_url: &str) -> Option<String> {
    const SUFFIX: &str = "/releases/latest/download/index.json";
    let base = index_url.strip_suffix(SUFFIX)?;
    Some(format!("{base}/releases"))
}

/// Check whether a newer version is available. See [`UpdateInfo`].
pub fn check_update(fetcher: &dyn Fetcher, index_url: &str, root: &Path) -> UpdateInfo {
    let current = match read_version(root) {
        Ok(version) => version,
        Err(error) => {
            return UpdateInfo {
                error: Some(error),
                ..UpdateInfo::default()
            };
        }
    };
    let meta = match fetch_remote_meta(fetcher, index_url) {
        Ok(meta) => meta,
        Err(error) => {
            return UpdateInfo {
                current,
                error: Some(error),
                ..UpdateInfo::default()
            };
        }
    };
    let has_update = newer_than(&current, &meta.latest);
    UpdateInfo {
        current,
        latest: Some(meta.latest.clone()),
        has_update,
        changelog_url: meta
            .changelog_url
            .or_else(|| fallback_changelog_url(index_url)),
        error: None,
    }
}

/// Locate the app root inside an extracted zip: the directory carrying
/// `config/` or `mpv.conf`, possibly wrapped in a single top-level folder
/// (the zip's `mpv-config/` root from the T10 layout).
fn find_app_root(extracted: &Path) -> Option<PathBuf> {
    let is_app = |dir: &Path| dir.join("config").is_dir() || dir.join("mpv.conf").is_file();
    if is_app(extracted) {
        return Some(extracted.to_path_buf());
    }
    let children: Vec<fs::DirEntry> = fs::read_dir(extracted)
        .ok()?
        .filter_map(Result::ok)
        .collect();
    if children.len() == 1 && children[0].file_type().ok()?.is_dir() {
        let inner = children[0].path();
        if is_app(&inner) {
            return Some(inner);
        }
    }
    None
}

/// Recursively copy `src` into `dst` (created as needed), skipping entries
/// named in `top_skip` at the top level and `deep_skip` at every level.
/// Returns the number of files copied.
fn copy_tree(src: &Path, dst: &Path, top_skip: &[&str], deep_skip: &[&str]) -> io::Result<usize> {
    let mut count = 0;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_str().unwrap_or_default();
        if top_skip.contains(&name_str) || deep_skip.contains(&name_str) {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&to)?;
            count += copy_tree(&from, &to, &[], deep_skip)?;
        } else {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&from, &to)?;
            count += 1;
        }
    }
    Ok(count)
}

/// Epoch seconds, used to name the backup directory.
fn timestamp_secs() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or_else(
            |_| "unknown".to_owned(),
            |duration| duration.as_secs().to_string(),
        )
}

/// Download, extract, structure-validate, and overlay the new app layer.
/// Runs after the backup exists, so every failure here is rolled back by
/// the caller.
fn apply_new_app(zip_path: &Path, work: &Path, root: &Path) -> Result<usize, String> {
    let extracted = work.join("extracted");
    fs::create_dir_all(&extracted).map_err(|error| format!("创建临时目录失败:{error}"))?;
    extract(zip_path, &extracted).map_err(|error| format!("升级包解压或校验失败:{error}"))?;
    let app_root = find_app_root(&extracted)
        .ok_or_else(|| "升级包结构无效(缺少 config/ 或 mpv.conf),拒绝替换".to_owned())?;
    copy_tree(&app_root, root, &[], OVERLAY_EXCLUDE)
        .map_err(|error| format!("替换 app 层失败:{error}"))
}

/// Perform a full upgrade. Never panics; every outcome is a report.
pub fn perform_upgrade(
    fetcher: &dyn Fetcher,
    index_url: &str,
    cache: &Path,
    root: &Path,
) -> UpgradeResult {
    let mut result = UpgradeResult::default();
    let local = match read_version(root) {
        Ok(version) => version,
        Err(error) => {
            result.message = format!("升级中止:{error}");
            return result;
        }
    };
    let meta = match fetch_remote_meta(fetcher, index_url) {
        Ok(meta) => meta,
        Err(error) => {
            result.message = format!("升级中止:{error}");
            return result;
        }
    };
    if !newer_than(&local, &meta.latest) {
        result.message = format!("当前版本 {local} 已是最新,无需升级");
        return result;
    }

    let upgrade_dir = cache.join("upgrade");
    let zip_path = upgrade_dir.join(format!("mpv-config-{}.zip", meta.latest));
    if let Err(error) = fs::create_dir_all(&upgrade_dir) {
        result.message = format!("升级中止:创建缓存目录失败:{error}");
        return result;
    }
    let bytes = match fetcher.get(&meta.zip_url) {
        Ok(bytes) => bytes,
        Err(error) => {
            result.message = format!("升级中止:下载升级包失败:{error}");
            return result;
        }
    };
    if let Err(error) = fs::write(&zip_path, bytes) {
        result.message = format!("升级中止:写入升级包失败:{error}");
        return result;
    }

    let work = cache.join(format!(".upgrade-{}", unique_suffix()));
    let backup = cache.join("backup").join(timestamp_secs());
    if let Err(error) = copy_tree(root, &backup, BACKUP_EXCLUDE, &[]) {
        let _ = fs::remove_dir_all(&work);
        result.message = format!("升级中止:备份 app 层失败:{error}");
        return result;
    }
    result.backup_dir = Some(backup.clone());

    match apply_new_app(&zip_path, &work, root) {
        Ok(replaced) => {
            let _ = fs::remove_dir_all(&work);
            result.applied_version = Some(meta.latest.clone());
            result.files_replaced = replaced;
            result.message = format!(
                "升级成功:{local} → {},共替换 {replaced} 个文件;升级后请重新运行 `mpv-config gen` 重新生成配置",
                meta.latest
            );
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&work);
            let restore_ok = copy_tree(&backup, root, &[], &[]).is_ok();
            result.rolled_back = true;
            result.message = format!(
                "升级失败:{error};已{}从备份还原,user 层不受影响",
                if restore_ok { "成功" } else { "未能" }
            );
            if !restore_ok {
                result.message = format!(
                    "{},请手动从 {} 恢复 app 层",
                    result.message,
                    backup.display()
                );
            }
        }
    }
    result
}

#[cfg(test)]
mod tests;
