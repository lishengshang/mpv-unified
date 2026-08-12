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
//! - [`deps`]: name-level dependency resolution (topological install order,
//!   cycle and missing-dependency detection) and conflict detection
//!   (`conflicts` exclusivity and dest-path overlap), consumed by the
//!   installer before it touches the filesystem.
//! - [`lock`]: the `packages.lock` document — schema, atomic read/write
//!   (tmp + rename), and the [`lock::verify`]/[`lock::repair`] consistency
//!   check between the lock and the repository files.
//! - [`catalog`]: the store-page merge of lock + pending + local + index
//!   sources into one sorted, status-annotated package list (consumed by
//!   the T20 Tauri store page).
//! - [`upgrade`]: app update checking against the index `latest_version`
//!   field, and the guided upgrade (download → validate → backup → overlay,
//!   user layer preserved, rollback on failure) behind the T22 UI/CLI.
//!
//! Every failure is a typed error carrying the offending field or URL;
//! nothing panics.

pub mod catalog;
pub mod deps;
pub mod fetch;
pub mod index;
pub mod lock;
pub mod manifest;
pub mod upgrade;
