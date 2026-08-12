//! Line-oriented `mpv.conf` parser.

use super::model::{ConfDoc, Entry};
use std::fmt;

/// Maximum length of a single line in bytes. Longer lines are rejected.
/// Real configs stay well below this (the shipped ones max out around 1KB);
/// the limit only guards against pathological input.
pub const MAX_LINE_BYTES: usize = 10_000;

/// A parse failure on a specific line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// 1-based line number where the problem was found.
    pub line: usize,
    /// Human-readable cause.
    pub message: String,
}

impl ParseError {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse a `mpv.conf` document.
///
/// Byte-level round-trip guarantee: for every input that parses successfully,
/// [`super::serialize`] returns exactly `input` (including whether the final
/// line ends with a newline).
///
/// # Errors
///
/// Returns a [`ParseError`] carrying a 1-based line number for a UTF-8 BOM,
/// a line longer than [`MAX_LINE_BYTES`], an empty key or value, an unclosed
/// quote, a malformed `[profile]` header, or a stray `]` line.
pub fn parse(input: &str) -> Result<ConfDoc, ParseError> {
    if input.starts_with('\u{feff}') {
        return Err(ParseError::new(
            1,
            "file starts with a UTF-8 BOM; save it as plain UTF-8",
        ));
    }
    if input.is_empty() {
        return Ok(ConfDoc {
            entries: Vec::new(),
            ends_with_newline: false,
        });
    }
    let ends_with_newline = input.ends_with('\n');
    let body = input.strip_suffix('\n').unwrap_or(input);
    let mut entries = Vec::new();
    for (index, line) in body.split('\n').enumerate() {
        let line_no = index + 1;
        if line.len() > MAX_LINE_BYTES {
            return Err(ParseError::new(
                line_no,
                format!(
                    "line is too long ({} bytes; limit is {MAX_LINE_BYTES})",
                    line.len()
                ),
            ));
        }
        entries.push(parse_line(line, line_no)?);
    }
    Ok(ConfDoc {
        entries,
        ends_with_newline,
    })
}

fn parse_line(line: &str, line_no: usize) -> Result<Entry, ParseError> {
    let without_cr = line.strip_suffix('\r').unwrap_or(line);
    let trimmed = without_cr.trim();
    if trimmed.is_empty() {
        return Ok(Entry::Blank {
            raw_line: line.to_owned(),
        });
    }
    if trimmed.starts_with('#') {
        return Ok(Entry::Comment {
            text: line.to_owned(),
        });
    }
    if trimmed.starts_with('[') {
        return parse_profile_line(trimmed, line, line_no);
    }
    if trimmed.starts_with(']') {
        return Err(ParseError::new(line_no, "stray ']' without a matching '['"));
    }
    parse_key_value_line(without_cr, line, line_no)
}

fn parse_profile_line(trimmed: &str, line: &str, line_no: usize) -> Result<Entry, ParseError> {
    let close = trimmed
        .find(']')
        .ok_or_else(|| ParseError::new(line_no, "profile header is missing its closing ']'"))?;
    let name = &trimmed[1..close];
    if name.is_empty() {
        return Err(ParseError::new(
            line_no,
            "profile header has an empty profile name",
        ));
    }
    if !trimmed[close + 1..].trim().is_empty() {
        return Err(ParseError::new(
            line_no,
            "unexpected content after ']' in profile header",
        ));
    }
    Ok(Entry::ProfileStart {
        name: name.to_owned(),
        raw_line: line.to_owned(),
    })
}

fn parse_key_value_line(without_cr: &str, line: &str, line_no: usize) -> Result<Entry, ParseError> {
    let comment_start = find_inline_comment_start(without_cr, line_no)?;
    let content_end = comment_start.unwrap_or(without_cr.len());
    let content = &without_cr[..content_end];
    let inline_comment = comment_start.map(|start| without_cr[start..].trim_end().to_owned());
    match content.find('=') {
        Some(equals) => {
            let key = content[..equals].trim();
            if key.is_empty() {
                return Err(ParseError::new(line_no, "missing option name before '='"));
            }
            let value = content[equals + 1..].trim();
            if value.is_empty() {
                return Err(ParseError::new(
                    line_no,
                    format!("option '{key}' has an empty value"),
                ));
            }
            Ok(Entry::KeyValue {
                key: key.to_owned(),
                value: value.to_owned(),
                inline_comment,
                raw_line: line.to_owned(),
            })
        }
        None => Ok(Entry::KeyValue {
            // Flag-only line such as `no-osd-bar` (implicit `yes`).
            key: content.trim().to_owned(),
            value: String::new(),
            inline_comment,
            raw_line: line.to_owned(),
        }),
    }
}

/// Byte offset of the `#` that starts an inline comment, if any.
///
/// An inline comment starts at a `#` that lies outside any quoted region and
/// is preceded by whitespace. The scan also validates that quotes are
/// balanced in the non-comment part of the line.
fn find_inline_comment_start(line: &str, line_no: usize) -> Result<Option<usize>, ParseError> {
    let mut open_quote: Option<char> = None;
    let mut prev: Option<char> = None;
    for (index, ch) in line.char_indices() {
        match open_quote {
            Some(quote) => {
                if ch == quote {
                    open_quote = None;
                }
            }
            None if ch == '"' || ch == '\'' => open_quote = Some(ch),
            None if ch == '#' && prev.is_some_and(char::is_whitespace) => {
                return Ok(Some(index));
            }
            None => {}
        }
        prev = Some(ch);
    }
    match open_quote {
        Some(quote) => Err(ParseError::new(
            line_no,
            format!("unclosed {quote} quote in value"),
        )),
        None => Ok(None),
    }
}
