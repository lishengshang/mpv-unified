//! Package manager library for mpv-config.
//!
//! Provides [`manifest`], the `package.yaml` schema with parsing and
//! validation: identity, platform constraint, dependency/conflict lists, the
//! `files` install map (`src` → `~~/`-prefixed `dest`), and package-layer
//! `config` snippets. Every validation failure is a [`manifest::PackageError`]
//! carrying the offending field name; nothing panics.

pub mod manifest;
