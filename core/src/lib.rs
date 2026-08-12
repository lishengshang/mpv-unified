//! Core library for the mpv-config generator.
//!
//! Currently provides [`conf`], a lossless `mpv.conf` parser: any file that
//! parses successfully round-trips back to the exact same bytes.

pub mod conf;
