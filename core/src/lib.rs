//! Core library for the mpv-config generator.
//!
//! Provides [`conf`], a lossless `mpv.conf` parser: any file that parses
//! successfully round-trips back to the exact same bytes; and [`platform`],
//! host platform detection plus mpv config-directory resolution.

pub mod conf;
pub mod platform;
