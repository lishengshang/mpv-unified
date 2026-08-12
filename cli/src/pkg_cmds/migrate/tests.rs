use super::*;

#[test]
fn infer_github_repo_lowercases_and_slugifies() {
    let candidate = infer_candidate("https://github.com/bloc97/Anime4K").expect("github url");
    assert_eq!(candidate.name, "anime4k");
    assert_eq!(candidate.display, "bloc97/Anime4K");
}

#[test]
fn infer_github_repo_with_underscore_becomes_hyphen() {
    let candidate = infer_candidate("https://github.com/foo/bar_baz").expect("github url");
    assert_eq!(candidate.name, "bar-baz");
}

#[test]
fn infer_gist_uses_owner_and_8_char_hash() {
    let candidate = infer_candidate("https://gist.github.com/igv/a015fc885d5c22e6891820ad89555637")
        .expect("gist url");
    assert_eq!(candidate.name, "igv-a015fc88");
}

#[test]
fn infer_strips_trailing_slash_and_git_suffix() {
    let candidate = infer_candidate("https://github.com/po5/evafast.git/").expect("github url");
    assert_eq!(candidate.name, "evafast");
    let scp = infer_candidate("git@github.com:bloc97/Anime4K.git").expect("scp url");
    assert_eq!(scp.name, "anime4k");
}

#[test]
fn infer_returns_none_without_repo_segment() {
    assert!(infer_candidate("https://example.com").is_none());
    assert!(infer_candidate("https://github.com/po5/").is_none());
    assert!(infer_candidate("not a url").is_none());
    assert!(infer_candidate("").is_none());
}

#[test]
fn slugify_collapses_and_trims_hyphens() {
    assert_eq!(slugify("a--b"), Some("a-b".to_owned()));
    assert_eq!(slugify("-a-"), Some("a".to_owned()));
    assert_eq!(slugify("___"), None);
}

#[test]
fn validate_name_probes_manifest_rules() {
    assert!(validate_package_name("evafast").is_ok());
    assert!(validate_package_name("anime4k").is_ok());
    assert!(validate_package_name("mpv-scripts-2").is_ok());
    assert!(validate_package_name("Uppercase").is_err());
    assert!(validate_package_name("").is_err());
    assert!(validate_package_name("-leading").is_err());
}

#[test]
fn uniquify_appends_incrementing_suffix_and_notes() {
    let mut used = HashSet::from(["mpv-scripts".to_owned()]);
    let mut notes = Vec::new();
    let name = uniquify(
        "mpv-scripts".to_owned(),
        &mut used,
        5,
        "https://github.com/Eisa01/mpv-scripts",
        &mut notes,
    );
    assert_eq!(name, "mpv-scripts-2");
    assert!(used.contains("mpv-scripts-2"));
    assert_eq!(notes.len(), 1);
}

#[test]
fn report_markdown_lists_successes_failures_and_notes() {
    let report = MigrationReport {
        input_display: "manager.json".to_owned(),
        out_display: "packages/pending".to_owned(),
        report_display: "docs/migration-report.md".to_owned(),
        succeeded: vec![MigratedEntry {
            name: "evafast".to_owned(),
            git: "https://github.com/po5/evafast".to_owned(),
            dest: "~~/scripts".to_owned(),
        }],
        failed: vec![FailedEntry {
            index: 2,
            git: "https://example.com".to_owned(),
            reason: "无法从 git URL 推断包名".to_owned(),
        }],
        notes: vec!["条目 #5 重名".to_owned()],
    };
    let markdown = report.to_markdown();
    assert!(markdown.contains("成功 1 / 失败 1"), "{markdown}");
    assert!(
        markdown.contains("| 1 | evafast | https://github.com/po5/evafast | ~~/scripts |"),
        "{markdown}"
    );
    assert!(
        markdown.contains("| 2 | https://example.com | 无法从 git URL 推断包名 |"),
        "{markdown}"
    );
    assert!(markdown.contains("## 备注"), "{markdown}");
}

#[test]
fn stdout_report_is_readable() {
    let report = MigrationReport {
        input_display: "manager.json".to_owned(),
        out_display: "packages/pending".to_owned(),
        report_display: "docs/migration-report.md".to_owned(),
        succeeded: vec![],
        failed: vec![],
        notes: vec![],
    };
    let text = report.to_string();
    assert!(text.contains("成功 0 / 失败 0"), "{text}");
    assert!(
        text.contains("报告已写入: docs/migration-report.md"),
        "{text}"
    );
}
