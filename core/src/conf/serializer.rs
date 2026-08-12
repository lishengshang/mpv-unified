//! Serializer that reconstructs the exact source text of a parsed document.

use super::model::ConfDoc;

/// Serialize `doc` back into `mpv.conf` text.
///
/// For every document produced by [`super::parse`], the output is
/// byte-identical to the original input: each line is emitted verbatim,
/// lines are joined with `\n`, and the trailing newline (or its absence) is
/// reproduced exactly.
#[must_use]
pub fn serialize(doc: &ConfDoc) -> String {
    let mut out = String::new();
    for (index, entry) in doc.entries.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(entry.raw_text());
    }
    if doc.ends_with_newline && !doc.entries.is_empty() {
        out.push('\n');
    }
    out
}
