//! Option-validity check: compare the option names of the merged output
//! against `mpv --list-options`.
//!
//! Warning-level only (never blocking): unknown names are reported with a
//! hint that script-defined custom options may be legitimate. When mpv is
//! unavailable the whole check is skipped with an explanatory warning.

use super::{CheckResult, Finding};
use crate::gen::layers::{load_layer, load_packages, platform_name};
use core::conf::{ConfDoc, Entry};
use core::merge::{merge_docs, MergeLayer};
use core::platform::Platform;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// Profile-block pseudo-options accepted by mpv but absent from
/// `--list-options`; allowed anywhere to avoid noise.
const PROFILE_PSEUDO_OPTIONS: [&str; 4] = [
    "profile-cond",
    "profile-desc",
    "profile-restore",
    "profile-bind",
];

/// `--{` / `--}` list-group markers, which are not real option names.
const GROUP_MARKERS: [&str; 2] = ["{", "}"];

/// Option names parsed from `mpv --list-options`: plain names plus the
/// Flag options (which additionally accept a `no-` prefix).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MpvOptions {
    names: HashSet<String>,
    flags: HashSet<String>,
}

/// Parse `mpv --list-options` output.
///
/// Each option line is `--<name> <padding> <type> (...)`; wrapped
/// continuation lines (e.g. `--audio-exts-append`) and the `--{`/`--}` list
/// markers are also listed, so any line whose trimmed text starts with `--`
/// and carries a token is accepted.
pub(crate) fn parse_option_names(text: &str) -> MpvOptions {
    let mut opts = MpvOptions::default();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let Some(token) = trimmed
            .strip_prefix("--")
            .and_then(|rest| rest.split_whitespace().next())
        else {
            continue;
        };
        if token.is_empty() {
            continue;
        }
        let type_info = trimmed[2 + token.len()..].trim_start();
        let kind = type_info.split('(').next().unwrap_or("").trim();
        opts.names.insert(token.to_owned());
        if kind == "Flag" {
            opts.flags.insert(token.to_owned());
        }
    }
    opts
}

/// Whether `key` is a known mpv option (or a `no-`-prefixed flag, or a
/// profile pseudo-option, or a list-group marker).
pub(crate) fn is_known(key: &str, opts: &MpvOptions) -> bool {
    if GROUP_MARKERS.contains(&key) {
        return true;
    }
    if opts.names.contains(key) {
        return true;
    }
    if let Some(rest) = key.strip_prefix("no-") {
        return opts.flags.contains(rest);
    }
    PROFILE_PSEUDO_OPTIONS.contains(&key)
}

/// Run `mpv --list-options`; `None` when mpv is missing or the call fails.
pub fn mpv_list_options(mpv: &str) -> Option<String> {
    let output = Command::new(mpv).arg("--list-options").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

/// Run the option-validity check; the injectable `options_source` makes the
/// check deterministic in tests without a real mpv binary.
pub(crate) fn check<F: Fn() -> Option<String>>(
    root: &Path,
    platform: Platform,
    options_source: &F,
) -> CheckResult {
    let Some(list) = options_source() else {
        return CheckResult {
            name: "选项合法性",
            summary: "已跳过".to_owned(),
            findings: vec![Finding::warning(
                "跳过选项校验:mpv 不可用(mpv --list-options 失败)".into(),
            )],
        };
    };
    let opts = parse_option_names(&list);
    let merged = match merged_doc(root, platform) {
        Ok(doc) => doc,
        Err(message) => {
            return CheckResult {
                name: "选项合法性",
                summary: "已跳过".to_owned(),
                findings: vec![Finding::warning(format!("跳过选项校验:{message}"))],
            };
        }
    };
    let mut checked = 0usize;
    let mut unknown: Vec<String> = Vec::new();
    for entry in &merged.entries {
        if let Entry::KeyValue { key, .. } = entry {
            checked += 1;
            if !is_known(key, &opts) {
                unknown.push(key.clone());
            }
        }
    }
    let findings: Vec<Finding> = unknown
        .into_iter()
        .map(|key| {
            Finding::warning(format!(
                "未知选项 {key}(mpv --list-options 未收录;若为脚本自定义选项可忽略)"
            ))
        })
        .collect();
    CheckResult {
        name: "选项合法性",
        summary: format!(
            "对照 mpv --list-options 校验 {checked} 个选项名({} 个未知)",
            findings.len()
        ),
        findings,
    }
}

/// Merge the four layers for `platform` (mirroring `gen`); on failure the
/// error message explains why the check was skipped.
fn merged_doc(root: &Path, platform: Platform) -> Result<ConfDoc, String> {
    let platform_file = format!("{}.conf", platform_name(platform));
    let base = load_layer(root, "config/base.conf", platform, false)
        .map_err(|error| error.to_string())?
        .unwrap_or_else(|| empty_layer("config/base.conf"));
    let platform_layer = load_layer(root, &format!("config/{platform_file}"), platform, false)
        .map_err(|error| error.to_string())?
        .unwrap_or_else(|| empty_layer(&platform_file));
    let packages = load_packages(root, platform).map_err(|error| error.to_string())?;
    let user =
        load_layer(root, "user/user.conf", platform, false).map_err(|error| error.to_string())?;
    merge_docs(&base, &platform_layer, &packages, user.as_ref()).map_err(|error| error.to_string())
}

/// An empty layer standing in for an optional missing file.
fn empty_layer(name: &str) -> MergeLayer {
    MergeLayer::new(
        name,
        ConfDoc {
            entries: Vec::new(),
            ends_with_newline: true,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_LIST: &str = "\
Options:

 --osd-bar                        Flag (default: yes)
 --vo                             String (default: gpu)
 --ab-loop-count                  Choices: inf (or an integer) (0 to 2147483647) (default: inf)
    --ytdl-raw-options-append
 --{
 --}
";

    #[test]
    fn parse_extracts_names_and_flags() {
        let opts = parse_option_names(SAMPLE_LIST);

        assert!(opts.names.contains("osd-bar"));
        assert!(opts.names.contains("vo"));
        assert!(opts.names.contains("ab-loop-count"));
        assert!(opts.names.contains("ytdl-raw-options-append"));
        assert!(opts.flags.contains("osd-bar"));
        assert!(!opts.flags.contains("ab-loop-count"));
    }

    #[test]
    fn parse_ignores_headers_and_blank_lines() {
        let opts = parse_option_names("Options:\n\n --only-real\n");

        assert_eq!(opts.names.len(), 1);
        assert!(opts.names.contains("only-real"));
    }

    #[test]
    fn known_names_and_no_prefix_flags_are_known() {
        let opts = parse_option_names(SAMPLE_LIST);

        assert!(is_known("osd-bar", &opts));
        assert!(is_known("no-osd-bar", &opts), "no- prefix on a Flag");
        assert!(!is_known("no-vo", &opts), "no- prefix on a non-Flag");
        assert!(!is_known("definitely-not-an-option", &opts));
    }

    #[test]
    fn profile_pseudo_options_and_group_markers_are_known() {
        let opts = parse_option_names(SAMPLE_LIST);

        assert!(is_known("profile-restore", &opts));
        assert!(is_known("profile-cond", &opts));
        assert!(is_known("{", &opts));
        assert!(is_known("}", &opts));
    }
}
