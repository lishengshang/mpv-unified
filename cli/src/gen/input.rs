//! input.conf assembly: base file plus platform variant, directive-evaluated.

use super::error::{cond_error, GenError};
use super::layers::platform_name;
use core::cond::evaluate;
use core::conf::{serialize, ConfDoc, Entry};
use core::platform::Platform;
use std::path::Path;

/// Assemble `input.conf`: the base file plus, when present, the platform
/// variant `config/{platform}.input.conf`, each directive-evaluated.
/// `None` when neither file exists.
///
/// input.conf is not mpv.conf syntax (keys like `[` and `]` would be read
/// as profile headers), so its lines are wrapped as opaque comments and
/// only the `#@if`/`#@else`/`#@endif` directives are interpreted; every
/// other line passes through byte-for-byte.
pub(crate) fn build_input(root: &Path, platform: Platform) -> Result<Option<String>, GenError> {
    let mut out = String::new();
    for rel in [
        "config/input.conf",
        &format!("config/{}.input.conf", platform_name(platform)),
    ] {
        let path = root.join(rel);
        let Some(text) = read_optional(&path)? else {
            continue;
        };
        let evaluated = evaluate_input_text(rel, &text, platform)?;
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&evaluated);
    }
    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some(out))
}

/// Evaluate the directives of `text` as an opaque input.conf document and
/// serialize the surviving lines.
fn evaluate_input_text(name: &str, text: &str, platform: Platform) -> Result<String, GenError> {
    let ends_with_newline = text.ends_with('\n');
    let body = text.strip_suffix('\n').unwrap_or(text);
    let entries: Vec<Entry> = body
        .split('\n')
        .map(|line| Entry::Comment {
            text: line.to_owned(),
        })
        .collect();
    let doc = ConfDoc {
        entries,
        ends_with_newline,
    };
    let entries = evaluate(&doc, platform).map_err(|error| cond_error(name, error))?;
    Ok(serialize(&ConfDoc {
        entries,
        ends_with_newline,
    }))
}

/// Read a file, treating a missing file as `None`; other I/O failures
/// propagate as [`GenError::Io`].
fn read_optional(path: &Path) -> Result<Option<String>, GenError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(GenError::Io {
            action: "读取",
            path: path.to_path_buf(),
            source,
        }),
    }
}
