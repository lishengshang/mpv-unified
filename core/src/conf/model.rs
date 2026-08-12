//! Data model for a parsed `mpv.conf` document.

/// A parsed `mpv.conf` document: one [`Entry`] per physical line, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfDoc {
    /// Entries in file order; `entries.len()` equals the number of lines.
    pub entries: Vec<Entry>,
    /// Whether the source text ends with a final `\n`. [`super::serialize`]
    /// reproduces this exactly, so files without a trailing newline round-trip
    /// without gaining one.
    pub ends_with_newline: bool,
}

/// One physical line of a `mpv.conf` file.
///
/// Every variant keeps the exact source text of its line (without the `\n`
/// terminator), which is what [`super::serialize`] emits. The remaining
/// fields are parsed views used by higher layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// A whole-line comment: the line's first non-whitespace character is `#`.
    /// `text` is the raw line including leading whitespace and the `#` prefix.
    /// Conditional directives (`#@if ...`) are kept as comments for now.
    Comment { text: String },

    /// A `key=value` line, or a flag-only line such as `no-osd-bar`
    /// (no `=`, implicit `yes`), possibly followed by an inline comment.
    ///
    /// - `key` and `value` are trimmed views; surrounding whitespace and
    ///   quoting are preserved in `raw_line`. Quotes stay part of `value`.
    /// - Flag-only lines have an empty `value`.
    /// - `inline_comment` starts at the `#` (inclusive) when present.
    KeyValue {
        key: String,
        value: String,
        inline_comment: Option<String>,
        raw_line: String,
    },

    /// A `[name]` profile header line. `raw_line` preserves any surrounding
    /// whitespace; `name` is the text between the brackets.
    ProfileStart { name: String, raw_line: String },

    /// An empty or whitespace-only line. `raw_line` keeps the exact
    /// whitespace bytes.
    Blank { raw_line: String },
}

impl Entry {
    /// The exact source text of this line (without the `\n` terminator).
    #[must_use]
    pub fn raw_text(&self) -> &str {
        match self {
            Entry::Comment { text } => text,
            Entry::KeyValue { raw_line, .. }
            | Entry::ProfileStart { raw_line, .. }
            | Entry::Blank { raw_line } => raw_line,
        }
    }
}
