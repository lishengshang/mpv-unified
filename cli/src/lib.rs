//! `cli` crate: the `mpv-config` command-line interface.
//!
//! The binary entry point lives in `main.rs`; the reusable `gen` engine is
//! exposed here so integration tests under `tests/` can drive it.

pub mod gen;
