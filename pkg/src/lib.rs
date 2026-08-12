//! Package manager library for mpv-config.
//!
//! - [`manifest`]: the `package.yaml` schema with parsing and validation —
//!   identity, platform constraint, dependency/conflict lists, the `files`
//!   install map (`src` → `~~/`-prefixed `dest`), and package-layer `config`
//!   snippets.
//! - [`index`]: the `index.json` package index schema with validation and
//!   name lookup.
//! - [`fetch`]: fetching the index and package archives from GitHub
//!   Releases over the [`fetch::Fetcher`] abstraction, with validation
//!   (manifest matches the index) and atomic cache writes.
//!
//! Every failure is a typed error carrying the offending field or URL;
//! nothing panics.

pub mod fetch;
pub mod index;
pub mod manifest;
