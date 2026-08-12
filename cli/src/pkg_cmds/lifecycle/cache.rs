//! Caching a generated/local manifest under `cache_dir/packages/<name>-<version>/`.
//!
//! The lock file records only name/version/files, but conflict checks and
//! updates need the full manifest of every installed package. Install and
//! update therefore mirror the manifest at the standard cache location (the
//! same layout `pkg::fetch` uses for downloaded archives). The public
//! `Manifest` type deserializes but does not serialize, so a small mirror
//! struct carries the fields out.

use std::fs;
use std::path::Path;

use pkg::manifest::Manifest;

use super::error::{LifecycleError, Result};

/// Serialization mirror of [`Manifest`] (public type is Deserialize-only).
#[derive(serde::Serialize)]
struct ManifestDump<'a> {
    name: &'a str,
    version: String,
    description: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    homepage: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<&'a str>,
    platform: String,
    requires: &'a [String],
    conflicts: &'a [String],
    files: Vec<FileDump<'a>>,
    config: &'a [String],
}

#[derive(serde::Serialize)]
struct FileDump<'a> {
    src: &'a str,
    dest: &'a str,
}

/// Write `manifest` at `cache_dir/packages/<name>-<version>/package.yaml`
/// unless it is already there (idempotent for repeated runs).
///
/// # Errors
///
/// [`LifecycleError`] on filesystem or serialization failures.
pub(crate) fn cache_manifest(cache_dir: &Path, manifest: &Manifest) -> Result<()> {
    let dir = cache_dir
        .join("packages")
        .join(format!("{}-{}", manifest.name, manifest.version));
    fs::create_dir_all(&dir)
        .map_err(|e| LifecycleError::io(format!("创建 {}", dir.display()), e))?;
    let path = dir.join("package.yaml");
    if path.is_file() {
        return Ok(());
    }
    let dump = ManifestDump {
        name: &manifest.name,
        version: manifest.version.to_string(),
        description: &manifest.description,
        author: manifest.author.as_deref(),
        homepage: manifest.homepage.as_deref(),
        license: manifest.license.as_deref(),
        platform: manifest.platform.as_str().to_owned(),
        requires: &manifest.requires,
        conflicts: &manifest.conflicts,
        files: manifest
            .files
            .iter()
            .map(|entry| FileDump {
                src: &entry.src,
                dest: &entry.dest,
            })
            .collect(),
        config: &manifest.config,
    };
    let yaml = serde_yaml::to_string(&dump)
        .map_err(|e| LifecycleError::io("序列化 manifest", std::io::Error::other(e.to_string())))?;
    fs::write(&path, yaml).map_err(|e| LifecycleError::io(format!("写入 {}", path.display()), e))
}
