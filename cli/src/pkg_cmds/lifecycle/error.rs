//! Typed, human-readable failures for the package lifecycle commands.
//!
//! Every failure mode of `install`/`uninstall`/`update` is one variant here
//! with a Chinese [`Display`] message the CLI prints as-is. Nothing panics.

use std::fmt;
use std::io;
use std::path::PathBuf;

use pkg::deps::DepError;
use pkg::fetch::FetchError;
use pkg::index::IndexError;
use pkg::manifest::PackageError;

use crate::pkg_cmds::lifecycle::lock::LockError;

/// A package-lifecycle failure with a user-facing Chinese message.
#[derive(Debug)]
pub enum LifecycleError {
    /// Filesystem failure with the operation that was underway.
    Io { context: String, source: io::Error },
    /// No installable source found for the requested name.
    PackageNotFound { name: String },
    /// Manifest (local, pending, or fetched archive) failed to parse.
    Manifest { name: String, error: PackageError },
    /// The pending record is not actually pending (or malformed).
    Pending { name: String, message: String },
    /// The local package index cache is missing or invalid.
    Index(IndexError),
    /// Fetching the index or a package archive failed.
    Fetch(FetchError),
    /// Dependency or conflict check rejected the package.
    Deps(DepError),
    /// One or more required packages are not installed (MVP: report and
    /// suggest installing them first; no automatic dependency install).
    MissingDeps {
        required_by: String,
        missing: Vec<String>,
    },
    /// The package is already installed; suggest `update` instead.
    AlreadyInstalled { name: String, version: String },
    /// Uninstall/update target is not recorded in `packages.lock`.
    NotInstalled { name: String },
    /// A required CLI tool (git) is missing.
    CommandUnavailable {
        command: &'static str,
        hint: &'static str,
    },
    /// A subprocess (git) ran but failed; stderr excerpt included.
    CommandFailed {
        command: &'static str,
        detail: String,
    },
    /// A whitelist/blacklist pattern is not a valid regex.
    Regex { pattern: String, message: String },
    /// The lock file could not be read or written.
    Lock(LockError),
    /// Rollback after a mid-install/update failure was not possible.
    RollbackFailed { detail: String },
}

impl LifecycleError {
    pub(crate) fn io(context: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }

    /// Map a `git` clone/checkout failure into a user-facing error.
    pub(crate) fn command_failed(command: &'static str, output: &std::process::Output) -> Self {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Self::CommandFailed { command, detail }
    }
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { context, source } => write!(f, "{context}: {source}"),
            Self::PackageNotFound { name } => write!(
                f,
                "找不到包 \"{name}\":packages/、packages/pending/ 与索引缓存中均无此包"
            ),
            Self::Manifest { name, error } => {
                write!(f, "包 \"{name}\" 的 manifest 无效:{}", error.field)
            }
            Self::Pending { name, message } => {
                write!(f, "包 \"{name}\" 的待安装记录无效:{message}")
            }
            Self::Index(error) => write!(f, "包索引无效:{}", error.field),
            Self::Fetch(error) => write!(f, "{error}"),
            Self::Deps(error) => write!(f, "{error}"),
            Self::MissingDeps {
                required_by,
                missing,
            } => {
                let list = missing
                    .iter()
                    .map(|m| format!("\"{m}\""))
                    .collect::<Vec<_>>()
                    .join(", ");
                let command = missing.join(" ");
                write!(
                    f,
                    "包 \"{required_by}\" 缺少依赖:{list};请先运行 `mpv-config pkg install {command}`"
                )
            }
            Self::AlreadyInstalled { name, version } => write!(
                f,
                "包 \"{name}\" 已安装(v{version});如需更新请运行 `mpv-config pkg update {name}`"
            ),
            Self::NotInstalled { name } => write!(
                f,
                "包 \"{name}\" 未安装(检查 packages.lock;如锁文件损坏可删除后重新安装)"
            ),
            Self::CommandUnavailable { command, hint } => {
                write!(f, "缺少命令 {command}:{hint}")
            }
            Self::CommandFailed { command, detail } => {
                if detail.is_empty() {
                    write!(f, "命令 `{command}` 执行失败")
                } else {
                    write!(f, "命令 `{command}` 执行失败:{detail}")
                }
            }
            Self::Regex { pattern, message } => {
                write!(f, "过滤正则无效 {pattern:?}:{message}")
            }
            Self::Lock(error) => write!(f, "packages.lock 处理失败:{error}"),
            Self::RollbackFailed { detail } => {
                write!(f, "安装失败且回滚不完整:{detail}(请手动检查仓库内残留文件)")
            }
        }
    }
}

impl std::error::Error for LifecycleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Manifest { error, .. } => Some(error),
            Self::Index(error) => Some(error),
            Self::Fetch(error) => Some(error),
            Self::Deps(error) => Some(error),
            Self::Lock(error) => Some(error),
            _ => None,
        }
    }
}

impl From<DepError> for LifecycleError {
    fn from(error: DepError) -> Self {
        Self::Deps(error)
    }
}

impl From<FetchError> for LifecycleError {
    fn from(error: FetchError) -> Self {
        Self::Fetch(error)
    }
}

impl From<IndexError> for LifecycleError {
    fn from(error: IndexError) -> Self {
        Self::Index(error)
    }
}

impl From<LockError> for LifecycleError {
    fn from(error: LockError) -> Self {
        Self::Lock(error)
    }
}

/// Internal shorthand used by sub-modules for I/O failures with context.
pub(crate) type Result<T> = std::result::Result<T, LifecycleError>;

/// Resolve the default package cache directory (`~/.cache/mpv-config` or
/// `$MPV_CONFIG_CACHE`).
///
/// # Errors
///
/// [`LifecycleError::Fetch`] when no home directory resolves.
pub fn default_cache_dir() -> Result<PathBuf> {
    pkg::fetch::cache_dir().map_err(LifecycleError::Fetch)
}
