//! `package.yaml` manifest schema, parser, and validation.
//!
//! A manifest describes one installable package: identity (`name`, `version`,
//! `description`, ...), a platform constraint, dependency/conflict lists, the
//! files to install (`files: [{src, dest}]`, where `dest` is a `~~/`-prefixed
//! repository-relative path such as `~~/scripts`), and optional `config`
//! snippets merged into the package config layer by the generator.
//!
//! Parsing is two-phase: YAML is deserialized into an unvalidated raw shape,
//! then [`Manifest::parse`] validates every field and reports failures as
//! [`PackageError`] carrying the offending field name. Nothing panics.

use std::fmt;
use std::fs;
use std::path::Path;

use serde::Deserialize;

/// Top-level directories a `dest` may target (the `~~/` prefix is the repo
/// root). Subpaths such as `~~/scripts/file-browser` are allowed; the first
/// component must be one of these.
const KNOWN_DEST_DIRS: &[&str] = &[
    "scripts",
    "shaders",
    "script-opts",
    "script-modules",
    "fonts",
    "config.d",
    "icc",
    "osc-style",
    "vs",
];

/// Target platform a package is built for.
///
/// `all` (the default) means the package installs on every platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Platform {
    #[default]
    All,
    Linux,
    Windows,
    MacOS,
}

impl Platform {
    /// YAML spelling of this variant (`"all"`, `"linux"`, ...).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Linux => "linux",
            Self::Windows => "windows",
            Self::MacOS => "macos",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Platform {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all" => Ok(Self::All),
            "linux" => Ok(Self::Linux),
            "windows" => Ok(Self::Windows),
            "macos" => Ok(Self::MacOS),
            _ => Err(()),
        }
    }
}

/// One file installed by a package: copied from `src` (package-relative) to
/// `dest` (repository-relative, `~~/`-prefixed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub src: String,
    pub dest: String,
}

/// A parsed and validated `package.yaml` manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Unique package identifier: lowercase letters, digits, hyphens.
    pub name: String,
    /// Semantic version (`major.minor.patch`).
    pub version: semver::Version,
    pub description: String,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    /// Platform constraint; `all` installs everywhere.
    pub platform: Platform,
    /// Names of packages this package depends on.
    pub requires: Vec<String>,
    /// Names of packages this package conflicts with.
    pub conflicts: Vec<String>,
    /// Install file list; never empty.
    pub files: Vec<FileEntry>,
    /// Configuration snippet lines merged into the package layer.
    pub config: Vec<String>,
}

impl Manifest {
    /// Parse and validate a `package.yaml` document.
    ///
    /// # Errors
    ///
    /// Returns [`PackageError`] with the offending field name when the YAML
    /// is malformed or any validation rule fails.
    pub fn parse(yaml: &str) -> Result<Self, PackageError> {
        let raw: RawManifest = serde_yaml::from_str(yaml).map_err(map_yaml_error)?;
        Self::from_raw(raw)
    }

    /// Parse and validate a `package.yaml` file on disk.
    ///
    /// # Errors
    ///
    /// Returns [`PackageError`] (field `"file"`) on I/O failure, or the
    /// ordinary validation error for malformed content.
    pub fn parse_file(path: &Path) -> Result<Self, PackageError> {
        let yaml = fs::read_to_string(path)
            .map_err(|e| PackageError::new("file", format!("{}: {e}", path.display())))?;
        Self::parse(&yaml)
    }

    /// Whether this package must be skipped when installing for `target`.
    ///
    /// A package constrained to another platform reports a mismatch but still
    /// parses successfully; the installer skips it instead of failing.
    #[must_use]
    pub fn platform_mismatch(&self, target: Platform) -> bool {
        match self.platform {
            Platform::All => false,
            specific => specific != target,
        }
    }

    fn from_raw(raw: RawManifest) -> Result<Self, PackageError> {
        validate_name(&raw.name)?;

        let version = semver::Version::parse(&raw.version)
            .map_err(|e| PackageError::new("version", format!("{:?}: {e}", raw.version)))?;

        let platform = match raw.platform.as_deref() {
            None => Platform::All,
            Some(s) => s.parse().map_err(|()| {
                PackageError::new(
                    "platform",
                    format!("{s:?}: expected one of all|linux|windows|macos"),
                )
            })?,
        };

        for dep in &raw.requires {
            validate_name(dep)
                .map_err(|e| PackageError::new("requires", format!("{dep:?}: {}", e.message)))?;
        }
        if raw.requires.iter().any(|dep| dep == &raw.name) {
            return Err(PackageError::new(
                "requires",
                format!("package cannot require itself ({:?})", raw.name),
            ));
        }

        for conflict in &raw.conflicts {
            validate_name(conflict).map_err(|e| {
                PackageError::new("conflicts", format!("{conflict:?}: {}", e.message))
            })?;
        }
        if raw.conflicts.iter().any(|conflict| conflict == &raw.name) {
            return Err(PackageError::new(
                "conflicts",
                format!("package cannot conflict with itself ({:?})", raw.name),
            ));
        }

        if raw.files.is_empty() {
            return Err(PackageError::new(
                "files",
                "at least one file entry is required",
            ));
        }
        let files = raw
            .files
            .into_iter()
            .enumerate()
            .map(|(i, f)| {
                validate_dest(&f.dest)
                    .map_err(|e| PackageError::new("dest", format!("files[{i}]: {}", e.message)))?;
                Ok(FileEntry {
                    src: f.src,
                    dest: f.dest,
                })
            })
            .collect::<Result<Vec<_>, PackageError>>()?;

        Ok(Self {
            name: raw.name,
            version,
            description: raw.description,
            author: raw.author,
            homepage: raw.homepage,
            license: raw.license,
            platform,
            requires: raw.requires,
            conflicts: raw.conflicts,
            files,
            config: raw.config,
        })
    }
}

/// Validation failure with the offending manifest field name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageError {
    pub field: &'static str,
    pub message: String,
}

impl PackageError {
    fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for PackageError {}

/// Unvalidated mirror of [`Manifest`] straight off the YAML document.
#[derive(Debug, Deserialize)]
struct RawManifest {
    name: String,
    version: String,
    description: String,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    platform: Option<String>,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    conflicts: Vec<String>,
    files: Vec<RawFileEntry>,
    #[serde(default)]
    config: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawFileEntry {
    src: String,
    dest: String,
}

/// Package names: non-empty, lowercase ASCII letters/digits/hyphens, starting
/// and ending with a letter or digit.
fn validate_name(name: &str) -> Result<(), PackageError> {
    if name.is_empty() {
        return Err(PackageError::new("name", "must not be empty"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(PackageError::new(
            "name",
            format!("{name:?}: must be lowercase letters, digits, or hyphens"),
        ));
    }
    let first = name.chars().next().expect("non-empty checked above");
    let last = name.chars().next_back().expect("non-empty checked above");
    if !first.is_ascii_alphanumeric() || !last.is_ascii_alphanumeric() {
        return Err(PackageError::new(
            "name",
            format!("{name:?}: must start and end with a letter or digit"),
        ));
    }
    Ok(())
}

/// `dest` must be `~~/`-prefixed with a known top-level directory and no `..`
/// or empty path components (nested subpaths such as `~~/scripts/sub` are
/// allowed).
fn validate_dest(dest: &str) -> Result<(), PackageError> {
    let rest = dest.strip_prefix("~~/").ok_or_else(|| {
        PackageError::new(
            "dest",
            format!("{dest:?}: must start with the `~~/` prefix"),
        )
    })?;
    if rest.is_empty() {
        return Err(PackageError::new(
            "dest",
            format!("{dest:?}: missing directory after `~~/`"),
        ));
    }
    let mut components = rest.split('/');
    let top = components.next().expect("rest is non-empty");
    if !KNOWN_DEST_DIRS.contains(&top) {
        return Err(PackageError::new(
            "dest",
            format!(
                "{dest:?}: unknown `~~/` directory {top:?}, expected one of {}",
                KNOWN_DEST_DIRS.join(", ")
            ),
        ));
    }
    if components.any(|c| c.is_empty() || c == "..") {
        return Err(PackageError::new(
            "dest",
            format!("{dest:?}: path must not contain empty or `..` components"),
        ));
    }
    Ok(())
}

/// Convert a serde_yaml failure into a [`PackageError`]. Missing required
/// fields are attributed to the named field; everything else to `"yaml"`.
fn map_yaml_error(e: serde_yaml::Error) -> PackageError {
    let message = e.to_string();
    let field = extract_missing_field(&message).unwrap_or("yaml");
    PackageError::new(field, message)
}

fn extract_missing_field(message: &str) -> Option<&'static str> {
    const PREFIX: &str = "missing field `";
    let rest = message.strip_prefix(PREFIX)?;
    let name = rest.split('`').next()?;
    Some(match name {
        "name" => "name",
        "version" => "version",
        "description" => "description",
        "files" => "files",
        _ => "yaml",
    })
}

#[cfg(test)]
mod tests;
