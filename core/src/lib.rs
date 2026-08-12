//! Core library for the mpv-config generator.
//!
//! Provides [`conf`], a lossless `mpv.conf` parser: any file that parses
//! successfully round-trips back to the exact same bytes; [`platform`], host
//! platform detection plus mpv config-directory resolution; [`cond`],
//! evaluation of `#@if`/`#@else`/`#@endif` platform directives on top of the
//! parser's comment-preserving model; [`profiles`], the profile-card engine
//! (parse `config/profiles.yaml`, render mpv profile blocks, persist the
//! enabled set in `user/profiles-state.json`); and [`uosc`], turning enabled
//! profiles into uosc context-menu items (`apply-profile` bindings).

pub mod cond;
pub mod conf;
pub mod merge;
pub mod options_gui;
pub mod platform;
pub mod profiles;
pub mod uosc;
