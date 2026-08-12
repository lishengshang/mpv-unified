//! Tauri commands for the Help page (T21): user.conf editor + tutorials.
//!
//! The editor only ever touches `user/user.conf` — the app layer
//! (`config/base.conf` etc.) is source material and never written here.
//! Saving is round-trip safe: the content is parsed with
//! [`mpv_core::conf::parse`] and re-serialized, and the byte-exact
//! round-trip is verified *before* anything hits the disk; on any
//! failure the error carries the offending line number and the file is
//! left untouched. The write itself is atomic (temp file + rename).

use mpv_core::conf;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// One tutorial in the list view: slug (file name) + first `# ` heading.
#[derive(Debug, Serialize)]
pub struct TutorialInfo {
    pub slug: String,
    pub title: String,
}

fn repo_root() -> PathBuf {
    cli::gen::repo_root()
}

fn user_conf_path(root: &Path) -> PathBuf {
    root.join("user").join("user.conf")
}

/// The current `user/user.conf` content; `None` on first run (the file
/// does not exist yet — the UI shows the onboarding card then).
#[tauri::command]
pub fn get_user_conf() -> Result<Option<String>, String> {
    let path = user_conf_path(&repo_root());
    match fs::read_to_string(&path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(format!("读取 user/user.conf 失败:{}", describe_io(&source))),
    }
}

/// The `user/user.conf` template, for the "打开示例" button.
#[tauri::command]
pub fn load_example_conf() -> Result<String, String> {
    let path = repo_root().join("user").join("user.example.conf");
    fs::read_to_string(&path)
        .map_err(|source| format!("读取 user/user.example.conf 失败:{}", describe_io(&source)))
}

/// Save the editor content to `user/user.conf`.
///
/// Validation (via [`conf::parse`] + round-trip re-serialization) runs
/// entirely in memory first; a parse error aborts with the line number
/// and nothing is written. Only user-layer files are ever touched.
#[tauri::command]
pub fn save_user_conf(content: String) -> Result<(), String> {
    save_user_conf_at(&repo_root(), &content)
}

fn save_user_conf_at(root: &Path, content: &str) -> Result<(), String> {
    let doc = conf::parse(content)
        .map_err(|error| format!("user.conf 校验失败:{error}"))?;
    // Round-trip guard: the parser guarantees byte-exact round-trip for
    // every document it accepts; assert it explicitly so a future parser
    // regression can never silently rewrite the user's file.
    if conf::serialize(&doc) != content {
        return Err("user.conf 校验失败:round-trip 不一致,未保存(请联系维护者)".to_owned());
    }
    let path = user_conf_path(root);
    let user_dir = root.join("user");
    fs::create_dir_all(&user_dir)
        .map_err(|source| format!("创建 user 目录失败:{}", describe_io(&source)))?;
    let tmp = user_dir.join(".user.conf.tmp");
    fs::write(&tmp, content).map_err(|source| {
        format!(
            "写入临时文件失败:{} (源文件未受影响)",
            describe_io(&source)
        )
    })?;
    fs::rename(&tmp, &path).map_err(|source| {
        let _ = fs::remove_file(&tmp);
        format!("写入 user/user.conf 失败:{}", describe_io(&source))
    })?;
    Ok(())
}

/// Every tutorial in `docs/tutorials/`: `[{slug, title}]`, sorted by slug.
///
/// A missing directory yields an empty list (the UI shows a hint), not
/// an error — the Help page must stay usable without docs.
#[tauri::command]
pub fn list_tutorials() -> Result<Vec<TutorialInfo>, String> {
    let dir = repo_root().join("docs").join("tutorials");
    let mut slugs: Vec<String> = match fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
            .filter_map(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|name| name.strip_suffix(".md").unwrap_or(name).to_owned())
            })
            .filter(|slug| valid_slug(slug))
            .collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(source) => return Err(format!("读取 docs/tutorials 失败:{}", describe_io(&source))),
    };
    slugs.sort();
    let mut tutorials = Vec::with_capacity(slugs.len());
    for slug in slugs {
        let title = read_title(&dir.join(format!("{slug}.md"))).unwrap_or_else(|| slug.clone());
        tutorials.push(TutorialInfo { slug, title });
    }
    Ok(tutorials)
}

/// The full markdown body of one tutorial, rendered as plain text by the
/// frontend. The slug is restricted to `[a-z0-9-]` so the path can never
/// escape `docs/tutorials/`.
#[tauri::command]
pub fn get_tutorial(slug: String) -> Result<String, String> {
    if !valid_slug(&slug) {
        return Err(format!("非法的教程标识:{slug}"));
    }
    let path = repo_root().join("docs").join("tutorials").join(format!("{slug}.md"));
    fs::read_to_string(&path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            format!("未找到教程:{slug}")
        } else {
            format!("读取教程 {slug} 失败:{}", describe_io(&source))
        }
    })
}

/// A slug is a lowercase `[a-z0-9-]` identifier (no dots, no slashes).
fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 64
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Title = the first `# ` line; `None` when the file has none (a
/// structurally broken tutorial still lists, with the slug as fallback).
fn read_title(path: &Path) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    raw.lines().find_map(|line| line.strip_prefix("# ")).map(str::to_owned)
}

fn describe_io(source: &std::io::Error) -> String {
    source.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(tag: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("help-test-{tag}-{nonce}"));
        fs::create_dir_all(&dir).expect("create temp root");
        dir
    }

    #[test]
    fn save_writes_content_round_trip_lossless() {
        let root = temp_root("save-ok");
        let content = "# 注释\nsub-font-size=44\nhwdec=no\n";

        save_user_conf_at(&root, content).expect("valid content saves");

        let saved = fs::read_to_string(user_conf_path(&root)).expect("file exists after save");
        assert_eq!(saved, content, "round-trip must be byte-identical");
        assert!(!root.join("user").join(".user.conf.tmp").exists(), "no temp file left");
    }

    #[test]
    fn save_rejects_unclosed_quote_with_line_number_and_keeps_file_untouched() {
        let root = temp_root("save-bad");
        let original = "volume=80\n";
        save_user_conf_at(&root, original).expect("seed valid content");
        let bad = "volume=80\nsub-font-size='44\n";

        let error = save_user_conf_at(&root, bad).expect_err("unclosed quote must fail");
        assert!(error.contains("line 2"), "error must carry the line number: {error}");
        assert!(error.contains("quote"), "error must name the cause: {error}");
        let on_disk = fs::read_to_string(user_conf_path(&root)).expect("file still readable");
        assert_eq!(on_disk, original, "file must be untouched on failure");
        assert!(!root.join("user").join(".user.conf.tmp").exists(), "no temp file left");
    }

    #[test]
    fn save_creates_file_when_user_conf_missing() {
        let root = temp_root("save-first");
        let content = "idle=yes\n";

        save_user_conf_at(&root, content).expect("first save creates the file");

        assert!(user_conf_path(&root).is_file(), "user.conf must exist after first save");
        assert_eq!(
            fs::read_to_string(user_conf_path(&root)).expect("read back"),
            content
        );
    }

    #[test]
    fn slugs_allow_only_lowercase_dash_names() {
        assert!(valid_slug("sub-font-size"));
        assert!(valid_slug("hdr"));
        assert!(valid_slug("a1"));
        assert!(!valid_slug("../secret"));
        assert!(!valid_slug("a/b"));
        assert!(!valid_slug("a.md"));
        assert!(!valid_slug(""));
        assert!(!valid_slug("UPPER"));
        assert!(!valid_slug(&"x".repeat(65)));
    }
}
