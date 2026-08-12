//! `index.json` package index: schema, validation, and lookup.
//!
//! A local index lists every package a user can install. The document is
//! fetched from the official index repository by [`crate::fetch`] and cached
//! under the cache directory; this module only parses and validates it.
//!
//! ```json
//! {
//!   "packages": [
//!     { "name": "evafast", "repo": "po5/evafast", "release_tag": "latest", "homepage": "https://..." }
//!   ]
//! }
//! ```
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
        Ok(Self { packages })
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
    let first = name.chars().next().expect("non-empty checked above");
    let last = name.chars().next_back().expect("non-empty checked above");
    if !first.is_ascii_alphanumeric() || !last.is_ascii_alphanumeric() {
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
    let owner = parts.next().expect("split always yields one part");
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
mod tests {
    use super::*;

    const VALID: &str = r#"{
      "packages": [
        { "name": "evafast", "repo": "po5/evafast", "release_tag": "latest", "homepage": "https://github.com/po5/evafast" },
        { "name": "uosc", "repo": "tomasklaen/uosc", "release_tag": "v6.0.0" }
      ]
    }"#;

    #[test]
    fn valid_index_parses_all_entries() {
        let index = Index::parse(VALID).expect("valid index");
        assert_eq!(index.packages.len(), 2);
        let evafast = &index.packages[0];
        assert_eq!(evafast.name, "evafast");
        assert_eq!(evafast.repo, "po5/evafast");
        assert_eq!(evafast.release_tag, "latest");
        assert_eq!(
            evafast.homepage.as_deref(),
            Some("https://github.com/po5/evafast")
        );
        let uosc = &index.packages[1];
        assert_eq!(uosc.release_tag, "v6.0.0");
        assert_eq!(uosc.homepage, None);
    }

    #[test]
    fn find_returns_matching_entry() {
        let index = Index::parse(VALID).expect("valid index");
        let found = index.find("evafast").expect("evafast is in the index");
        assert_eq!(found.repo, "po5/evafast");
        assert_eq!(found.release_tag, "latest");
    }

    #[test]
    fn find_returns_none_for_unknown_name() {
        let index = Index::parse(VALID).expect("valid index");
        assert!(index.find("does-not-exist").is_none());
    }

    #[test]
    fn invalid_name_is_rejected() {
        for name in ["EvaFast", "evafast!", "-evafast", "evafast-"] {
            let json = format!(
                r#"{{ "packages": [ {{ "name": "{name}", "repo": "po5/evafast", "release_tag": "latest" }} ] }}"#
            );
            let err = Index::parse(&json).expect_err("invalid name must fail");
            assert_eq!(err.field, "name", "name {name:?}");
        }
    }

    #[test]
    fn invalid_repo_is_rejected() {
        for repo in [
            "po5",
            "po5/",
            "/evafast",
            "po5/evafast/extra",
            "po5/evafast!",
        ] {
            let json = format!(
                r#"{{ "packages": [ {{ "name": "evafast", "repo": "{repo}", "release_tag": "latest" }} ] }}"#
            );
            let err = Index::parse(&json).expect_err("invalid repo must fail");
            assert_eq!(err.field, "repo", "repo {repo:?}");
            assert!(
                err.message.contains("owner/repo"),
                "message for {repo:?}: {}",
                err.message
            );
        }
    }

    #[test]
    fn invalid_release_tag_is_rejected() {
        for tag in ["", "v1.0/rc1", "v 1.0"] {
            let json = format!(
                r#"{{ "packages": [ {{ "name": "evafast", "repo": "po5/evafast", "release_tag": "{tag}" }} ] }}"#
            );
            let err = Index::parse(&json).expect_err("invalid release_tag must fail");
            assert_eq!(err.field, "release_tag", "tag {tag:?}");
        }
    }

    #[test]
    fn invalid_homepage_is_rejected() {
        let json = r#"{ "packages": [ { "name": "evafast", "repo": "po5/evafast", "release_tag": "latest", "homepage": "ftp://x" } ] }"#;
        let err = Index::parse(json).expect_err("invalid homepage must fail");
        assert_eq!(err.field, "homepage");
    }

    #[test]
    fn duplicate_names_are_rejected() {
        let json = r#"{ "packages": [
          { "name": "evafast", "repo": "po5/evafast", "release_tag": "latest" },
          { "name": "evafast", "repo": "po5/evafast", "release_tag": "v2" }
        ] }"#;
        let err = Index::parse(json).expect_err("duplicate name must fail");
        assert_eq!(err.field, "name");
        assert!(err.message.contains("duplicate"), "{}", err.message);
    }

    #[test]
    fn empty_packages_is_valid() {
        let index = Index::parse(r#"{ "packages": [] }"#).expect("empty index is valid");
        assert!(index.packages.is_empty());
    }

    #[test]
    fn missing_packages_field_is_rejected() {
        let err = Index::parse("{}").expect_err("missing packages must fail");
        assert_eq!(err.field, "packages");
    }

    #[test]
    fn malformed_json_is_rejected() {
        let err = Index::parse("not json").expect_err("malformed json must fail");
        assert_eq!(err.field, "json");
    }

    #[test]
    fn missing_entry_field_is_rejected() {
        let json = r#"{ "packages": [ { "repo": "po5/evafast", "release_tag": "latest" } ] }"#;
        let err = Index::parse(json).expect_err("missing name must fail");
        assert_eq!(err.field, "name");
    }
}
