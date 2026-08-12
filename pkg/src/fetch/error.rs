//! Fetching failures: every way index/package fetching can fail, one enum.

use std::fmt;
use std::io;

use crate::index::IndexError;
use crate::manifest;

/// Failure of index or package fetching, extraction, or validation.
#[derive(Debug)]
pub enum FetchError {
    /// A required external command (`curl`, `unzip`, `tar`) is not on PATH.
    CommandUnavailable {
        command: &'static str,
        hint: &'static str,
    },
    /// The HTTP fetch failed: non-zero `curl` exit or a mock 404.
    Http {
        url: String,
        /// `curl` exit code / HTTP status when known.
        status: Option<i32>,
        detail: String,
    },
    /// The response body was not valid UTF-8.
    Utf8 { url: String },
    /// The GitHub API (or index) payload failed to parse as JSON.
    Json {
        context: String,
        source: serde_json::Error,
    },
    /// The remote index failed validation.
    Index(IndexError),
    /// The manifest inside an archive failed validation.
    Manifest(manifest::PackageError),
    /// The release payload is missing the `assets` list.
    BadRelease { repo: String },
    /// The release has no `.zip`/`.tar.gz` asset.
    NoAsset {
        repo: String,
        tag: String,
        available: Vec<String>,
    },
    /// The archive is not a supported type (`.zip`/`.tar.gz`/`.tgz`).
    UnsupportedArchive { name: String },
    /// An archive entry would escape the extraction directory (zip-slip).
    UnsafeArchive { archive: String, entry: String },
    /// No `package.yaml` was found anywhere in the archive.
    MissingManifest { dir: String },
    /// More than one `package.yaml` was found.
    AmbiguousManifest { dir: String, found: Vec<String> },
    /// The manifest name does not match the index entry name.
    NameMismatch { expected: String, actual: String },
    /// Neither `$MPV_CONFIG_CACHE`, `$HOME`, nor `$USERPROFILE` resolved.
    NoCacheDir,
    /// Filesystem failure while writing/renaming cache artifacts.
    Io { context: String, source: io::Error },
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandUnavailable { command, hint } => {
                write!(f, "`{command}` not found on PATH: {hint}")
            }
            Self::Http { url, status, detail } => write!(
                f,
                "GET {url} failed (status {}){}{}",
                status.map_or_else(|| "?".to_owned(), |s| s.to_string()),
                if detail.is_empty() { "" } else { ": " },
                detail
            ),
            Self::Utf8 { url } => write!(f, "GET {url} returned non-UTF-8 bytes"),
            Self::Json { context, source } => {
                write!(f, "invalid JSON in {context}: {source}")
            }
            Self::Index(err) => write!(f, "index: {err}"),
            Self::Manifest(err) => write!(f, "manifest: {err}"),
            Self::BadRelease { repo } => {
                write!(f, "release payload for {repo} has no `assets` list")
            }
            Self::NoAsset {
                repo,
                tag,
                available,
            } => write!(
                f,
                "release {repo}@{tag} has no zip/tar.gz asset (available: {})",
                if available.is_empty() {
                    "none".to_owned()
                } else {
                    available.join(", ")
                }
            ),
            Self::UnsupportedArchive { name } => {
                write!(f, "unsupported archive {name:?}: expected .zip or .tar.gz")
            }
            Self::UnsafeArchive { archive, entry } => write!(
                f,
                "archive {archive:?} contains unsafe entry {entry:?} escaping the extraction directory"
            ),
            Self::MissingManifest { dir } => {
                write!(f, "no package.yaml found in {dir}")
            }
            Self::AmbiguousManifest { dir, found } => write!(
                f,
                "multiple package.yaml files in {dir}: {}",
                found.join(", ")
            ),
            Self::NameMismatch { expected, actual } => write!(
                f,
                "manifest name {actual:?} does not match index entry {expected:?}"
            ),
            Self::NoCacheDir => write!(
                f,
                "cannot resolve a cache directory: set MPV_CONFIG_CACHE or HOME"
            ),
            Self::Io { context, source } => write!(f, "{context}: {source}"),
        }
    }
}

impl std::error::Error for FetchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json { source, .. } => Some(source),
            Self::Index(err) => Some(err),
            Self::Manifest(err) => Some(err),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Convenience for wrapping `std::io::Error`s in [`FetchError::Io`].
pub(crate) fn io_error(context: impl Into<String>) -> impl FnOnce(io::Error) -> FetchError {
    let context = context.into();
    move |source| FetchError::Io { context, source }
}
