//! Layer-file loading: read, parse and directive-evaluate one conf layer.

use super::error::{cond_error, GenError};
use core::cond::evaluate;
use core::conf::{parse, ConfDoc};
use core::merge::MergeLayer;
use core::platform::Platform;
use std::fs;
use std::io;
use std::path::Path;

/// Lowercase file-name stem of a platform (`linux` / `windows` / `macos`).
pub(crate) fn platform_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Linux => "linux",
        Platform::Windows => "windows",
        Platform::MacOS => "macos",
    }
}

/// Read and evaluate one layer file. `required` turns a missing file into a
/// [`GenError::MissingLayer`]; optional layers yield `None` instead.
pub(crate) fn load_layer(
    root: &Path,
    rel: &str,
    platform: Platform,
    required: bool,
) -> Result<Option<MergeLayer>, GenError> {
    let path = root.join(rel);
    let Some(text) = read_optional(&path)? else {
        if required {
            return Err(GenError::MissingLayer { path });
        }
        return Ok(None);
    };
    Ok(Some(parse_layer(rel, &text, platform)?))
}

/// Parse `text`, evaluate its conditional directives for `platform`, and
/// wrap the result as a merge layer named `name`.
fn parse_layer(name: &str, text: &str, platform: Platform) -> Result<MergeLayer, GenError> {
    let doc = parse(text).map_err(|error| GenError::Parse {
        layer: name.to_owned(),
        line: error.line,
        message: error.message,
    })?;
    let entries = evaluate(&doc, platform).map_err(|error| cond_error(name, error))?;
    Ok(MergeLayer::new(
        name,
        ConfDoc {
            entries,
            ends_with_newline: doc.ends_with_newline,
        },
    ))
}

/// Read every `*.conf` fragment under `config.d/packages/`, name-sorted, as
/// evaluated merge layers. A missing directory is an empty package layer.
pub(crate) fn load_packages(root: &Path, platform: Platform) -> Result<Vec<MergeLayer>, GenError> {
    let dir = root.join("config.d").join("packages");
    let Some(mut names) = read_dir_conf_names(&dir)? else {
        return Ok(Vec::new());
    };
    names.sort();
    let mut layers = Vec::with_capacity(names.len());
    for name in names {
        let path = dir.join(&name);
        let text =
            read_optional(&path)?.ok_or_else(|| GenError::MissingLayer { path: path.clone() })?;
        layers.push(parse_layer(&format!("package:{name}"), &text, platform)?);
    }
    Ok(layers)
}

/// Read a file, treating a missing file as `None`; other I/O failures
/// propagate as [`GenError::Io`].
fn read_optional(path: &Path) -> Result<Option<String>, GenError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(GenError::Io {
            action: "读取",
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// `*.conf` file names inside `dir` (files only, UTF-8 names), or `None`
/// when the directory does not exist.
fn read_dir_conf_names(dir: &Path) -> Result<Option<Vec<String>>, GenError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(GenError::Io {
                action: "读取目录",
                path: dir.to_path_buf(),
                source,
            });
        }
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| GenError::Io {
            action: "读取目录",
            path: dir.to_path_buf(),
            source,
        })?;
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if name.ends_with(".conf") && entry.path().is_file() {
            names.push(name.to_owned());
        }
    }
    Ok(Some(names))
}
