//! Fetching the package index and package archives from GitHub Releases.
//!
//! HTTP is performed by shelling out to `curl` (shipped on Linux, macOS, and
//! Windows 10+) and archives are extracted with `unzip`/`tar` — zero native
//! networking or compression dependencies. The [`Fetcher`] trait keeps every
//! offline test on the preset-bytes [`MockFetcher`].
//!
//! # Windows note
//!
//! `curl.exe` and `tar.exe` ship with Windows 10+; `unzip` does not — zip
//! archives are therefore extracted with the bundled bsdtar on Windows
//! ([`package::ZipTool`]), while Unix keeps Info-ZIP `unzip`. The CLI
//! reports a clear [`FetchError::CommandUnavailable`] hint when a command
//! is missing.
//!
//! # Cache layout
//!
//! - `index.json` — the last validated remote index (atomic write).
//! - `packages/<name>-<version>/` — extracted, validated archives.
//!
//! The cache root defaults to `~/.cache/mpv-config` and can be overridden
//! with `MPV_CONFIG_CACHE`.

mod error;
pub mod package;

use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::index::Index;
pub(crate) use error::io_error;
pub use error::FetchError;
pub(crate) use package::extract;
pub use package::{fetch_package, PackageArchive};

/// Default remote index URL: the official index repository's latest release
/// artifact. The CLI's `--index-url` overrides it (per user-hosted indexes).
pub const DEFAULT_INDEX_URL: &str =
    "https://github.com/<user>/mpv-packages-index/releases/latest/download/index.json";

/// Reject an index URL that still carries the `<user>` placeholder before it
/// reaches the network — a 404 from `https://github.com/<user>/...` says
/// nothing to the user.
///
/// # Errors
///
/// [`FetchError::IndexUnconfigured`] when `url` contains the placeholder.
pub fn ensure_index_url(url: &str) -> Result<(), FetchError> {
    if url.contains("<user>") {
        Err(FetchError::IndexUnconfigured)
    } else {
        Ok(())
    }
}

/// HTTP abstraction over fetching a URL. Real traffic goes through
/// [`HttpFetcher`] (a `curl` subprocess); tests use [`MockFetcher`].
pub trait Fetcher {
    /// Fetch the resource at `url`.
    ///
    /// # Errors
    ///
    /// [`FetchError::Http`] on transport or status failures, or
    /// [`FetchError::CommandUnavailable`] when `curl` is missing.
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError>;
}

/// Real fetcher: `curl -fsSL` (follow redirects, fail on 4xx/5xx, silent).
#[derive(Debug, Default)]
pub struct HttpFetcher;

impl Fetcher for HttpFetcher {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        let out = run_command(
            "curl",
            &["-fsSL", "--connect-timeout", "15", "--max-time", "60", url],
            "install curl or add it to PATH (Windows 10+ ships curl.exe)",
        )?;
        if out.status.success() {
            return Ok(out.stdout);
        }
        Err(FetchError::Http {
            url: url.to_owned(),
            status: out.status.code(),
            detail: String::from_utf8_lossy(&out.stderr).trim().to_owned(),
        })
    }
}

/// Test double serving preset byte payloads per URL; unconfigured URLs fail
/// with a mock 404. Also records every requested URL in order.
#[derive(Debug, Default)]
pub struct MockFetcher {
    responses: HashMap<String, Vec<u8>>,
    requests: RefCell<Vec<String>>,
}

impl MockFetcher {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Serve `body` for `url`.
    #[must_use]
    pub fn with(mut self, url: &str, body: Vec<u8>) -> Self {
        self.responses.insert(url.to_owned(), body);
        self
    }

    /// URLs requested so far, in request order (assertion aid).
    #[must_use]
    pub fn requested(&self) -> Vec<String> {
        self.requests.borrow().clone()
    }
}

impl Fetcher for MockFetcher {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.requests.borrow_mut().push(url.to_owned());
        match self.responses.get(url) {
            Some(body) => Ok(body.clone()),
            None => Err(FetchError::Http {
                url: url.to_owned(),
                status: Some(404),
                detail: "mock: no response configured for this URL".to_owned(),
            }),
        }
    }
}

/// Resolve the local cache directory: `$MPV_CONFIG_CACHE`, else
/// `$XDG_CACHE_HOME/mpv-config`, else `~/.cache/mpv-config` (Windows:
/// `%LOCALAPPDATA%/mpv-config`).
///
/// # Errors
///
/// [`FetchError::NoCacheDir`] when no environment variable resolves.
pub fn cache_dir() -> Result<PathBuf, FetchError> {
    if let Some(dir) = env::var_os("MPV_CONFIG_CACHE") {
        return Ok(PathBuf::from(dir));
    }
    if let Some(xdg) = env::var_os("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(xdg).join("mpv-config"));
    }
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        return Ok(PathBuf::from(local).join("mpv-config"));
    }
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .ok_or(FetchError::NoCacheDir)?;
    Ok(PathBuf::from(home).join(".cache").join("mpv-config"))
}

/// Fetch the remote `index.json` and return its text.
///
/// # Errors
///
/// Any [`FetchError`]; notably [`FetchError::Utf8`] for non-UTF-8 bodies.
pub fn fetch_index(fetcher: &dyn Fetcher, url: &str) -> Result<String, FetchError> {
    let bytes = fetcher.get(url)?;
    String::from_utf8(bytes).map_err(|_| FetchError::Utf8 {
        url: url.to_owned(),
    })
}

/// Fetch and atomically store the index at `cache_dir/index.json`.
///
/// The document is validated with [`Index::parse`] before anything is
/// written; an invalid index never replaces a good cached one. Returns the
/// written path.
///
/// # Errors
///
/// Any [`FetchError`]; [`FetchError::Index`] when the remote document is
/// invalid (nothing written).
pub fn update_index(
    fetcher: &dyn Fetcher,
    url: &str,
    cache_dir: &Path,
) -> Result<PathBuf, FetchError> {
    let text = fetch_index(fetcher, url)?;
    Index::parse(&text).map_err(FetchError::Index)?;
    fs::create_dir_all(cache_dir).map_err(io_error(format!(
        "create cache dir {}",
        cache_dir.display()
    )))?;
    let target = cache_dir.join("index.json");
    let tmp = cache_dir.join(format!("index.json.tmp{}", unique_suffix()));
    fs::write(&tmp, &text).map_err(io_error(format!("write {}", tmp.display())))?;
    fs::rename(&tmp, &target).map_err(io_error(format!("move {} into place", tmp.display())))?;
    Ok(target)
}

/// Run a subprocess capturing output; maps a missing binary to
/// [`FetchError::CommandUnavailable`].
fn run_command(
    command: &'static str,
    args: &[&str],
    hint: &'static str,
) -> Result<std::process::Output, FetchError> {
    Command::new(command)
        .args(args)
        .output()
        .map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => FetchError::CommandUnavailable { command, hint },
            _ => FetchError::Io {
                context: format!("spawn {command}"),
                source: e,
            },
        })
}

/// Unique per-process suffix for temp artifacts (pid + counter).
pub(crate) fn unique_suffix() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    format!(
        "{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) mod testutil;
