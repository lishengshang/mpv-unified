//! `pkg migrate-manager`: one-click migrator from the legacy mpv_manager
//! `manager.json` format into pending git-source records under
//! `packages/pending/`.
//!
//! Legacy entries look like:
//!
//! ```json
//! {"git": "https://github.com/po5/evafast", "branch": "rewrite",
//!  "whitelist": "%.lua$", "dest": "~~/scripts"}
//! ```
//!
//! A git source cannot be expanded into a static `files` list without
//! network access (whitelist/blacklist are regexes applied to a clone), so
//! this migrator is *index-only*: every entry becomes a `pending` record that
//! keeps the original `git`/`branch`/`whitelist`/`blacklist`/`dest` verbatim.
//! `pkg install` (T14) consumes those records, clones the source, expands the
//! file list and writes `packages.lock`.
//!
//! Name validation reuses `pkg::manifest::Manifest::parse` — the canonical
//! manifest validation — via a minimal probe document, so this module never
//! duplicates or forks validation logic.

pub mod report;

pub use report::{FailedEntry, MigratedEntry, MigrationReport};

use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use pkg::manifest::Manifest;
use serde::{Deserialize, Serialize};

/// Version placeholder: git sources carry no version tags readable offline.
const PLACEHOLDER_VERSION: &str = "0.0.0";

/// Marker proving the record awaits T14 installation.
const MIGRATED_PENDING: &str = "pending";

/// Comment header prepended to every generated pending record.
const FILE_HEADER: &str = "\
# 由 `mpv-config pkg migrate-manager` 生成 - 请勿手动编辑。
# 待安装记录(migrated: pending):`pkg install` 会克隆 source.git,
# 按 whitelist/blacklist 展开具体文件清单,再写入 packages.lock。
# version 0.0.0 为占位符(git 源无版本号)。
";

/// One entry of the legacy `manager.json` format.
#[derive(Debug, Deserialize)]
struct ManagerEntry {
    git: String,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    whitelist: Option<String>,
    #[serde(default)]
    blacklist: Option<String>,
    dest: String,
}

/// Pending record written to `packages/pending/<name>.yaml`: the extended
/// (non-standard) manifest consumed by T14 installation.
#[derive(Debug, Serialize)]
struct PendingRecord {
    name: String,
    version: String,
    description: String,
    platform: String,
    source: PendingSource,
    migrated: String,
}

/// Git source description kept verbatim for later expansion.
#[derive(Debug, Serialize)]
struct PendingSource {
    git: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    whitelist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blacklist: Option<String>,
    dest: String,
}

/// Failure modes of a migration run. I/O and JSON failures abort the whole
/// run before any pending record is written (no half-products).
#[derive(Debug)]
pub enum MigrateError {
    Io(io::Error),
    Json(serde_json::Error),
    Serialize(String),
}

impl fmt::Display for MigrateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "文件读写失败: {error}"),
            Self::Json(error) => write!(f, "manager.json 解析失败: {error}"),
            Self::Serialize(message) => write!(f, "待安装记录序列化失败: {message}"),
        }
    }
}

impl std::error::Error for MigrateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Serialize(_) => None,
        }
    }
}

/// Migrate every entry of `input` into a pending record under `out_dir` and
/// write the migration report to `report_path`.
///
/// The JSON document is fully parsed before the first write, so a corrupted
/// input fails without producing half-products.
///
/// # Errors
///
/// Returns [`MigrateError::Io`] on read/write failures and
/// [`MigrateError::Json`] on malformed input.
pub fn run(
    input: &Path,
    out_dir: &Path,
    report_path: &Path,
) -> Result<MigrationReport, MigrateError> {
    let raw = fs::read_to_string(input).map_err(MigrateError::Io)?;
    let entries: Vec<ManagerEntry> = serde_json::from_str(&raw).map_err(MigrateError::Json)?;

    let input_display = input.display().to_string();
    let out_display = out_dir.display().to_string();
    let report_display = report_path.display().to_string();

    fs::create_dir_all(out_dir).map_err(MigrateError::Io)?;

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    let mut notes = Vec::new();
    let mut used_names = HashSet::new();

    for (offset, entry) in entries.iter().enumerate() {
        let index = offset + 1;
        let Some(candidate) = infer_candidate(&entry.git) else {
            failed.push(FailedEntry {
                index,
                git: entry.git.clone(),
                reason: "无法从 git URL 推断包名".to_owned(),
            });
            continue;
        };
        if let Err(message) = validate_package_name(&candidate.name) {
            failed.push(FailedEntry {
                index,
                git: entry.git.clone(),
                reason: format!("包名非法:{message}"),
            });
            continue;
        }
        let name = uniquify(
            candidate.name,
            &mut used_names,
            index,
            &entry.git,
            &mut notes,
        );
        let record = PendingRecord {
            name: name.clone(),
            version: PLACEHOLDER_VERSION.to_owned(),
            description: format!(
                "从 manager.json 迁移:{} -> {}",
                candidate.display, entry.dest
            ),
            platform: "all".to_owned(),
            source: PendingSource {
                git: entry.git.clone(),
                branch: entry.branch.clone(),
                whitelist: entry.whitelist.clone(),
                blacklist: entry.blacklist.clone(),
                dest: entry.dest.clone(),
            },
            migrated: MIGRATED_PENDING.to_owned(),
        };
        let yaml =
            serde_yaml::to_string(&record).map_err(|e| MigrateError::Serialize(e.to_string()))?;
        let path = out_dir.join(format!("{name}.yaml"));
        fs::write(&path, format!("{FILE_HEADER}{yaml}")).map_err(MigrateError::Io)?;
        succeeded.push(MigratedEntry {
            name,
            git: entry.git.clone(),
            dest: entry.dest.clone(),
        });
    }

    let report = MigrationReport {
        input_display,
        out_display,
        report_display,
        succeeded,
        failed,
        notes,
    };
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent).map_err(MigrateError::Io)?;
    }
    fs::write(report_path, report.to_markdown()).map_err(MigrateError::Io)?;
    Ok(report)
}

/// A derived package name plus a human-readable display of the source.
struct Candidate {
    name: String,
    display: String,
}

/// Derive a package name and display from a git URL.
///
/// - `https://github.com/<owner>/<repo>` -> `<repo>` slugified
/// - `https://gist.github.com/<owner>/<hash>` -> `<owner>-<hash[..8]>`
/// - scp-style `git@host:<owner>/<repo>.git`, trailing `/` and `.git` handled
///
/// Returns `None` when no `<owner>/<repo>` pair can be identified.
fn infer_candidate(git_url: &str) -> Option<Candidate> {
    let (owner, repo, is_gist) = split_owner_repo(git_url)?;
    if is_gist {
        let short: String = repo.trim_end_matches(".git").chars().take(8).collect();
        let name = format!("{}-{}", slugify(&owner)?, short);
        Some(Candidate {
            name,
            display: format!("gist {owner}/{short}"),
        })
    } else {
        let name = slugify(repo.trim_end_matches(".git"))?;
        Some(Candidate {
            name,
            display: format!("{owner}/{repo}"),
        })
    }
}

/// Split a git URL into `(owner, repo, is_gist)`. Accepts
/// `https://host/owner/repo`, `ssh://host/owner/repo`, and
/// `git@host:owner/repo` spellings.
fn split_owner_repo(git_url: &str) -> Option<(String, String, bool)> {
    let url = git_url.trim().trim_end_matches('/');
    if url.is_empty() {
        return None;
    }
    let is_gist = url.contains("gist.github.com");
    let after_scheme = match url.find("://") {
        Some(index) => &url[index + 3..],
        None => url,
    };
    let after_user = after_scheme.strip_prefix("git@").unwrap_or(after_scheme);
    let path = match after_user.split_once(':') {
        Some((_, path)) if !path.is_empty() => path,
        _ => {
            let mut parts = after_user.splitn(2, '/');
            parts.next()?;
            parts.next()?
        }
    };
    let mut segments = path.split('/').filter(|s| !s.is_empty());
    let owner = segments.next()?;
    let repo = segments.next()?;
    Some((owner.to_owned(), repo.to_owned(), is_gist))
}

/// Lowercase; every non-alphanumeric becomes `-`; runs collapse; leading and
/// trailing `-` are trimmed. `None` when nothing usable remains.
fn slugify(value: &str) -> Option<String> {
    let mut out = String::with_capacity(value.len());
    let mut pending_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(ch.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Validate a candidate package name against the `pkg::manifest` rules by
/// probing a minimal manifest through [`Manifest::parse`]: the canonical
/// validation is reused instead of being re-implemented here.
fn validate_package_name(name: &str) -> Result<(), String> {
    let probe = format!(
        "name: {name}\nversion: 0.0.0\ndescription: probe\nfiles:\n  - src: probe\n    dest: ~~/scripts\n"
    );
    match Manifest::parse(&probe) {
        Ok(_) => Ok(()),
        Err(error) => Err(format!("{}:{}", error.field, error.message)),
    }
}

/// Ensure `name` is unique among `used`, appending `-2`, `-3`, ... in order;
/// records a note when renamed.
fn uniquify(
    name: String,
    used: &mut HashSet<String>,
    index: usize,
    git: &str,
    notes: &mut Vec<String>,
) -> String {
    if used.insert(name.clone()) {
        return name;
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{name}-{suffix}");
        if used.insert(candidate.clone()) {
            notes.push(format!(
                "条目 #{index}({git})与已迁移包重名,命名为 {candidate}"
            ));
            return candidate;
        }
        suffix += 1;
    }
}

#[cfg(test)]
mod tests;
