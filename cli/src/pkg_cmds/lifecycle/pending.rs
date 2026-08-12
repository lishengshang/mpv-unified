//! Pending (git-source) records: parsing, cloning, and expansion.
//!
//! A pending record (written by `pkg migrate-manager`, T12) is a manifest
//! superset with a `source: {git, branch?, whitelist?, blacklist?, dest}`
//! block and `migrated: pending`. The file list is not static — it is
//! expanded by cloning the git source and applying the whitelist/blacklist
//! regexes to the clone's file tree, which yields a real
//! [`pkg::manifest::Manifest`] the installer can consume like any other.
//!
//! The whitelist/blacklist patterns come from the legacy `manager.json`
//! format (Java-flavored regexes). They are applied verbatim with the Rust
//! `regex` crate; patterns that do not compile surface as a clear error
//! instead of being silently dropped.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pkg::manifest::{FileEntry, Manifest, Platform};

use crate::pkg_cmds::lifecycle::error::{LifecycleError, Result};
use crate::pkg_cmds::lifecycle::pending::regex_filters::compile_filters;

/// Marker field value proving a record awaits T14 installation.
const MIGRATED_PENDING: &str = "pending";

/// A pending git-source record (`packages/pending/<name>.yaml`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PendingRecord {
    pub name: String,
    pub version: String,
    pub description: String,
    pub platform: String,
    pub source: PendingSource,
    pub migrated: String,
}

/// The verbatim git source description from `manager.json`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PendingSource {
    pub git: String,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub whitelist: Option<String>,
    #[serde(default)]
    pub blacklist: Option<String>,
    pub dest: String,
}

/// A pending record resolved into an installable manifest plus the clone
/// directory holding the source files.
#[derive(Debug)]
pub struct ResolvedPending {
    pub manifest: Manifest,
    /// Root of the freshly cloned repository (removed by the caller when
    /// the install finishes or fails).
    pub clone_dir: PathBuf,
}

impl PendingRecord {
    /// Parse and validate a pending record file.
    ///
    /// # Errors
    ///
    /// [`LifecycleError::Pending`] on malformed YAML, a missing `migrated:
    /// pending` marker, an invalid package name/version/platform, or a `dest`
    /// that is not a valid `~~/` manifest destination.
    pub fn parse_file(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .map_err(|e| LifecycleError::io(format!("读取 {}", path.display()), e))?;
        let record: PendingRecord =
            serde_yaml::from_str(&text).map_err(|e| LifecycleError::Pending {
                name: path
                    .file_stem()
                    .map_or_else(|| "未知".to_owned(), |s| s.to_string_lossy().into_owned()),
                message: format!("YAML 解析失败:{e}"),
            })?;
        record.validate()?;
        Ok(record)
    }

    /// Validate name/version/platform/`migrated`/`dest` without touching the
    /// network. Dest validity reuses the canonical `Manifest::parse` rules
    /// via a probe document (same trick as the migrator).
    ///
    /// # Errors
    ///
    /// [`LifecycleError::Pending`] on the first invalid field.
    pub fn validate(&self) -> Result<()> {
        let name = self.name.clone();
        if self.migrated != MIGRATED_PENDING {
            return Err(LifecycleError::Pending {
                name,
                message: format!(
                    "migrated={:?},期望 \"{MIGRATED_PENDING}\"(本命令只安装待安装记录)",
                    self.migrated
                ),
            });
        }
        if self.source.git.trim().is_empty() {
            return Err(LifecycleError::Pending {
                name,
                message: "source.git 为空".to_owned(),
            });
        }
        let probe = format!(
            "name: {name}\nversion: {}\ndescription: probe\nplatform: {}\nfiles:\n  - src: probe\n    dest: {}\n",
            self.version, self.platform, self.source.dest
        );
        if let Err(error) = Manifest::parse(&probe) {
            return Err(LifecycleError::Pending {
                name,
                message: format!("{}:{}", error.field, error.message),
            });
        }
        Ok(())
    }

    /// Target platform of the record (`all` by convention from the migrator).
    #[must_use]
    pub fn platform(&self) -> Platform {
        self.platform.parse().unwrap_or(Platform::All)
    }
}

/// Clone the source into `work_dir`, apply the whitelist/blacklist filters,
/// and expand the surviving files into a real manifest whose `files` map
/// each kept clone file to `source.dest/<relpath>`.
///
/// The clone lives at `work_dir/<name>-<pid>-<counter>` and is NOT removed
/// here — the installer owns its lifecycle.
///
/// # Errors
///
/// [`LifecycleError::CommandUnavailable`] when git is missing,
/// [`LifecycleError::CommandFailed`] when the clone fails, [`LifecycleError::Regex`]
/// when a filter pattern does not compile, [`LifecycleError::Pending`] when
/// the clone yields no matching files.
pub fn resolve(record: &PendingRecord, work_dir: &Path) -> Result<ResolvedPending> {
    let clone_dir = work_dir.join(format!(
        "{}-{}-{}",
        record.name,
        std::process::id(),
        super::unique_suffix()
    ));

    let mut command = Command::new("git");
    command.args(["clone", "--depth", "1"]);
    if let Some(branch) = &record.source.branch {
        command.args(["--branch", branch]);
    }
    command.arg("--").arg(&record.source.git).arg(&clone_dir);
    let output = command.output().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => LifecycleError::CommandUnavailable {
            command: "git",
            hint: "请安装 git(或加入 PATH)",
        },
        _ => LifecycleError::io("spawn git", e),
    })?;
    if !output.status.success() {
        return Err(LifecycleError::command_failed("git clone", &output));
    }

    let files = collect_files(&clone_dir)
        .map_err(|e| LifecycleError::io(format!("遍历克隆目录 {}", clone_dir.display()), e))?;
    let filters = compile_filters(
        record.source.whitelist.as_deref(),
        record.source.blacklist.as_deref(),
    )?;
    let kept: Vec<String> = files.into_iter().filter(|rel| filters.keep(rel)).collect();

    if kept.is_empty() {
        return Err(LifecycleError::Pending {
            name: record.name.clone(),
            message: format!(
                "git 源 {:?} 克隆成功但过滤后无文件(检查 whitelist/blacklist)",
                record.source.git
            ),
        });
    }

    let base = record.source.dest.trim_end_matches('/').to_owned();
    let files = kept
        .into_iter()
        .map(|rel| FileEntry {
            src: rel.clone(),
            dest: format!("{base}/{rel}"),
        })
        .collect();
    let manifest = Manifest {
        name: record.name.clone(),
        version: record
            .version
            .parse()
            .unwrap_or_else(|_| semver::Version::new(0, 0, 0)),
        description: record.description.clone(),
        author: None,
        homepage: None,
        license: None,
        platform: record.platform(),
        requires: Vec::new(),
        conflicts: Vec::new(),
        files,
        config: Vec::new(),
    };
    Ok(ResolvedPending {
        manifest,
        clone_dir,
    })
}

/// Recursively list every file under `root` as a `/`-joined relative path
/// (skipping `.git/`; a clone's VCS metadata is never installed).
fn collect_files(root: &Path) -> std::io::Result<Vec<String>> {
    let mut out = Vec::new();
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .map_err(|_| std::io::Error::other("path outside clone root"))?;
            if rel.components().any(|c| c.as_os_str() == ".git") {
                continue;
            }
            let rel_str = rel
                .components()
                .map(|c| c.as_os_str())
                .collect::<Vec<_>>()
                .join(std::ffi::OsStr::new("/"))
                .into_string()
                .map_err(|_| std::io::Error::other("非 UTF-8 文件名,跳过"))?;
            if entry.file_type()?.is_dir() {
                walk(&path, root, out)?;
            } else if entry.file_type()?.is_file() {
                out.push(rel_str);
            }
        }
        Ok(())
    }
    walk(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

/// Compiled whitelist/blacklist pair.
#[derive(Debug)]
pub(crate) struct Filters {
    whitelist: Option<regex::Regex>,
    blacklist: Option<regex::Regex>,
}

impl Filters {
    /// Whether `rel_path` survives: whitelist (when present) must match,
    /// blacklist (when present) must not.
    #[must_use]
    pub fn keep(&self, rel_path: &str) -> bool {
        let allowed = self
            .whitelist
            .as_ref()
            .is_none_or(|re| re.is_match(rel_path));
        let denied = self
            .blacklist
            .as_ref()
            .is_some_and(|re| re.is_match(rel_path));
        allowed && !denied
    }
}

mod regex_filters {
    use super::Filters;
    use crate::pkg_cmds::lifecycle::error::{LifecycleError, Result};

    /// Compile the optional patterns; an invalid regex is a hard error so a
    /// typo in a legacy filter can never silently install the wrong files.
    pub(super) fn compile_filters(
        whitelist: Option<&str>,
        blacklist: Option<&str>,
    ) -> Result<Filters> {
        let whitelist = whitelist
            .map(|pattern| {
                regex::Regex::new(pattern).map_err(|e| LifecycleError::Regex {
                    pattern: pattern.to_owned(),
                    message: e.to_string(),
                })
            })
            .transpose()?;
        let blacklist = blacklist
            .map(|pattern| {
                regex::Regex::new(pattern).map_err(|e| LifecycleError::Regex {
                    pattern: pattern.to_owned(),
                    message: e.to_string(),
                })
            })
            .transpose()?;
        Ok(Filters {
            whitelist,
            blacklist,
        })
    }
}

/// Unique per-process suffix for temp artifacts (same scheme as `pkg`).
pub(crate) fn unique_suffix() -> String {
    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    format!(
        "{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

#[cfg(test)]
mod tests;
