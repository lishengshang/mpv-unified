//! Four-layer merge engine: `base` → `platform` → `packages` → `user`.
//!
//! Combines the four configuration layers of decision D5 into one
//! [`ConfDoc`]. Layers are applied strictly in order; later layers override
//! earlier ones. Every layer must already be a parsed document (use
//! [`MergeLayer::parse`] for the layer-aware parse entry), so merging
//! itself cannot fail and never produces partial output.
//!
//! # Override semantics
//!
//! - **Keys**: a `key=value` line and a flag line (empty value) are both
//!   identified by their [`Entry::KeyValue::key`] field. A later layer's
//!   entry with the same key **replaces** the earlier entry **in place** —
//!   the earlier entry's position is kept, so the earlier layer's layout
//!   stays stable. A key no earlier layer defines is **appended**:
//!   top-level keys go to the end of the document; keys inside an existing
//!   profile go to the end of that profile's block (before the next
//!   `[name]` header) so they stay inside the block.
//! - **Profiles**: same-named `[name]` blocks merge into one block — a
//!   single header keeps the position of its first occurrence while the
//!   later layer's header text wins — and the keys inside the block follow
//!   the same replace-in-place / append rules. Top-level keys and profile
//!   keys are independent scopes: the composite key is `(profile, key)`,
//!   with top-level keys using `(None, key)`.
//! - **Comments and blank lines**: never replaced. Every comment/blank
//!   line of every layer is appended in layer order, so no comment is ever
//!   lost; comments inside profile blocks are preserved too.
//! - **Trailing newline**: the merged document's `ends_with_newline` is
//!   taken from the last (highest-priority) layer.
//!
//! # Atomicity
//!
//! [`MergeLayer::parse`] is the only entry that can fail: it tags every
//! parse error with the layer's name plus the 1-based line number. A caller
//! that parses all layers before calling [`merge`] / [`merge_docs`] can
//! never observe a partial document — either every layer parses and the
//! merge runs, or the first bad line yields a [`MergeError`] and no output
//! exists at all.
//!
//! This module is pure `std`, never panics, and is consumed by the `gen`
//! command (task 6), the profile-card engine (task 18) and the option-form
//! writer (task 19).

use crate::conf::{parse, ConfDoc, Entry};
use std::collections::HashMap;
use std::fmt;

/// One merge layer: a display name (used in error messages) plus the parsed
/// document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeLayer {
    /// Human-readable identity of the layer, e.g. `base.conf`, `linux.conf`,
    /// `package:uosc`, or `user.conf`. Used to tag [`MergeError`]s.
    pub name: String,
    /// The layer's entries.
    pub doc: ConfDoc,
}

impl MergeLayer {
    /// Wrap an already-parsed document as a layer.
    #[must_use]
    pub fn new(name: impl Into<String>, doc: ConfDoc) -> Self {
        Self {
            name: name.into(),
            doc,
        }
    }

    /// Parse `text` as one layer.
    ///
    /// This is the layer-aware parse entry: on failure the returned
    /// [`MergeError`] carries this layer's `name` plus the offending
    /// 1-based line, so multi-layer merges can name the culprit.
    pub fn parse(name: impl Into<String>, text: &str) -> Result<Self, MergeError> {
        let name = name.into();
        match parse(text) {
            Ok(doc) => Ok(Self { name, doc }),
            Err(e) => Err(MergeError {
                layer: name,
                line: e.line,
                message: e.message,
            }),
        }
    }
}

/// A failure attributed to a specific layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeError {
    /// Name of the layer that failed.
    pub layer: String,
    /// 1-based line number within that layer.
    pub line: usize,
    /// Human-readable cause.
    pub message: String,
}

impl fmt::Display for MergeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: line {}: {}", self.layer, self.line, self.message)
    }
}

impl std::error::Error for MergeError {}

/// Merge the four layers of decision D5: `base` first, then `platform`,
/// then the `packages` fragments, then `user` (skipped when `None`).
///
/// Packages form one logical layer, so they are applied in ascending
/// `name` order regardless of the caller's slice order; a later package in
/// that order wins conflicts. Later layers override earlier ones; see the
/// module docs for the exact override and comment-preservation semantics.
///
/// Pass an empty slice when no packages are installed and `None` when there
/// is no user file — both cases are skipped without error.
pub fn merge_docs(
    base: &MergeLayer,
    platform: &MergeLayer,
    packages: &[MergeLayer],
    user: Option<&MergeLayer>,
) -> Result<ConfDoc, MergeError> {
    let mut ordered = Vec::with_capacity(2 + packages.len() + usize::from(user.is_some()));
    ordered.push(base.clone());
    ordered.push(platform.clone());
    let mut packages = packages.to_vec();
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    ordered.extend(packages);
    if let Some(user) = user {
        ordered.push(user.clone());
    }
    merge(ordered)
}

/// Merge pre-parsed layers into one document; layer order is priority order
/// (index 0 is applied first, the last layer wins conflicts).
///
/// The layers are already parsed, so this cannot fail: the `Result` keeps
/// the signature uniform with [`MergeLayer::parse`] so callers can thread
/// both through `?`. Merge errors originate exclusively from parsing.
pub fn merge(layers: Vec<MergeLayer>) -> Result<ConfDoc, MergeError> {
    Ok(merge_unchecked(&layers))
}

/// Apply `layers` in order; infallible because every layer is already a
/// valid [`ConfDoc`].
fn merge_unchecked(layers: &[MergeLayer]) -> ConfDoc {
    let mut state = MergeState::default();
    for layer in layers {
        state.apply(layer);
    }
    ConfDoc {
        entries: state.entries,
        ends_with_newline: layers.last().is_some_and(|l| l.doc.ends_with_newline),
    }
}

/// Builder for the merged document: the entry list plus the indexes that
/// make replace-in-place and in-scope appends cheap to find.
#[derive(Default)]
struct MergeState {
    entries: Vec<Entry>,
    /// Composite key `(profile, key)` → entry index; top-level keys use
    /// `None` as the profile. The value is the position of the current
    /// (highest-priority) entry for that key.
    keys: HashMap<(Option<String>, String), usize>,
    /// Profile name → index of its `[name]` header entry.
    profiles: HashMap<String, usize>,
}

impl MergeState {
    /// The composite key of `key` inside `profile`; `None` means top level.
    fn composite_key(profile: Option<&str>, key: &str) -> (Option<String>, String) {
        (profile.map(str::to_owned), key.to_owned())
    }

    /// Merge one layer: comments and blank lines are appended, keys replace
    /// same-scope same-key entries in place (or append), and profile
    /// headers merge into existing blocks (or append new ones).
    fn apply(&mut self, layer: &MergeLayer) {
        // Profile scope is tracked per layer: keys after a `[name]` header
        // belong to that profile until the next header.
        let mut profile: Option<&str> = None;
        for entry in &layer.doc.entries {
            match entry {
                Entry::ProfileStart { name, raw_line } => {
                    self.merge_profile_header(name, raw_line);
                    profile = Some(name);
                }
                Entry::KeyValue { key, .. } => {
                    let composite = Self::composite_key(profile, key);
                    let existing = self.keys.get(&composite).copied();
                    if let Some(at) = existing {
                        // Same key in the same scope: replace in place so
                        // the earlier layer's layout stays stable.
                        self.entries[at] = entry.clone();
                    } else {
                        self.insert_new(composite, entry.clone(), profile);
                    }
                }
                Entry::Comment { .. } | Entry::Blank { .. } => {
                    self.entries.push(entry.clone());
                }
            }
        }
    }

    /// Insert (or re-place) a profile header. Same-named profiles merge
    /// into one block: the header keeps the position of its first
    /// occurrence while the later layer's header text wins; the block's
    /// keys merge via the composite-key rules as the layer continues.
    fn merge_profile_header(&mut self, name: &str, raw_line: &str) {
        let header_entry = Entry::ProfileStart {
            name: name.to_owned(),
            raw_line: raw_line.to_owned(),
        };
        match self.profiles.get(name).copied() {
            Some(header) => self.entries[header] = header_entry,
            None => {
                self.entries.push(header_entry);
                self.profiles
                    .insert(name.to_owned(), self.entries.len() - 1);
            }
        }
    }

    /// Append a brand-new key: top-level keys go to the end of the
    /// document, profile keys to the end of their profile's block (before
    /// the next `[header]` line) so they stay inside the profile.
    ///
    /// Every entry after the insertion point shifts one place, so all
    /// tracked indexes at or past it are bumped. `profile` is `Some` only
    /// when the same layer emitted that profile's header just before, which
    /// guarantees the slot exists; the fallback keeps this panic-free.
    fn insert_new(
        &mut self,
        composite: (Option<String>, String),
        entry: Entry,
        profile: Option<&str>,
    ) {
        let at = match profile.and_then(|name| self.profiles.get(name)).copied() {
            Some(header) => self.block_end(header),
            None => self.entries.len(),
        };
        self.entries.insert(at, entry);
        self.bump_indexes(at);
        self.keys.insert(composite, at);
    }

    /// Index just past the last entry of the profile block that starts at
    /// `header`: the first [`Entry::ProfileStart`] after it, or the end of
    /// the document.
    fn block_end(&self, header: usize) -> usize {
        self.entries[header + 1..]
            .iter()
            .position(|e| matches!(e, Entry::ProfileStart { .. }))
            .map_or(self.entries.len(), |offset| header + 1 + offset)
    }

    /// After inserting at `at`, every tracked index that pointed at or past
    /// the insertion point moved one place to the right.
    fn bump_indexes(&mut self, at: usize) {
        for value in self.keys.values_mut() {
            if *value >= at {
                *value += 1;
            }
        }
        for header in self.profiles.values_mut() {
            if *header >= at {
                *header += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conf::serialize;

    /// Parse `text` as a layer; test fixtures must be valid, so a parse
    /// failure here is a bug in the test itself.
    fn layer(name: &str, text: &str) -> MergeLayer {
        MergeLayer::parse(name, text)
            .unwrap_or_else(|e| panic!("fixture {name:?} failed to parse: {e}"))
    }

    /// Merge the four layers and return the serialized merged document.
    fn merge_text(
        base: &str,
        platform: &str,
        packages: &[(&str, &str)],
        user: Option<&str>,
    ) -> String {
        let packages: Vec<MergeLayer> = packages
            .iter()
            .map(|(name, text)| layer(name, text))
            .collect();
        let user = user.map(|text| layer("user.conf", text));
        let merged = merge_docs(
            &layer("base.conf", base),
            &layer("linux.conf", platform),
            &packages,
            user.as_ref(),
        )
        .unwrap_or_else(|e| panic!("merge failed: {e}"));
        serialize(&merged)
    }

    // ------------------------------------------------------ four-layer order

    #[test]
    fn later_layer_overrides_earlier_for_same_key_in_place() {
        // volume=100 in base is overridden by platform (90), package (80)
        // and finally user (70). The winning entry keeps its base position;
        // the base-only key and every layer's comments stay.
        let out = merge_text(
            "# base header\nvolume=100\nkeep=1\n",
            "# platform\nvolume=90\n",
            &[("pkg", "volume=80\n")],
            Some("# user\nvolume=70\n"),
        );
        assert_eq!(
            out,
            "# base header\nvolume=70\nkeep=1\n# platform\n# user\n"
        );
    }

    #[test]
    fn missing_user_layer_is_skipped() {
        let out = merge_text(
            "volume=100\n",
            "volume=90\n",
            &[("pkg", "volume=80\n")],
            None,
        );
        assert_eq!(out, "volume=80\n");
    }

    #[test]
    fn empty_packages_are_skipped() {
        let out = merge_text("volume=100\n", "volume=90\n", &[], None);
        assert_eq!(out, "volume=90\n");
    }

    #[test]
    fn packages_apply_in_name_sorted_order() {
        // merge_docs sorts packages by name: a-first applies before z-last,
        // so z-last wins the conflict deterministically.
        let out = merge_text(
            "k=0\n",
            "k=0\n",
            &[("z-last", "k=1\n"), ("a-first", "k=2\n")],
            None,
        );
        assert_eq!(out, "k=1\n");
    }

    // ------------------------------------------------------ profile merging

    #[test]
    fn same_named_profile_blocks_merge_with_inner_override() {
        let out = merge_text(
            "[Video]\nprofile-desc=base\nquality=high\n[Audio]\nvol=100\n",
            "[Video]\nquality=medium\n",
            &[],
            None,
        );
        assert_eq!(
            out,
            "[Video]\nprofile-desc=base\nquality=medium\n[Audio]\nvol=100\n"
        );
    }

    #[test]
    fn top_level_and_profile_keys_are_independent_scopes() {
        // The composite key is (profile, key): the top-level key1 and the
        // key1 inside [P] never collide.
        let out = merge_text(
            "key1=top\n[P]\nkey1=inner\n",
            "[P]\nkey1=inner2\n",
            &[],
            None,
        );
        assert_eq!(out, "key1=top\n[P]\nkey1=inner2\n");
    }

    #[test]
    fn new_profile_key_is_inserted_inside_its_block() {
        let out = merge_text("[P]\nx=1\n[Q]\ny=2\n", "[P]\nz=3\n", &[], None);
        // z=3 must land inside [P] — before [Q], not at the document end,
        // where it would escape the profile.
        assert_eq!(out, "[P]\nx=1\nz=3\n[Q]\ny=2\n");
    }

    #[test]
    fn comments_inside_profile_blocks_are_preserved() {
        let out = merge_text(
            "[P]\n# block note\nx=1\n[Q]\ny=2\n",
            "[P]\nz=3\n",
            &[],
            None,
        );
        assert_eq!(out, "[P]\n# block note\nx=1\nz=3\n[Q]\ny=2\n");
    }

    #[test]
    fn brand_new_profile_from_later_layer_appends_at_end() {
        let out = merge_text("k1=1\n", "[P]\np=2\n", &[], None);
        assert_eq!(out, "k1=1\n[P]\np=2\n");
    }

    #[test]
    fn later_profile_header_text_wins_position_stays() {
        let out = merge_text("[P]\nx=1\n", "  [P]\nx=2\n", &[], None);
        assert_eq!(out, "  [P]\nx=2\n");
    }

    // --------------------------------------------- comments and blank lines

    #[test]
    fn comments_and_blanks_of_all_layers_are_preserved_in_layer_order() {
        let out = merge_text(
            "# base1\nk=1\n\n# base2\n",
            "# platform\n",
            &[("pkg-a", "# pkgA\n"), ("pkg-b", "# pkgB\n")],
            Some("# user\n"),
        );
        assert_eq!(
            out,
            "# base1\nk=1\n\n# base2\n# platform\n# pkgA\n# pkgB\n# user\n"
        );
    }

    #[test]
    fn new_top_level_key_appends_after_existing_layout() {
        let out = merge_text("# head\nk1=1\n# tail\nk2=2\n", "k3=3\n", &[], None);
        assert_eq!(out, "# head\nk1=1\n# tail\nk2=2\nk3=3\n");
    }

    // ------------------------------------------------------ key equivalence

    #[test]
    fn flag_line_and_key_value_override_each_other() {
        // A flag line (empty value) and key=value are the same key.
        let out = merge_text("keep-open\n", "keep-open=yes\n", &[], None);
        assert_eq!(out, "keep-open=yes\n");

        let out = merge_text("no-osd-bar=yes\n", "no-osd-bar\n", &[], None);
        assert_eq!(out, "no-osd-bar\n");
    }

    // ------------------------------------------------------------- newlines

    #[test]
    fn trailing_newline_comes_from_last_layer() {
        // base without trailing newline, user with → merged ends with \n.
        let out = merge_text("a=1", "", &[], Some("b=2\n"));
        assert_eq!(out, "a=1\nb=2\n");

        // base with, user without → merged does not end with \n.
        let out = merge_text("a=1\n", "", &[], Some("b=2"));
        assert_eq!(out, "a=1\nb=2");
    }

    // ------------------------------------------------------------ atomicity

    /// The real-consumer shape: parse every layer with `?`, then merge.
    fn merge_with_parsed_user(
        base: &str,
        platform: &str,
        user_text: &str,
    ) -> Result<ConfDoc, MergeError> {
        let base = MergeLayer::parse("base.conf", base)?;
        let platform = MergeLayer::parse("linux.conf", platform)?;
        let user = MergeLayer::parse("user.conf", user_text)?;
        merge_docs(&base, &platform, &[], Some(&user))
    }

    #[test]
    fn bad_layer_aborts_the_whole_merge_atomically() {
        // A bad line in the user layer: the error names the layer and the
        // 1-based line, and no partial document exists — only Err.
        let err = merge_with_parsed_user("a=1\n", "b=2\n", "=broken\n").unwrap_err();
        assert_eq!(err.layer, "user.conf");
        assert_eq!(err.line, 1);
        assert!(err.message.contains("missing option name"), "{err}");

        // A bad line in the middle layer is tagged with that layer; the
        // later layer is never merged onto a partial result.
        let err = merge_with_parsed_user("a=1\n", "b=2\n=broken\n", "c=3\n").unwrap_err();
        assert_eq!(err.layer, "linux.conf");
        assert_eq!(err.line, 2);
        assert!(err.message.contains("missing option name"), "{err}");
    }

    // ------------------------------------------------------------ plain API

    #[test]
    fn merge_applies_layers_in_index_order() {
        let doc = merge(vec![
            layer("first", "k=1\n"),
            layer("second", "k=2\n"),
            layer("third", "# c\n"),
        ])
        .unwrap();
        assert_eq!(serialize(&doc), "k=2\n# c\n");
    }

    #[test]
    fn merge_with_no_layers_is_an_empty_document() {
        let doc = merge(Vec::new()).unwrap();
        assert_eq!(doc.entries, Vec::<Entry>::new());
        assert!(!doc.ends_with_newline);
    }
}
