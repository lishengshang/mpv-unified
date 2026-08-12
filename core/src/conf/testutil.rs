//! Shared helpers for the conf test modules.

use super::{parse, ConfDoc, Entry, ParseError};

pub(super) fn parse_ok(input: &str) -> ConfDoc {
    parse(input).unwrap_or_else(|e| panic!("expected Ok for {input:?}, got Err: {e}"))
}

pub(super) fn parse_err(input: &str) -> ParseError {
    match parse(input) {
        Ok(doc) => panic!("expected Err for {input:?}, got Ok: {doc:?}"),
        Err(e) => e,
    }
}

pub(super) fn single_entry(input: &str) -> Entry {
    let doc = parse_ok(input);
    assert_eq!(
        doc.entries.len(),
        1,
        "expected exactly one entry for {input:?}"
    );
    doc.entries.into_iter().next().unwrap()
}

pub(super) fn assert_kv(entry: &Entry, key: &str, value: &str, inline_comment: Option<&str>) {
    match entry {
        Entry::KeyValue {
            key: k,
            value: v,
            inline_comment: c,
            ..
        } => {
            assert_eq!(k, key, "key mismatch");
            assert_eq!(v, value, "value mismatch");
            assert_eq!(c.as_deref(), inline_comment, "inline_comment mismatch");
        }
        other => panic!("expected KeyValue, got {other:?}"),
    }
}
