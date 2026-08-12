//! Core library for the mpv-config generator.
//!
//! Provides [`conf`], a lossless `mpv.conf` parser: any file that parses
//! successfully round-trips back to the exact same bytes; [`platform`], host
//! platform detection plus mpv config-directory resolution; and [`cond`],
//! evaluation of `#@if`/`#@else`/`#@endif` platform directives on top of the
//! parser's comment-preserving model.

pub mod cond;
pub mod conf;
pub mod platform;
