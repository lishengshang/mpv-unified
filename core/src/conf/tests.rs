//! Happy-path unit tests for the conf parser and serializer.
//!
//! Failure-path tests live in [`super::errors`]; shared helpers live in
//! [`super::testutil`].

use super::testutil::*;
use super::*;

// ---------------------------------------------------------------- comments

#[test]
fn comment_full_line() {
    assert_eq!(
        single_entry("# hello world"),
        Entry::Comment {
            text: "# hello world".into()
        }
    );
}

#[test]
fn comment_double_hash_with_chinese() {
    let text = "##⇘⇘基本说明：";
    assert_eq!(single_entry(text), Entry::Comment { text: text.into() });
}

#[test]
fn comment_keeps_leading_whitespace() {
    let text = "                                      ## gpu 是旧版渲染后端";
    assert_eq!(single_entry(text), Entry::Comment { text: text.into() });
}

#[test]
fn conditional_directive_is_kept_as_comment_for_now() {
    // Reserved for the T4 conditional-directive task: today these lines are
    // plain comments and must round-trip untouched.
    for text in ["#@if platform==windows", "#@else", "#@endif"] {
        assert_eq!(single_entry(text), Entry::Comment { text: text.into() });
    }
}

// -------------------------------------------------------------- key values

#[test]
fn key_value_simple() {
    assert_kv(&single_entry("vo=gpu-next"), "vo", "gpu-next", None);
}

#[test]
fn key_value_spaces_around_equals() {
    assert_kv(
        &single_entry("use-filedir-conf = yes"),
        "use-filedir-conf",
        "yes",
        None,
    );
}

#[test]
fn key_value_indented_inside_profile() {
    assert_kv(&single_entry(" icc-intent=0"), "icc-intent", "0", None);
}

#[test]
fn flag_only_line_has_empty_value() {
    assert_kv(&single_entry("no-osd-bar"), "no-osd-bar", "", None);
}

#[test]
fn flag_line_with_inline_comment() {
    assert_kv(
        &single_entry("write-filename-in-watch-later-config  # 将文件名写入播放记录缓存文件"),
        "write-filename-in-watch-later-config",
        "",
        Some("# 将文件名写入播放记录缓存文件"),
    );
}

#[test]
fn key_value_with_inline_comment() {
    assert_kv(
        &single_entry("vo=gpu-next                           # <gpu/gpu-next/libmpv> 视频输出驱动"),
        "vo",
        "gpu-next",
        Some("# <gpu/gpu-next/libmpv> 视频输出驱动"),
    );
}

#[test]
fn hash_without_preceding_whitespace_stays_in_value() {
    assert_kv(&single_entry("key=ab#c d"), "key", "ab#c d", None);
    assert_kv(&single_entry("key=#fff"), "key", "#fff", None);
}

#[test]
fn hash_inside_double_quotes_is_not_a_comment() {
    assert_kv(
        &single_entry("osd-color=\"#ef14d5\""),
        "osd-color",
        "\"#ef14d5\"",
        None,
    );
}

#[test]
fn hash_inside_quotes_with_real_trailing_comment() {
    assert_kv(
        &single_entry("key=\"a # b\" # real comment"),
        "key",
        "\"a # b\"",
        Some("# real comment"),
    );
}

#[test]
fn single_quoted_value_with_apostrophe_pairs() {
    let line = " profile-cond=path:find('://') ~= nil or path:find('^magnet:') ~= nil";
    assert_kv(
        &single_entry(line),
        "profile-cond",
        "path:find('://') ~= nil or path:find('^magnet:') ~= nil",
        None,
    );
}

#[test]
fn value_splits_on_first_equals_only() {
    assert_kv(
        &single_entry("title=${?pause==yes:⏸}${?mute==yes:🔇}"),
        "title",
        "${?pause==yes:⏸}${?mute==yes:🔇}",
        None,
    );
    assert_kv(
        &single_entry("ytdl-raw-options-append=cookies-from-browser=Firefox"),
        "ytdl-raw-options-append",
        "cookies-from-browser=Firefox",
        None,
    );
}

#[test]
fn empty_quoted_value_is_allowed() {
    assert_kv(
        &single_entry("icc-profile=\"\""),
        "icc-profile",
        "\"\"",
        None,
    );
}

// ---------------------------------------------------------------- profiles

#[test]
fn profile_start_simple() {
    match single_entry("[DeBand-high]") {
        Entry::ProfileStart { name, raw_line } => {
            assert_eq!(name, "DeBand-high");
            assert_eq!(raw_line, "[DeBand-high]");
        }
        other => panic!("expected ProfileStart, got {other:?}"),
    }
}

#[test]
fn profile_start_keeps_surrounding_whitespace() {
    match single_entry("  [Target]  ") {
        Entry::ProfileStart { name, raw_line } => {
            assert_eq!(name, "Target");
            assert_eq!(raw_line, "  [Target]  ");
        }
        other => panic!("expected ProfileStart, got {other:?}"),
    }
}

#[test]
fn commented_out_profile_is_a_comment() {
    assert_eq!(
        single_entry("#[image]"),
        Entry::Comment {
            text: "#[image]".into()
        }
    );
}

// ------------------------------------------------------------------ blanks

#[test]
fn blank_line_keeps_whitespace() {
    assert_eq!(
        single_entry("   "),
        Entry::Blank {
            raw_line: "   ".into()
        }
    );
}

#[test]
fn empty_input_is_an_empty_doc() {
    let doc = parse_ok("");
    assert!(doc.entries.is_empty());
    assert!(!doc.ends_with_newline);
    assert_eq!(serialize(&doc), "");
}

// ------------------------------------------- structure and trailing newline

#[test]
fn trailing_newline_state_is_recorded() {
    assert!(parse_ok("a=b\n").ends_with_newline);
    assert!(!parse_ok("a=b").ends_with_newline);
    assert!(parse_ok("\n").ends_with_newline);
}

#[test]
fn multi_line_doc_keeps_order_and_kinds() {
    let input = "##⇘⇘说明\n\nvo=gpu-next  # 驱动\n\n[HDR]\n profile-cond=x\n #@if platform==windows\nno-osd-bar";
    let doc = parse_ok(input);
    let kinds: Vec<&str> = doc
        .entries
        .iter()
        .map(|e| match e {
            Entry::Comment { .. } => "comment",
            Entry::KeyValue { .. } => "kv",
            Entry::ProfileStart { .. } => "profile",
            Entry::Blank { .. } => "blank",
        })
        .collect();
    assert_eq!(
        kinds,
        ["comment", "blank", "kv", "blank", "profile", "kv", "comment", "kv"]
    );
    assert!(!doc.ends_with_newline);
    assert_eq!(serialize(&doc), input);
}

#[test]
fn comment_insertion_does_not_swallow_neighbors() {
    let input = "a=1\n# inserted half-line comment\nb=2";
    let doc = parse_ok(input);
    assert_eq!(doc.entries.len(), 3);
    assert_kv(&doc.entries[0], "a", "1", None);
    assert_eq!(
        doc.entries[1],
        Entry::Comment {
            text: "# inserted half-line comment".into()
        }
    );
    assert_kv(&doc.entries[2], "b", "2", None);
    assert_eq!(serialize(&doc), input);
}

#[test]
fn crlf_input_roundtrips_and_strips_cr_from_fields() {
    let input = "key=value\r\n# comment\r\n\r\n";
    let doc = parse_ok(input);
    assert!(doc.ends_with_newline);
    assert_kv(&doc.entries[0], "key", "value", None);
    assert_eq!(serialize(&doc), input);
}

#[test]
fn serialize_joins_with_newlines_only() {
    let doc = ConfDoc {
        entries: vec![
            Entry::Comment { text: "# a".into() },
            Entry::Blank {
                raw_line: String::new(),
            },
            Entry::KeyValue {
                key: "k".into(),
                value: "v".into(),
                inline_comment: None,
                raw_line: "k=v".into(),
            },
        ],
        ends_with_newline: true,
    };
    assert_eq!(serialize(&doc), "# a\n\nk=v\n");
}
