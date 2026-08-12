//! File-level doctor checks: layer syntax, conditional directives and
//! platform completeness.
//!
//! All checks are read-only. Missing *optional* layers are silent here
//! (they surface in [`platform_check`]); a missing `config/base.conf` or
//! current-platform layer is an error finding there, while a missing macOS
//! layer is only a warning (experimental layer).

use super::{CheckResult, Finding};
use core::cond::evaluate;
use core::conf::{parse, ConfDoc, Entry};
use core::platform::Platform;
use std::fs;
use std::io;
use std::path::Path;

/// Parse every mpv.conf-style layer (base, all platform layers, installed
/// package fragments) and report syntax errors with layer name and line.
pub(crate) fn syntax_check(root: &Path) -> CheckResult {
    let mut findings = Vec::new();
    let mut count = 0usize;
    for rel in conf_layer_files(root) {
        match read_optional(&root.join(&rel)) {
            Ok(Some(text)) => {
                count += 1;
                if let Err(error) = parse(&text) {
                    findings.push(Finding::error(format!(
                        "{rel} line {}: {}",
                        error.line, error.message
                    )));
                }
            }
            Ok(None) => {}
            Err(source) => findings.push(Finding::error(format!("读取 {rel} 失败:{source}"))),
        }
    }
    CheckResult {
        name: "语法检查",
        summary: format!("解析 {count} 个层文件(mpv.conf 语法)"),
        findings,
    }
}

/// Evaluate the `#@if`/`#@else`/`#@endif` directives of every layer and
/// report unclosed or orphan directives with layer name and line.
///
/// `input.conf` files are not mpv.conf syntax, so their lines are treated as
/// opaque text (same convention as the `gen` command) and only their
/// directives are interpreted.
pub(crate) fn cond_check(root: &Path, platform: Platform) -> CheckResult {
    let mut findings = Vec::new();
    let mut count = 0usize;
    for rel in conf_layer_files(root) {
        let Ok(Some(text)) = read_optional(&root.join(&rel)) else {
            continue;
        };
        count += 1;
        let Ok(doc) = parse(&text) else {
            continue; // syntax errors are reported by syntax_check
        };
        if let Err(error) = evaluate(&doc, platform) {
            findings.push(Finding::error(format!(
                "{rel} line {}: {}",
                error.line, error.message
            )));
        }
    }
    for rel in input_layer_files() {
        let Ok(Some(text)) = read_optional(&root.join(&rel)) else {
            continue;
        };
        count += 1;
        if let Err(error) = evaluate_opaque(&text, platform) {
            findings.push(Finding::error(format!(
                "{rel} line {}: {}",
                error.line, error.message
            )));
        }
    }
    CheckResult {
        name: "条件指令检查",
        summary: format!("求值 {count} 个层文件的条件指令"),
        findings,
    }
}

/// Platform completeness: `config/base.conf` and the current platform layer
/// must exist; a missing `config/macos.conf` is warning-level only.
pub(crate) fn platform_check(root: &Path, platform: Platform) -> CheckResult {
    let mut findings = Vec::new();
    if !root.join("config/base.conf").is_file() {
        findings.push(Finding::error("缺少必需层文件 config/base.conf".into()));
    }
    let current = format!("config/{}.conf", platform_name(platform));
    if !root.join(&current).is_file() {
        if platform == Platform::MacOS {
            findings.push(Finding::warning(format!(
                "{current} 缺失(macOS 平台层为实验性,合并时跳过)"
            )));
        } else {
            findings.push(Finding::error(format!("缺少当前平台层 {current}")));
        }
    }
    if platform != Platform::MacOS && !root.join("config/macos.conf").is_file() {
        findings.push(Finding::warning(
            "config/macos.conf 缺失(macOS 平台层为实验性,仅警告)".into(),
        ));
    }
    CheckResult {
        name: "平台完整性",
        summary: format!("base + {} 层存在性检查", platform_name(platform)),
        findings,
    }
}

/// mpv.conf-style layers, in a stable order: base, the three platform
/// layers, the optional user layer, then the installed package fragments
/// (name-sorted).
fn conf_layer_files(root: &Path) -> Vec<String> {
    let mut files = vec![
        "config/base.conf".to_owned(),
        "config/linux.conf".to_owned(),
        "config/windows.conf".to_owned(),
        "config/macos.conf".to_owned(),
        "user/user.conf".to_owned(),
    ];
    files.extend(package_files(root));
    files
}

/// input.conf layers: the base file plus the three platform variants.
fn input_layer_files() -> Vec<String> {
    vec![
        "config/input.conf".to_owned(),
        "config/linux.input.conf".to_owned(),
        "config/windows.input.conf".to_owned(),
        "config/macos.input.conf".to_owned(),
    ]
}

/// `*.conf` fragments under `config.d/packages/`, name-sorted; empty when
/// the directory does not exist.
fn package_files(root: &Path) -> Vec<String> {
    let dir = root.join("config.d").join("packages");
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".conf"))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| format!("config.d/packages/{name}"))
        .collect()
}

/// Read a file; a missing file is `None`, other I/O failures propagate.
fn read_optional(path: &Path) -> Result<Option<String>, io::Error> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(source),
    }
}

/// Evaluate the directives of `text` as an opaque input.conf document
/// (every line wrapped as a comment, like the `gen` command does).
fn evaluate_opaque(text: &str, platform: Platform) -> Result<(), core::cond::CondError> {
    let entries: Vec<Entry> = text
        .lines()
        .map(|line| Entry::Comment {
            text: line.to_owned(),
        })
        .collect();
    let doc = ConfDoc {
        entries,
        ends_with_newline: text.ends_with('\n'),
    };
    evaluate(&doc, platform).map(|_| ())
}

/// Lowercase file-name stem of a platform (`linux` / `windows` / `macos`).
fn platform_name(platform: Platform) -> &'static str {
    crate::gen::layers::platform_name(platform)
}
