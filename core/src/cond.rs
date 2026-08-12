//! Conditional directives (`#@if` / `#@else` / `#@endif`) for `mpv.conf`.
//!
//! The conf parser keeps every `#@...` line as a plain [`Entry::Comment`] so
//! files round-trip byte-for-byte. This module interprets the subset of
//! comment lines that are actually directives — those whose trimmed text
//! starts with `#@` — and prunes the document to the branches that apply to
//! a given [`Platform`]:
//!
//! ```text
//! #@if platform==windows
//! vo=gpu-next
//! #@else
//! vo=gpu
//! #@endif
//! ```
//!
//! Supported syntax (decision D14):
//! - `#@if platform==<value>` / `#@if platform!=<value>`, where `<value>` is
//!   one of `linux`, `windows`, `macos`; whitespace around the operator is
//!   allowed;
//! - `#@else` — the else branch of the innermost open `#@if`;
//! - `#@endif` — closes the innermost open `#@if`.
//!
//! Blocks nest to arbitrary depth. A directive line may be preceded by
//! leading whitespace. Ordinary comments (`# ...`, `## ...`) are never
//! treated as directives. Unknown `#@...` forms (including `#@else` and
//! `#@endif` that carry trailing content) are errors, so typos cannot
//! silently vanish a branch.
//!
//! [`evaluate`] never panics; every malformed document yields a
//! [`CondError`] carrying the 1-based line number of the offending line.

use crate::conf::{ConfDoc, Entry};
use crate::platform::Platform;
use std::fmt;

/// A problem found while evaluating conditional directives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CondError {
    /// 1-based line number of the offending line.
    pub line: usize,
    /// Human-readable cause.
    pub message: String,
}

impl fmt::Display for CondError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for CondError {}

/// Evaluate the conditional directives in `doc` for `platform`.
///
/// Returns the entries that survive pruning: non-directive lines are passed
/// through unchanged (including comments and blank lines), directive lines
/// are removed, and each `#@if` block keeps only the branch that matches
/// `platform` (the `#@if` branch, or the `#@else` branch when the condition
/// is false and an `#@else` is present). Non-directive entries are cloned
/// verbatim, so their raw text — and therefore any later serialization —
/// is preserved exactly.
///
/// # Errors
///
/// [`CondError`] with the offending line number for: an unclosed `#@if`
/// (reported at the opening directive), an orphan `#@else` or `#@endif`,
/// a duplicate `#@else`, trailing content after `#@else`/`#@endif`, an
/// unknown `#@...` directive, or a malformed `#@if` condition (missing
/// operator, unknown variable, unknown or missing platform value).
pub fn evaluate(doc: &ConfDoc, platform: Platform) -> Result<Vec<Entry>, CondError> {
    // One frame per open `#@if`. Directives must balance structurally even
    // inside branches that are not emitted, so every `#@if` is pushed.
    let mut frames: Vec<Frame> = Vec::new();
    let mut out: Vec<Entry> = Vec::with_capacity(doc.entries.len());
    for (index, entry) in doc.entries.iter().enumerate() {
        let line_no = index + 1;
        let trimmed = entry.raw_text().trim();
        if !trimmed.starts_with("#@") {
            if emitting(&frames) {
                out.push(entry.clone());
            }
            continue;
        }
        match parse_directive(&trimmed[2..], line_no)? {
            Directive::If { negated, value } => {
                let parent_emits = emitting(&frames);
                let matches = platform_matches(platform, value) != negated;
                frames.push(Frame {
                    parent_emits,
                    taken: parent_emits && matches,
                    active: parent_emits && matches,
                    seen_else: false,
                    open_line: line_no,
                });
            }
            Directive::Else => {
                let Some(frame) = frames.last_mut() else {
                    return Err(cond_error(line_no, "`#@else` without a matching `#@if`"));
                };
                if frame.seen_else {
                    return Err(cond_error(line_no, "duplicate `#@else`"));
                }
                frame.seen_else = true;
                // The else region is emitted only when the if region was not.
                frame.active = frame.parent_emits && !frame.taken;
                if frame.active {
                    frame.taken = true;
                }
            }
            Directive::Endif => {
                if frames.pop().is_none() {
                    return Err(cond_error(line_no, "`#@endif` without a matching `#@if`"));
                }
            }
        }
    }
    if let Some(frame) = frames.last() {
        return Err(cond_error(
            frame.open_line,
            "unclosed `#@if` (missing `#@endif`)",
        ));
    }
    Ok(out)
}

/// Whether lines at the current nesting level are emitted: the innermost
/// open frame must be inside an emitted region and its current branch
/// (`#@if` or `#@else`) must be active.
fn emitting(frames: &[Frame]) -> bool {
    frames
        .last()
        .map_or(true, |frame| frame.parent_emits && frame.active)
}

/// One open `#@if` block.
struct Frame {
    /// Whether the region enclosing this block is emitted.
    parent_emits: bool,
    /// Whether a branch (`#@if` or `#@else`) is selected for this platform.
    taken: bool,
    /// Whether the current branch region is emitted (flips at `#@else`).
    active: bool,
    /// Whether `#@else` was already seen for this block.
    seen_else: bool,
    /// 1-based line of the `#@if` that opened this block.
    open_line: usize,
}

/// A parsed directive body (the trimmed line minus its leading `#@`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Directive {
    If { negated: bool, value: PlatformValue },
    Else,
    Endif,
}

/// A platform value on the right-hand side of a condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlatformValue {
    Linux,
    Windows,
    MacOS,
}

impl PlatformValue {
    fn from_token(token: &str) -> Option<Self> {
        match token {
            "linux" => Some(Self::Linux),
            "windows" => Some(Self::Windows),
            "macos" => Some(Self::MacOS),
            _ => None,
        }
    }
}

/// Whether `platform` equals the value named in the directive.
fn platform_matches(platform: Platform, value: PlatformValue) -> bool {
    matches!(
        (platform, value),
        (Platform::Linux, PlatformValue::Linux)
            | (Platform::Windows, PlatformValue::Windows)
            | (Platform::MacOS, PlatformValue::MacOS)
    )
}

/// Parse the directive body of a `#@...` line (without the `#@` prefix).
///
/// `body` is the trimmed source text after `#@`, e.g. `if platform==windows`.
fn parse_directive(body: &str, line: usize) -> Result<Directive, CondError> {
    let (keyword, rest) = match body.find(char::is_whitespace) {
        Some(at) => (&body[..at], body[at..].trim()),
        None => (body, ""),
    };
    match keyword {
        "if" => parse_condition(rest, line),
        "else" if rest.is_empty() => Ok(Directive::Else),
        "else" => Err(cond_error(line, "`#@else` must not be followed by content")),
        "endif" if rest.is_empty() => Ok(Directive::Endif),
        "endif" => Err(cond_error(
            line,
            "`#@endif` must not be followed by content",
        )),
        other => Err(cond_error(
            line,
            format!("unknown conditional directive `#@{other}`"),
        )),
    }
}

/// Parse the condition after `#@if`, e.g. `platform==windows`.
fn parse_condition(expr: &str, line: usize) -> Result<Directive, CondError> {
    if expr.is_empty() {
        return Err(cond_error(
            line,
            "missing condition after `#@if` (expected `platform==<value>` or `platform!=<value>`)",
        ));
    }
    let Some(at) = expr.find("==").or_else(|| expr.find("!=")) else {
        return Err(cond_error(
            line,
            format!(
                "invalid condition `{expr}`: expected `platform==<value>` or `platform!=<value>`"
            ),
        ));
    };
    let negated = !expr[at..].starts_with("==");
    let lhs = expr[..at].trim();
    let rhs = expr[at + 2..].trim();
    if lhs != "platform" {
        return Err(cond_error(
            line,
            format!(
                "invalid condition `{expr}`: unknown variable `{lhs}` (only `platform` is supported)"
            ),
        ));
    }
    if rhs.is_empty() {
        return Err(cond_error(
            line,
            format!(
                "invalid condition `{expr}`: missing value after `{}`",
                if negated { "!=" } else { "==" }
            ),
        ));
    }
    let Some(value) = PlatformValue::from_token(rhs) else {
        return Err(cond_error(
            line,
            format!(
                "invalid condition `{expr}`: unknown platform `{rhs}` (expected linux, windows, or macos)"
            ),
        ));
    };
    Ok(Directive::If { negated, value })
}

fn cond_error(line: usize, message: impl Into<String>) -> CondError {
    CondError {
        line,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conf::parse;

    /// Parse then evaluate; panics (in tests only) on parse failure.
    fn evaluate_text(input: &str, platform: Platform) -> Result<Vec<Entry>, CondError> {
        let doc = parse(input).unwrap_or_else(|e| panic!("parse failed for {input:?}: {e}"));
        evaluate(&doc, platform)
    }

    /// Raw source text of each output entry, in order.
    fn raw_lines(entries: &[Entry]) -> Vec<String> {
        entries
            .iter()
            .map(Entry::raw_text)
            .map(str::to_owned)
            .collect()
    }

    /// Assert that evaluating `input` for `platform` yields exactly `expected`.
    fn assert_evaluates_to(input: &str, platform: Platform, expected: &[&str]) {
        let entries = evaluate_text(input, platform)
            .unwrap_or_else(|e| panic!("evaluate failed for {input:?}: {e}"));
        assert_eq!(
            raw_lines(&entries),
            expected,
            "platform {platform:?}, input {input:?}"
        );
    }

    // ------------------------------------------------------------ equality

    #[test]
    fn matching_platform_keeps_if_branch() {
        let input = "#@if platform==windows\nvo=gpu-next\n#@else\nvo=gpu\n#@endif";
        assert_evaluates_to(input, Platform::Windows, &["vo=gpu-next"]);
    }

    #[test]
    fn non_matching_platform_takes_else_branch() {
        let input = "#@if platform==windows\nvo=gpu-next\n#@else\nvo=gpu\n#@endif";
        assert_evaluates_to(input, Platform::Linux, &["vo=gpu"]);
        assert_evaluates_to(input, Platform::MacOS, &["vo=gpu"]);
    }

    #[test]
    fn false_condition_without_else_drops_whole_block() {
        let input = "#@if platform==windows\nvo=gpu-next\n#@endif";
        assert_evaluates_to(input, Platform::Linux, &[]);
    }

    #[test]
    fn true_condition_without_else_keeps_block() {
        let input = "#@if platform==windows\nvo=gpu-next\n#@endif";
        assert_evaluates_to(input, Platform::Windows, &["vo=gpu-next"]);
    }

    // ------------------------------------------------------------- inequality

    #[test]
    fn not_equal_comparison() {
        let input = "#@if platform!=macos\nvo=gpu-next\n#@endif";
        assert_evaluates_to(input, Platform::Linux, &["vo=gpu-next"]);
        assert_evaluates_to(input, Platform::Windows, &["vo=gpu-next"]);
        assert_evaluates_to(input, Platform::MacOS, &[]);
    }

    #[test]
    fn not_equal_with_else() {
        let input = "#@if platform!=windows\n# lin\nvo=gpu\n#@else\n# win\nvo=gpu-next\n#@endif";
        assert_evaluates_to(input, Platform::Windows, &["# win", "vo=gpu-next"]);
        assert_evaluates_to(input, Platform::Linux, &["# lin", "vo=gpu"]);
    }

    // ---------------------------------------------------------------- nesting

    #[test]
    fn nested_conditionals_combine_inner_and_outer() {
        let input = "\
#@if platform==windows
windowed=yes
#@if platform==windows
a=b
#@endif
#@else
c=d
#@endif";
        assert_evaluates_to(input, Platform::Windows, &["windowed=yes", "a=b"]);
        assert_evaluates_to(input, Platform::Linux, &["c=d"]);
    }

    #[test]
    fn inner_block_dropped_when_outer_taken_but_inner_false() {
        let input = "\
#@if platform==windows
outer=1
#@if platform==linux
never=1
#@endif
outer=2
#@endif";
        assert_evaluates_to(input, Platform::Windows, &["outer=1", "outer=2"]);
    }

    #[test]
    fn directive_inside_dropped_outer_branch_still_must_balance() {
        let input = "\
#@if platform==windows
#@if platform==linux
inner=1
#@endif
#@endif
after=1";
        // The inner if is structurally inside the dropped outer branch:
        // its own else is still honoured (here: nothing).
        assert_evaluates_to(input, Platform::Linux, &["after=1"]);
    }

    // ----------------------------------------------------------- error cases

    #[test]
    fn unclosed_if_reports_opening_line() {
        let err = evaluate_text("a=1\n#@if platform==windows\nb=2", Platform::Linux)
            .expect_err("expected Err");
        assert_eq!(err.line, 2);
        assert!(err.message.contains("unclosed"), "{err}");
    }

    #[test]
    fn unclosed_nested_if_reports_innermost_opening_line() {
        let input = "\
#@if platform==windows
#@if platform==windows
a=1";
        let err = evaluate_text(input, Platform::Linux).expect_err("expected Err");
        assert_eq!(err.line, 2);
        assert!(err.message.contains("unclosed"), "{err}");
    }

    #[test]
    fn orphan_endif_reports_line() {
        for (input, line) in [("#@endif", 1), ("a=1\n#@endif\nb=2", 2)] {
            let err = evaluate_text(input, Platform::Linux).expect_err("expected Err");
            assert_eq!(err.line, line, "input {input:?}");
            assert!(err.message.contains("`#@endif`"), "{err}");
            assert!(err.message.contains("without a matching"), "{err}");
        }
    }

    #[test]
    fn orphan_else_reports_line() {
        let err = evaluate_text("a=1\n#@else", Platform::Linux).expect_err("expected Err");
        assert_eq!(err.line, 2);
        assert!(err.message.contains("`#@else`"), "{err}");
        assert!(err.message.contains("without a matching"), "{err}");
    }

    #[test]
    fn duplicate_else_reports_line() {
        let input = "\
#@if platform==windows
a=1
#@else
b=2
#@else
c=3
#@endif";
        let err = evaluate_text(input, Platform::Windows).expect_err("expected Err");
        assert_eq!(err.line, 5);
        assert!(err.message.contains("duplicate"), "{err}");
    }

    #[test]
    fn else_with_trailing_content_reports_line() {
        let input = "#@if platform==windows\n#@else something\n#@endif";
        let err = evaluate_text(input, Platform::Linux).expect_err("expected Err");
        assert_eq!(err.line, 2);
        assert!(
            err.message.contains("must not be followed by content"),
            "{err}"
        );
    }

    #[test]
    fn endif_with_trailing_content_reports_line() {
        let input = "#@if platform==windows\n#@endif extra";
        let err = evaluate_text(input, Platform::Windows).expect_err("expected Err");
        assert_eq!(err.line, 2);
        assert!(
            err.message.contains("must not be followed by content"),
            "{err}"
        );
    }

    #[test]
    fn unknown_directive_reports_line() {
        for (input, line) in [
            ("#@include foo", 1),
            ("a=1\n#@elese", 2),
            ("#@ifx platform==windows", 1),
        ] {
            let err = evaluate_text(input, Platform::Linux).expect_err("expected Err");
            assert_eq!(err.line, line, "input {input:?}");
            assert!(
                err.message.contains("unknown conditional directive"),
                "{err}"
            );
        }
    }

    #[test]
    fn invalid_condition_forms_report_line() {
        // Missing operator, unknown variable, missing value, unknown value,
        // trailing junk, empty condition.
        for (input, needle) in [
            ("#@if platform", "expected `platform==<value>`"),
            ("#@if foo==bar", "unknown variable"),
            ("#@if platform==", "missing value"),
            ("#@if platform==bsd", "unknown platform"),
            ("#@if platform==windows extra", "unknown platform"),
            ("#@if ==windows", "unknown variable"),
            ("#@if", "missing condition"),
        ] {
            let err = evaluate_text(input, Platform::Linux).expect_err("expected Err");
            assert_eq!(err.line, 1, "input {input:?}");
            assert!(err.message.contains(needle), "input {input:?}: {err}");
        }
    }

    // ------------------------------------------- ordinary comments unaffected

    #[test]
    fn plain_and_double_hash_comments_are_never_directives() {
        let input = "\
# 平台相关:
## 说明行
#@if platform==windows
vo=gpu-next
#@endif
# trailing #@lookalike";
        assert_evaluates_to(
            input,
            Platform::Windows,
            &[
                "# 平台相关:",
                "## 说明行",
                "vo=gpu-next",
                "# trailing #@lookalike",
            ],
        );
    }

    #[test]
    fn hash_at_in_middle_of_comment_is_not_a_directive() {
        // Trimmed line starts with `# `, not `#@`.
        assert_evaluates_to(
            "# not @a directive",
            Platform::Linux,
            &["# not @a directive"],
        );
    }

    // ------------------------------------------- branch content preservation

    #[test]
    fn comments_and_key_values_inside_branches_are_kept_with_branch() {
        let input = "\
#@if platform==windows
# win branch
vo=gpu-next  # <gpu-next>
#@else
# lin branch
vo=gpu  # <gpu>
#@endif";
        assert_evaluates_to(
            input,
            Platform::Windows,
            &["# win branch", "vo=gpu-next  # <gpu-next>"],
        );
        assert_evaluates_to(input, Platform::Linux, &["# lin branch", "vo=gpu  # <gpu>"]);
    }

    #[test]
    fn surrounding_entries_and_blank_lines_are_preserved() {
        let input = "\
a=1

#@if platform==windows
b=2
#@endif

c=3";
        assert_evaluates_to(input, Platform::Windows, &["a=1", "", "b=2", "", "c=3"]);
        assert_evaluates_to(input, Platform::Linux, &["a=1", "", "", "c=3"]);
    }

    #[test]
    fn directive_in_profile_block_keeps_rest_of_document() {
        let input = "\
[DeBand-high]
profile-cond=x
#@if platform==windows
deband=yes
#@endif
[Target]
brightness=100";
        assert_evaluates_to(
            input,
            Platform::Windows,
            &[
                "[DeBand-high]",
                "profile-cond=x",
                "deband=yes",
                "[Target]",
                "brightness=100",
            ],
        );
        assert_evaluates_to(
            input,
            Platform::Linux,
            &[
                "[DeBand-high]",
                "profile-cond=x",
                "[Target]",
                "brightness=100",
            ],
        );
    }

    // -------------------------------------------------------------- hygiene

    #[test]
    fn leading_whitespace_before_directive_is_allowed() {
        let input = "  #@if platform==windows\na=1\n  #@endif";
        assert_evaluates_to(input, Platform::Windows, &["a=1"]);
        assert_evaluates_to(input, Platform::Linux, &[]);
    }

    #[test]
    fn empty_if_block_evaluates_to_nothing() {
        assert_evaluates_to("#@if platform==windows\n#@endif", Platform::Windows, &[]);
        assert_evaluates_to("#@if platform==windows\n#@endif", Platform::Linux, &[]);
    }

    #[test]
    fn evaluate_of_empty_doc_is_empty_output() {
        let doc = parse("").unwrap();
        assert_eq!(
            evaluate(&doc, Platform::Linux).unwrap(),
            Vec::<Entry>::new()
        );
    }

    #[test]
    fn same_doc_yields_mutually_exclusive_branches_per_platform() {
        let input = "\
#@if platform==windows
vo=gpu-next
#@else
vo=gpu
#@endif";
        let windows = evaluate_text(input, Platform::Windows).unwrap();
        let linux = evaluate_text(input, Platform::Linux).unwrap();
        assert_eq!(raw_lines(&windows), ["vo=gpu-next"]);
        assert_eq!(raw_lines(&linux), ["vo=gpu"]);
        // Neither output contains a directive line.
        for entries in [&windows, &linux] {
            for entry in entries {
                assert!(
                    !entry.raw_text().trim().starts_with("#@"),
                    "directive leaked into output: {:?}",
                    entry.raw_text()
                );
            }
        }
    }

    #[test]
    fn whitespace_around_operator_is_accepted() {
        let input = "#@if platform  ==  windows\nvo=gpu-next\n#@endif";
        assert_evaluates_to(input, Platform::Windows, &["vo=gpu-next"]);
        assert_evaluates_to(input, Platform::Linux, &[]);
    }
}
