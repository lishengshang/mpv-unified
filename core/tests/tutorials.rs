//! Structure validation for the long-tail tutorial docs (T21).
//!
//! `docs/tutorials/` must hold at least 30 real tutorials, each with a
//! `# ` title, at least one `## ` section, and 20+ lines of actual
//! content. Runs on every CI platform via `cargo test --workspace`;
//! `tools/check-tutorials.sh` mirrors the same checks for manual runs.

use std::fs;
use std::path::{Path, PathBuf};

/// Minimum number of tutorials the docs directory must contain.
const MIN_TUTORIALS: usize = 30;
/// Minimum total line count per tutorial (keeps "real content" honest).
const MIN_LINES: usize = 20;

fn tutorials_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("docs")
        .join("tutorials")
}

fn tutorial_slugs() -> Vec<String> {
    let mut slugs: Vec<String> = fs::read_dir(tutorials_dir())
        .unwrap_or_else(|error| panic!("docs/tutorials 不可读:{error}"))
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
        .filter_map(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| name.strip_suffix(".md").unwrap_or(name).to_owned())
        })
        .collect();
    slugs.sort();
    slugs
}

#[test]
fn has_at_least_30_tutorials() {
    let slugs = tutorial_slugs();
    assert!(
        slugs.len() >= MIN_TUTORIALS,
        "docs/tutorials 只有 {} 篇教程,要求 ≥ {MIN_TUTORIALS}",
        slugs.len()
    );
}

#[test]
fn every_tutorial_is_well_formed() {
    for slug in tutorial_slugs() {
        let path = tutorials_dir().join(format!("{slug}.md"));
        let raw =
            fs::read_to_string(&path).unwrap_or_else(|error| panic!("读取 {slug}.md 失败:{error}"));
        let lines: Vec<&str> = raw.lines().collect();

        assert!(
            lines.iter().any(|line| line.starts_with("# ")),
            "{slug}.md 缺少 `# ` 标题行"
        );
        assert!(
            lines.iter().any(|line| line.starts_with("## ")),
            "{slug}.md 缺少 `## ` 小节"
        );
        assert!(
            lines.len() >= MIN_LINES,
            "{slug}.md 只有 {} 行,要求 ≥ {MIN_LINES}",
            lines.len()
        );
    }
}
