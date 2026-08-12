//! Secret audit: scan `user/` and `config/` for lines that look like API
//! keys or tokens, skipping `*.example` templates and hidden files.
//!
//! Warning-level only: a hit means "double-check this file before pushing
//! the repository", not an automatic failure.

use super::{CheckResult, Finding};
use std::fs;
use std::path::Path;

/// Scan `user/` and `config/` (recursively) for suspected secret lines.
pub(crate) fn check(root: &Path) -> CheckResult {
    let mut findings = Vec::new();
    let mut scanned = 0usize;
    for dir in ["user", "config"] {
        scan_dir(&root.join(dir), &mut findings, &mut scanned);
    }
    CheckResult {
        name: "密钥审计",
        summary: format!("扫描 {scanned} 个文件(user/ 与 config/,跳过 *.example)"),
        findings,
    }
}

/// Walk `dir` recursively; hidden subdirectories are skipped.
fn scan_dir(dir: &Path, findings: &mut Vec<Finding>, scanned: &mut usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let hidden = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with('.'));
            if !hidden {
                scan_dir(&path, findings, scanned);
            }
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.contains(".example") || name.starts_with('.') {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        *scanned += 1;
        for (index, line) in text.lines().enumerate() {
            if let Some(pattern) = looks_secret(line) {
                findings.push(Finding::warning(format!(
                    "{}:{} 疑似密钥(匹配模式 {pattern})",
                    path.display(),
                    index + 1
                )));
            }
        }
    }
}

/// The matched pattern when `line` looks like a secret, else `None`.
fn looks_secret(line: &str) -> Option<&'static str> {
    let trimmed = line.trim();
    if trimmed.is_empty() || is_placeholder(trimmed) {
        return None;
    }
    let lower = trimmed.to_lowercase();
    if lower.contains("sk-") {
        return Some("sk-");
    }
    for pattern in ["token", "secret", "apikey", "api_key", "api-key"] {
        if key_position_contains(&lower, pattern) {
            return Some(pattern);
        }
    }
    key_assignment(trimmed).then_some("key=")
}

/// Template placeholder lines (`<...>`), e.g. `token=<你的 assrt token>` or
/// `YOUR_WHISPER_TOKEN`; these are documentation, not real secrets.
fn is_placeholder(line: &str) -> bool {
    line.contains('<') && line.contains('>')
}

/// Whether `needle` appears in key position: followed by optional
/// whitespace and then `=`. This ignores prose like "API key / token。"
/// and names like `YOUR_WHISPER_TOKEN`.
fn key_position_contains(lower: &str, needle: &str) -> bool {
    let mut start = 0;
    while let Some(index) = lower[start..].find(needle) {
        let after = &lower[start + index + needle.len()..];
        if after.trim_start().starts_with('=') {
            return true;
        }
        start += index + needle.len();
    }
    false
}

/// A `key` / `key = <value>` assignment whose value is at least 4
/// characters — the classic "secret in a config" shape.
fn key_assignment(line: &str) -> bool {
    let lower = line.to_lowercase();
    let Some(rest) = lower.strip_prefix("key") else {
        return false;
    };
    let Some(value) = rest.trim_start().strip_prefix('=') else {
        return false;
    };
    value.trim().chars().count() >= 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sk_prefix_keys() {
        assert_eq!(looks_secret("sk-abcdef1234567890"), Some("sk-"));
        assert_eq!(
            looks_secret("openai_key = sk-proj-xYz1234567890"),
            Some("sk-")
        );
    }

    #[test]
    fn detects_token_and_secret_words() {
        assert_eq!(looks_secret("token = abc123"), Some("token"));
        assert_eq!(looks_secret("assrt_token=abc123"), Some("token"));
        assert_eq!(looks_secret("client_secret=abc123"), Some("secret"));
        assert_eq!(looks_secret("api_key=abc123"), Some("api_key"));
        assert_eq!(looks_secret("apikey=abc123"), Some("apikey"));
        assert_eq!(looks_secret("api-key=abc123"), Some("api-key"));
    }

    #[test]
    fn ignores_prose_mentions_and_placeholders() {
        assert_eq!(looks_secret("# 写真实 API key / token。"), None);
        assert_eq!(looks_secret("## 需要 assrt token:"), None);
        assert_eq!(looks_secret("## 占位:YOUR_WHISPER_TOKEN"), None);
        assert_eq!(
            looks_secret("## token=<你的 assrt token,占位:YOUR_ASSRT_TOKEN_HERE>"),
            None
        );
    }

    #[test]
    fn detects_key_assignments() {
        assert_eq!(looks_secret("key = abc123"), Some("key="));
        assert_eq!(looks_secret("key=abcd"), Some("key="));
        assert_eq!(looks_secret("KEY = abcd"), Some("key="));
        assert_eq!(looks_secret("key = ab"), None, "value too short");
    }

    #[test]
    fn ignores_clean_config_lines() {
        assert_eq!(looks_secret("vo=gpu"), None);
        assert_eq!(looks_secret("# a comment"), None);
        assert_eq!(looks_secret("keybind=foo"), None);
        assert_eq!(looks_secret("keyboard=us"), None);
        assert_eq!(looks_secret(""), None);
        assert_eq!(looks_secret("   "), None);
    }
}
