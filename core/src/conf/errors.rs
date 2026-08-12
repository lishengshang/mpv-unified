//! Malformed-input tests: every failure carries a 1-based line number.

use super::testutil::*;
use super::*;

#[test]
fn malformed_empty_value() {
    for input in ["key=", "volume=   ", "key= \t "] {
        let e = parse_err(input);
        assert_eq!(e.line, 1, "input {input:?}");
        assert!(e.message.contains("empty value"), "input {input:?}: {e}");
    }
}

#[test]
fn malformed_empty_value_before_inline_comment() {
    let e = parse_err("key= # comment");
    assert_eq!(e.line, 1);
    assert!(e.message.contains("empty value"), "{e}");
}

#[test]
fn malformed_unclosed_quotes() {
    for input in ["osd-color=\"#ef14d5", "key='abc", "key=\"a 'b' c"] {
        let e = parse_err(input);
        assert_eq!(e.line, 1, "input {input:?}");
        assert!(e.message.contains("quote"), "input {input:?}: {e}");
    }
}

#[test]
fn unclosed_quote_inside_comment_is_allowed() {
    let doc = parse_ok("key=value # note with \"unclosed quote");
    assert_kv(
        &doc.entries[0],
        "key",
        "value",
        Some("# note with \"unclosed quote"),
    );
}

#[test]
fn malformed_lone_closing_bracket() {
    let e = parse_err("]foo");
    assert_eq!(e.line, 1);
    assert!(e.message.contains(']'), "{e}");
}

#[test]
fn malformed_unclosed_profile_header() {
    let e = parse_err("[name");
    assert_eq!(e.line, 1);
    assert!(e.message.contains(']'), "{e}");
}

#[test]
fn malformed_empty_profile_name() {
    let e = parse_err("[]");
    assert_eq!(e.line, 1);
    assert!(e.message.contains("profile name"), "{e}");
}

#[test]
fn malformed_junk_after_profile_header() {
    let e = parse_err("[a] b");
    assert_eq!(e.line, 1);
    assert!(e.message.contains("after"), "{e}");
}

#[test]
fn malformed_line_too_long() {
    let long = "x".repeat(MAX_LINE_BYTES + 1);
    let e = parse_err(&long);
    assert_eq!(e.line, 1);
    assert!(e.message.contains("too long"), "{e}");
    // Exactly at the limit a plain key line is still accepted.
    let at_limit = "x".repeat(MAX_LINE_BYTES);
    assert_kv(&single_entry(&at_limit), &at_limit, "", None);
}

#[test]
fn malformed_bom() {
    let e = parse_err("\u{feff}key=value");
    assert_eq!(e.line, 1);
    assert!(e.message.contains("BOM"), "{e}");
}

#[test]
fn malformed_empty_key() {
    let e = parse_err("=value");
    assert_eq!(e.line, 1);
    assert!(e.message.contains("option name"), "{e}");
}

#[test]
fn errors_report_the_right_line_number() {
    let e = parse_err("# ok\nvo=gpu-next\nkey=");
    assert_eq!(e.line, 3);
    let e = parse_err("a=1\n\n]stray");
    assert_eq!(e.line, 3);
    let e = parse_err("\u{feff}# bom on line one\nkey=");
    assert_eq!(e.line, 1);
}

#[test]
fn commented_out_malformed_lines_are_allowed() {
    for input in [
        "#key=",
        "#[broken",
        "#ytdl-raw-options-append=write-subs=",
        "#  \"unclosed quote in a comment",
    ] {
        assert_eq!(
            single_entry(input),
            Entry::Comment { text: input.into() },
            "input {input:?}"
        );
    }
}
