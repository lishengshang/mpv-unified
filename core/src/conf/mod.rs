//! Lossless parser and serializer for `mpv.conf` files.
//!
//! The parser is line-oriented and keeps the exact source text of every line
//! inside its [`Entry`]. [`serialize`] therefore reproduces the original file
//! byte for byte (including trailing whitespace and whether the final line
//! ends with a newline), while the parsed fields (`key`, `value`, profile
//! name, ...) are available for higher layers such as the merge engine.
//!
//! Conditional directive lines (`#@if ...`) are deliberately preserved as
//! plain [`Entry::Comment`] values for now; a later task upgrades them to
//! structured entries without breaking round-trip fidelity.

mod model;
mod parser;
mod serializer;

#[cfg(test)]
mod errors;
#[cfg(test)]
mod roundtrip;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod testutil;

pub use model::{ConfDoc, Entry};
pub use parser::{parse, ParseError, MAX_LINE_BYTES};
pub use serializer::serialize;
