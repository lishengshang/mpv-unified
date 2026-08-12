//! `index.json` package index: schema, validation, and lookup.
//!
//! A local index lists every package a user can install. The document is
//! fetched from the official index repository by [`crate::fetch`] and cached
//! under the cache directory; this module only parses and validates it.
//!
//! ```json
//! {
//!   "latest_version": "0.2.0",
//!   "upgrade_zip_url": "https://github.com/<user>/mpv-config/releases/latest/download/mpv-config.zip",
//!   "changelog_url": "https://github.com/<user>/mpv-config/releases",
//!   "packages": [
//!     { "name": "evafast", "repo": "po5/evafast", "release_tag": "latest", "homepage": "https://..." }
//!   ]
//! }
//! ```
//!
//! The three top-level fields are optional app-upgrade metadata (T22):
//! `latest_version` (version the update check compares against), the zip URL
//! to download, and a changelog link. A package index without them still
//! parses; the update check then reports "no update information".
//!
//! Every field is validated at the parse boundary (package names, `owner/repo`
//! shape, release tag, homepage scheme, duplicate names); lookups after that
//! are infallible.

use std::collections::HashSet;
use std::fmt;

use serde::Deserialize;

/// One installable package in the index.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PackageEntry {
    /// Unique package name: lowercase letters, digits, hyphens.
    pub name: String,
    /// GitHub repository in `owner/repo` form.
    pub repo: String,
    /// Release tag to fetch; the special value `latest` resolves to the
    /// newest release of the repository.
    pub release_tag: String,
    /// Optional package homepage.
    #[serde(default)]
    pub homepage: Option<String>,
}

/// A parsed and validated package index.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Index {
    /// Newest app version announced by the index (`latest_version`); `None`
    /// when the index carries no update metadata.
    pub latest_version: Option<String>,
    /// Direct download URL of the newest app zip (`upgrade_zip_url`).
    pub upgrade_zip_url: Option<String>,
    /// Link to the changelog / release page (`changelog_url`).
    pub changelog_url: Option<String>,
    pub packages: Vec<PackageEntry>,
}

impl Index {
    /// Parse and validate an `index.json` document.
    ///
    /// # Errors
    ///
    /// Returns [`IndexError`] naming the offending field (`"json"` for
    /// malformed documents, `"packages"` for a missing list, `"name"`/`"repo"`/
    /// `"release_tag"`/`"homepage"` for invalid entries, `"name"` again for
    /// duplicate package names).
    pub fn parse(json: &str) -> Result<Self, IndexError> {
        let raw: RawIndex = serde_json::from_str(json).map_err(map_json_error)?;
        let mut seen = HashSet::new();
        let packages = raw
            .packages
            .into_iter()
            .map(|entry| {
                validate_name(&entry.name)?;
                validate_repo(&entry.repo)?;
                validate_release_tag(&entry.release_tag)?;
                if let Some(homepage) = &entry.homepage {
                    validate_homepage(homepage)?;
                }
                if !seen.insert(entry.name.clone()) {
                    return Err(IndexError::new(
                        "name",
                        format!("duplicate package {:?}", entry.name),
                    ));
                }
                Ok(PackageEntry {
                    name: entry.name,
                    repo: entry.repo,
                    release_tag: entry.release_tag,
                    homepage: entry.homepage,
                })
            })
            .collect::<Result<Vec<_>, IndexError>>()?;
        Ok(Self {
            latest_version: raw.latest_version,
            upgrade_zip_url: raw.upgrade_zip_url,
            changelog_url: raw.changelog_url,
            packages,
        })
    }

    /// Look up a package entry by exact name.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&PackageEntry> {
        self.packages.iter().find(|entry| entry.name == name)
    }
}

/// Index validation failure carrying the offending field name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexError {
    pub field: &'static str,
    pub message: String,
}

impl IndexError {
    fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for IndexError {}

/// Unvalidated mirror of [`Index`] straight off the JSON document.
#[derive(Debug, Deserialize)]
struct RawIndex {
    #[serde(default)]
    latest_version: Option<String>,
    #[serde(default)]
    upgrade_zip_url: Option<String>,
    #[serde(default)]
    changelog_url: Option<String>,
    packages: Vec<RawEntry>,
}

#[derive(Debug, Deserialize)]
struct RawEntry {
    name: String,
    repo: String,
    release_tag: String,
    #[serde(default)]
    homepage: Option<String>,
}

/// Package names: non-empty, lowercase ASCII letters/digits/hyphens,
/// starting and ending with a letter or digit (same rule as
/// [`crate::manifest`]).
fn validate_name(name: &str) -> Result<(), IndexError> {
    if name.is_empty() {
        return Err(IndexError::new("name", "must not be empty"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(IndexError::new(
            "name",
            format!("{name:?}: must be lowercase letters, digits, or hyphens"),
        ));
    }
    let first = name.chars().next();
    let last = name.chars().next_back();
    if !matches!(first, Some(c) if c.is_ascii_alphanumeric())
        || !matches!(last, Some(c) if c.is_ascii_alphanumeric())
    {
        return Err(IndexError::new(
            "name",
            format!("{name:?}: must start and end with a letter or digit"),
        ));
    }
    Ok(())
}

/// Repositories are `owner/repo`; both halves non-empty and restricted to
/// ASCII letters, digits, `-`, `_`, `.` (safe for URL interpolation into
/// `api.github.com/repos/{owner}/{repo}`).
fn validate_repo(repo: &str) -> Result<(), IndexError> {
    let mut parts = repo.split('/');
    let owner = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default();
    let ok = !owner.is_empty()
        && !name.is_empty()
        && parts.next().is_none()
        && owner
            .chars()
            .chain(name.chars())
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !ok {
        return Err(IndexError::new(
            "repo",
            format!("{repo:?}: expected `owner/repo` with letters, digits, `-`, `_`, `.`"),
        ));
    }
    Ok(())
}

/// Release tags are non-empty and free of `/`, whitespace, and control
/// characters (`latest` is the special alias for the newest release).
fn validate_release_tag(tag: &str) -> Result<(), IndexError> {
    if tag.is_empty()
        || tag
            .chars()
            .any(|c| c == '/' || c.is_whitespace() || c.is_control())
    {
        return Err(IndexError::new(
            "release_tag",
            format!("{tag:?}: must be a non-empty tag without `/` or whitespace"),
        ));
    }
    Ok(())
}

/// Homepages, when present, must be absolute HTTP(S) URLs.
fn validate_homepage(homepage: &str) -> Result<(), IndexError> {
    if !(homepage.starts_with("https://") || homepage.starts_with("http://")) {
        return Err(IndexError::new(
            "homepage",
            format!("{homepage:?}: must be an http(s) URL"),
        ));
    }
    Ok(())
}

/// Convert a serde_json failure into an [`IndexError`]. Missing required
/// fields are attributed to the named field; everything else to `"json"`.
fn map_json_error(e: serde_json::Error) -> IndexError {
    let message = e.to_string();
    let field = extract_missing_field(&message).unwrap_or("json");
    IndexError::new(field, message)
}

fn extract_missing_field(message: &str) -> Option<&'static str> {
    const PREFIX: &str = "missing field `";
    let rest = message.strip_prefix(PREFIX)?;
    let name = rest.split('`').next()?;
    Some(match name {
        "packages" => "packages",
        "name" => "name",
        "repo" => "repo",
        "release_tag" => "release_tag",
        _ => "json",
    })
}

#[cfg(test)]
mod tests;
