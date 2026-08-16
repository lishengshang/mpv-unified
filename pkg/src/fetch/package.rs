//! Fetching one package archive from GitHub Releases.
//!
//! Flow: release lookup (`latest` or a pinned tag) → asset selection
//! (zip preferred over tar.gz) → download → extraction (`unzip`/`tar`
//! subprocesses, with a zip-slip entry pre-scan) → manifest validation
//! (parses, and its `name` matches the index entry) → atomic placement at
//! `cache_dir/packages/<name>-<version>/`.
//!
//! On any failure the temporary work directory is removed; a half-cached
//! package is never left behind.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::fetch::{io_error, unique_suffix, FetchError, Fetcher};
use crate::index::PackageEntry;
use crate::manifest;

const API_BASE: &str = "https://api.github.com/repos";

/// A successfully fetched, validated, and extracted package on disk.
#[derive(Debug)]
pub struct PackageArchive {
    /// Extracted package directory: `cache_dir/packages/<name>-<version>/`.
    pub dir: PathBuf,
    /// The validated manifest found inside the archive.
    pub manifest: manifest::Manifest,
}

/// Fetch `entry` from GitHub Releases into `cache_dir`.
///
/// # Errors
///
/// Any [`FetchError`]: transport, malformed release payload, missing or
/// unsafe asset, invalid/mismatched manifest, filesystem failure.
pub fn fetch_package(
    entry: &PackageEntry,
    fetcher: &dyn Fetcher,
    cache_dir: &Path,
) -> Result<PackageArchive, FetchError> {
    let work = cache_dir.join(format!(".fetch-{}", unique_suffix()));
    let result = run(entry, fetcher, cache_dir, &work);
    if result.is_err() {
        let _ = fs::remove_dir_all(&work);
    }
    result
}

fn run(
    entry: &PackageEntry,
    fetcher: &dyn Fetcher,
    cache_dir: &Path,
    work: &Path,
) -> Result<PackageArchive, FetchError> {
    fs::create_dir_all(work).map_err(io_error(format!("create {}", work.display())))?;

    let release: serde_json::Value = serde_json::from_slice(&fetcher.get(&release_url(entry))?)
        .map_err(|e| FetchError::Json {
            context: format!("release for {}", entry.repo),
            source: e,
        })?;
    let asset = pick_asset(&release, entry)?;

    let archive_path = work.join(&asset.name);
    fs::write(&archive_path, fetcher.get(&asset.url)?)
        .map_err(io_error(format!("write {}", archive_path.display())))?;

    let extracted = work.join("extracted");
    fs::create_dir_all(&extracted).map_err(io_error(format!("create {}", extracted.display())))?;
    extract(&archive_path, &extracted)?;

    let manifest_path = find_manifest(&extracted)?;
    let parsed = manifest::Manifest::parse(
        &fs::read_to_string(&manifest_path)
            .map_err(io_error(format!("read {}", manifest_path.display())))?,
    )
    .map_err(FetchError::Manifest)?;
    if parsed.name != entry.name {
        return Err(FetchError::NameMismatch {
            expected: entry.name.clone(),
            actual: parsed.name,
        });
    }

    let packages_dir = cache_dir.join("packages");
    fs::create_dir_all(&packages_dir)
        .map_err(io_error(format!("create {}", packages_dir.display())))?;
    let final_dir = packages_dir.join(format!("{}-{}", parsed.name, parsed.version));
    if final_dir.exists() {
        fs::remove_dir_all(&final_dir).map_err(io_error(format!(
            "remove stale cache {}",
            final_dir.display()
        )))?;
    }
    fs::rename(&extracted, &final_dir).map_err(io_error(format!(
        "move extracted package to {}",
        final_dir.display()
    )))?;
    let _ = fs::remove_dir_all(work);
    Ok(PackageArchive {
        dir: final_dir,
        manifest: parsed,
    })
}

fn release_url(entry: &PackageEntry) -> String {
    let suffix = if entry.release_tag == "latest" {
        "latest".to_owned()
    } else {
        format!("tags/{}", entry.release_tag)
    };
    format!("{API_BASE}/{}/releases/{suffix}", entry.repo)
}

struct Asset {
    name: String,
    url: String,
}

fn pick_asset(release: &serde_json::Value, entry: &PackageEntry) -> Result<Asset, FetchError> {
    let assets = release
        .get("assets")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| FetchError::BadRelease {
            repo: entry.repo.clone(),
        })?;
    let mut zips = Vec::new();
    let mut tarballs = Vec::new();
    for asset in assets {
        let Some(name) = asset.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(url) = asset
            .get("browser_download_url")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        if name.ends_with(".zip") {
            zips.push(Asset {
                name: name.to_owned(),
                url: url.to_owned(),
            });
        } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            tarballs.push(Asset {
                name: name.to_owned(),
                url: url.to_owned(),
            });
        }
    }
    zips.into_iter()
        .next()
        .or_else(|| tarballs.into_iter().next())
        .ok_or_else(|| {
            let available = assets
                .iter()
                .filter_map(|a| a.get("name").and_then(serde_json::Value::as_str))
                .map(str::to_owned)
                .collect();
            FetchError::NoAsset {
                repo: entry.repo.clone(),
                tag: entry.release_tag.clone(),
                available,
            }
        })
}

const UNZIP_HINT: &str = "install unzip (Linux: `apt install unzip`, macOS: `brew install unzip`)";
const TAR_HINT: &str = "install tar (Windows 10+ ships tar.exe; Linux distributions preinstall it)";

/// Per-platform handler for `.zip` archives.
///
/// Windows has no `unzip`, but its bundled bsdtar (`tar.exe`) reads zip
/// archives as well as tarballs; Unix systems keep Info-ZIP `unzip`, whose
/// GNU tar cannot read zip files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZipTool {
    /// Info-ZIP `unzip`: `-Z1` lists, `-o -q <archive> -d <dest>` extracts.
    Unzip,
    /// bsdtar: `-tf` lists, `-xf <archive> -C <dest>` extracts.
    Tar,
}

impl ZipTool {
    /// The handler for the current platform.
    pub(crate) fn select() -> Self {
        if cfg!(windows) {
            Self::Tar
        } else {
            Self::Unzip
        }
    }

    pub(crate) fn command(&self) -> &'static str {
        match self {
            Self::Unzip => "unzip",
            Self::Tar => "tar",
        }
    }

    fn hint(&self) -> &'static str {
        match self {
            Self::Unzip => UNZIP_HINT,
            Self::Tar => TAR_HINT,
        }
    }

    pub(crate) fn list_args<'a>(&self, archive: &'a str) -> Vec<&'a str> {
        match self {
            Self::Unzip => vec!["-Z1", archive],
            Self::Tar => vec!["-tf", archive],
        }
    }

    pub(crate) fn extract_args<'a>(&self, archive: &'a str, dest: &'a str) -> Vec<&'a str> {
        match self {
            Self::Unzip => vec!["-o", "-q", archive, "-d", dest],
            Self::Tar => vec!["-xf", archive, "-C", dest],
        }
    }
}

/// Extract an archive into `dest`, refusing entries that escape the
/// directory (zip-slip / tar-slip) before any bytes hit disk.
pub(crate) fn extract(archive: &Path, dest: &Path) -> Result<(), FetchError> {
    let name = archive
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let archive_str = archive.to_string_lossy().into_owned();
    let dest_str = dest.to_string_lossy().into_owned();
    if name.ends_with(".zip") {
        let tool = ZipTool::select();
        let listing = run_capture(tool.command(), &tool.list_args(&archive_str), tool.hint())?;
        validate_entries(&name, listing)?;
        run_status(
            tool.command(),
            &tool.extract_args(&archive_str, &dest_str),
            tool.hint(),
        )
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let listing = run_capture("tar", &["-tzf", &archive_str], TAR_HINT)?;
        validate_entries(&name, listing)?;
        run_status("tar", &["-xzf", &archive_str, "-C", &dest_str], TAR_HINT)
    } else {
        Err(FetchError::UnsupportedArchive { name })
    }
}

/// Every listed entry must stay inside the extraction directory: no `..`
/// components, no absolute paths, no backslashes (Windows separator).
fn validate_entries(archive: &str, listing: Vec<u8>) -> Result<(), FetchError> {
    for raw in String::from_utf8_lossy(&listing).lines() {
        let entry = raw.strip_prefix("./").unwrap_or(raw);
        if entry.is_empty() {
            continue;
        }
        let unsafe_entry = entry.starts_with('/')
            || entry.contains('\\')
            || entry.split('/').any(|component| component == "..");
        if unsafe_entry {
            return Err(FetchError::UnsafeArchive {
                archive: archive.to_owned(),
                entry: entry.to_owned(),
            });
        }
    }
    Ok(())
}

fn find_manifest(dir: &Path) -> Result<PathBuf, FetchError> {
    let mut found = Vec::new();
    collect_manifests(dir, &mut found)?;
    let Some(only) = found.pop() else {
        return Err(FetchError::MissingManifest {
            dir: dir.display().to_string(),
        });
    };
    if found.is_empty() {
        return Ok(only);
    }
    found.push(only);
    Err(FetchError::AmbiguousManifest {
        dir: dir.display().to_string(),
        found: found.iter().map(|p| p.display().to_string()).collect(),
    })
}

fn collect_manifests(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), FetchError> {
    for item in fs::read_dir(dir).map_err(io_error(format!("read {}", dir.display())))? {
        let item = item.map_err(io_error(format!("read {}", dir.display())))?;
        let file_type = item
            .file_type()
            .map_err(io_error("stat entry".to_owned()))?;
        let path = item.path();
        if file_type.is_dir() {
            collect_manifests(&path, out)?;
        } else if file_type.is_file()
            && path.file_name().and_then(|n| n.to_str()) == Some("package.yaml")
        {
            out.push(path);
        }
    }
    Ok(())
}

fn run_capture(
    command: &'static str,
    args: &[&str],
    hint: &'static str,
) -> Result<Vec<u8>, FetchError> {
    let out = run_subprocess(command, args, hint)?;
    if out.status.success() {
        return Ok(out.stdout);
    }
    Err(process_error(command, out))
}

fn run_status(command: &'static str, args: &[&str], hint: &'static str) -> Result<(), FetchError> {
    let out = run_subprocess(command, args, hint)?;
    if out.status.success() {
        return Ok(());
    }
    Err(process_error(command, out))
}

fn run_subprocess(
    command: &'static str,
    args: &[&str],
    hint: &'static str,
) -> Result<std::process::Output, FetchError> {
    Command::new(command).args(args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            FetchError::CommandUnavailable { command, hint }
        } else {
            FetchError::Io {
                context: format!("spawn {command}"),
                source: e,
            }
        }
    })
}

fn process_error(command: &'static str, out: std::process::Output) -> FetchError {
    FetchError::Io {
        context: format!("`{command}` exited with {}", out.status),
        source: io::Error::other(String::from_utf8_lossy(&out.stderr).trim().to_owned()),
    }
}
