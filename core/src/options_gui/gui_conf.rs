//! Read/write of the GUI fragment `user/gui.conf`.

use super::{
    invalid, validate_value, OptionsGuiError, OptionsTable, GUI_CONF_FILE, HEADER_COMMENT,
};
use crate::conf::{self, ConfDoc, Entry};
use std::fs;
use std::io;
use std::path::Path;

/// Write the GUI fragment `user/gui.conf` from a complete form state.
///
/// Semantics (mpv.net strategy): only `(key, value)` pairs whose value
/// differs from the table default are written; keys present in an existing
/// `gui.conf` but absent from the non-default set are removed (a reset).
/// Every value is validated first — on failure nothing is written. The
/// existing file is re-parsed and re-serialized through [`crate::conf`], so
/// all comments survive; the header comment [`HEADER_COMMENT`] is ensured on
/// the first line. Unknown keys (manual additions outside the table) are
/// preserved untouched. The write is atomic (tmp + rename).
///
/// # Errors
///
/// [`OptionsGuiError::Invalid`] for invalid values or a corrupt existing
/// file (never overwritten), [`OptionsGuiError::Io`] for filesystem
/// failures.
pub fn write_gui_conf(
    table: &OptionsTable,
    user_dir: &Path,
    values: &[(String, String)],
) -> Result<(), OptionsGuiError> {
    // 1. Filter to non-default values and validate each one up front, so
    //    a single bad value aborts before anything is written.
    let mut written: Vec<(String, String)> = Vec::new();
    for (key, value) in values {
        let Some(option) = table.find(key) else {
            continue; // unknown keys are manual content, left untouched
        };
        if table.is_default(option, value) {
            continue;
        }
        let normalized = validate_value(table, key, value).map_err(invalid)?;
        if let Some(slot) = written.iter_mut().find(|(k, _)| k == key) {
            slot.1 = normalized; // later occurrence wins
        } else {
            written.push((key.clone(), normalized));
        }
    }

    // 2. Parse the existing fragment (missing file = empty document).
    let path = user_dir.join(GUI_CONF_FILE);
    let existing = match fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(source) if source.kind() == io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(OptionsGuiError::Io {
                action: "读取",
                path,
                source,
            });
        }
    };
    let mut doc = match existing {
        Some(text) => conf::parse(&text).map_err(|error| {
            invalid(format!(
                "解析 {} 失败:line {}: {};已中止写入(损坏文件不会被覆盖)",
                path.display(),
                error.line,
                error.message
            ))
        })?,
        None => ConfDoc {
            entries: Vec::new(),
            ends_with_newline: true,
        },
    };

    // 3. Rebuild entries: table-managed keys are replaced or removed,
    //    everything else (comments, blanks, unknown keys) is preserved.
    let mut result: Vec<Entry> = Vec::with_capacity(doc.entries.len() + written.len());
    let mut pending: Vec<(String, String)> = written;
    for entry in doc.entries {
        match entry {
            Entry::KeyValue {
                key,
                inline_comment,
                ..
            } if table.find(&key).is_some() => {
                let Some(index) = pending.iter().position(|(k, _)| *k == key) else {
                    continue; // table key reset to default: drop the line
                };
                let (_, new_value) = pending.remove(index);
                let raw_line = match inline_comment.as_ref() {
                    Some(comment) => format!("{key}={new_value} {comment}"),
                    None => format!("{key}={new_value}"),
                };
                result.push(Entry::KeyValue {
                    key,
                    value: new_value.clone(),
                    inline_comment,
                    raw_line,
                });
            }
            other => result.push(other),
        }
    }
    for (key, value) in pending {
        let raw_line = format!("{key}={value}");
        result.push(Entry::KeyValue {
            key,
            value,
            inline_comment: None,
            raw_line,
        });
    }

    // 4. Ensure the managed-file header comment sits on the first line.
    let has_header = result.iter().any(
        |entry| matches!(entry, Entry::Comment { text } if text.contains("由 mpv-config GUI 管理")),
    );
    if !has_header {
        result.insert(
            0,
            Entry::Comment {
                text: HEADER_COMMENT.to_owned(),
            },
        );
        result.insert(
            1,
            Entry::Blank {
                raw_line: String::new(),
            },
        );
    }
    doc.entries = result;
    doc.ends_with_newline = true;
    let text = conf::serialize(&doc);

    // 5. Atomic write.
    fs::create_dir_all(user_dir).map_err(|source| OptionsGuiError::Io {
        action: "创建目录",
        path: user_dir.to_path_buf(),
        source,
    })?;
    let tmp = user_dir.join(format!("{GUI_CONF_FILE}.tmp"));
    fs::write(&tmp, text).map_err(|source| OptionsGuiError::Io {
        action: "写入",
        path: tmp.clone(),
        source,
    })?;
    fs::rename(&tmp, &path).map_err(|source| {
        let _ = fs::remove_file(&tmp);
        OptionsGuiError::Io {
            action: "写入",
            path: path.clone(),
            source,
        }
    })
}

/// Read the GUI fragment `user/gui.conf` as top-level `(key, value)` pairs.
///
/// A missing file reads as an empty list; a *corrupt* one is an error (the
/// UI must show an error state instead of silently ignoring the damage).
/// Comments, blank lines and keys inside `[profile]` blocks are skipped.
///
/// # Errors
///
/// [`OptionsGuiError::Io`] on filesystem failures (other than missing-file),
/// [`OptionsGuiError::Invalid`] on parse failures.
pub fn read_gui_conf(user_dir: &Path) -> Result<Vec<(String, String)>, OptionsGuiError> {
    let path = user_dir.join(GUI_CONF_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(OptionsGuiError::Io {
                action: "读取",
                path,
                source,
            });
        }
    };
    let doc = conf::parse(&text).map_err(|error| {
        invalid(format!(
            "解析 {} 失败:line {}: {}",
            path.display(),
            error.line,
            error.message
        ))
    })?;
    let mut out = Vec::new();
    let mut in_profile = false;
    for entry in doc.entries {
        match entry {
            Entry::ProfileStart { .. } => in_profile = true,
            Entry::KeyValue { key, value, .. } if !in_profile => out.push((key, value)),
            _ => {}
        }
    }
    Ok(out)
}
