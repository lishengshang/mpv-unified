//! `index.json` parsing and lookup tests.

use super::*;

const VALID: &str = r#"{
  "packages": [
    { "name": "evafast", "repo": "po5/evafast", "release_tag": "latest", "homepage": "https://github.com/po5/evafast" },
    { "name": "uosc", "repo": "tomasklaen/uosc", "release_tag": "v6.0.0" }
  ]
}"#;

#[test]
fn valid_index_parses_all_entries() {
    let index = Index::parse(VALID).expect("valid index");
    assert_eq!(index.packages.len(), 2);
    let evafast = &index.packages[0];
    assert_eq!(evafast.name, "evafast");
    assert_eq!(evafast.repo, "po5/evafast");
    assert_eq!(evafast.release_tag, "latest");
    assert_eq!(
        evafast.homepage.as_deref(),
        Some("https://github.com/po5/evafast")
    );
    let uosc = &index.packages[1];
    assert_eq!(uosc.release_tag, "v6.0.0");
    assert_eq!(uosc.homepage, None);
}

#[test]
fn find_returns_matching_entry() {
    let index = Index::parse(VALID).expect("valid index");
    let found = index.find("evafast").expect("evafast is in the index");
    assert_eq!(found.repo, "po5/evafast");
    assert_eq!(found.release_tag, "latest");
}

#[test]
fn find_returns_none_for_unknown_name() {
    let index = Index::parse(VALID).expect("valid index");
    assert!(index.find("does-not-exist").is_none());
}

#[test]
fn invalid_name_is_rejected() {
    for name in ["EvaFast", "evafast!", "-evafast", "evafast-"] {
        let json = format!(
            r#"{{ "packages": [ {{ "name": "{name}", "repo": "po5/evafast", "release_tag": "latest" }} ] }}"#
        );
        let err = Index::parse(&json).expect_err("invalid name must fail");
        assert_eq!(err.field, "name", "name {name:?}");
    }
}

#[test]
fn invalid_repo_is_rejected() {
    for repo in [
        "po5",
        "po5/",
        "/evafast",
        "po5/evafast/extra",
        "po5/evafast!",
    ] {
        let json = format!(
            r#"{{ "packages": [ {{ "name": "evafast", "repo": "{repo}", "release_tag": "latest" }} ] }}"#
        );
        let err = Index::parse(&json).expect_err("invalid repo must fail");
        assert_eq!(err.field, "repo", "repo {repo:?}");
        assert!(
            err.message.contains("owner/repo"),
            "message for {repo:?}: {}",
            err.message
        );
    }
}

#[test]
fn invalid_release_tag_is_rejected() {
    for tag in ["", "v1.0/rc1", "v 1.0"] {
        let json = format!(
            r#"{{ "packages": [ {{ "name": "evafast", "repo": "po5/evafast", "release_tag": "{tag}" }} ] }}"#
        );
        let err = Index::parse(&json).expect_err("invalid release_tag must fail");
        assert_eq!(err.field, "release_tag", "tag {tag:?}");
    }
}

#[test]
fn invalid_homepage_is_rejected() {
    let json = r#"{ "packages": [ { "name": "evafast", "repo": "po5/evafast", "release_tag": "latest", "homepage": "ftp://x" } ] }"#;
    let err = Index::parse(json).expect_err("invalid homepage must fail");
    assert_eq!(err.field, "homepage");
}

#[test]
fn duplicate_names_are_rejected() {
    let json = r#"{ "packages": [
      { "name": "evafast", "repo": "po5/evafast", "release_tag": "latest" },
      { "name": "evafast", "repo": "po5/evafast", "release_tag": "v2" }
    ] }"#;
    let err = Index::parse(json).expect_err("duplicate name must fail");
    assert_eq!(err.field, "name");
    assert!(err.message.contains("duplicate"), "{}", err.message);
}

#[test]
fn empty_packages_is_valid() {
    let index = Index::parse(r#"{ "packages": [] }"#).expect("empty index is valid");
    assert!(index.packages.is_empty());
}

#[test]
fn missing_packages_field_is_rejected() {
    let err = Index::parse("{}").expect_err("missing packages must fail");
    assert_eq!(err.field, "packages");
}

#[test]
fn malformed_json_is_rejected() {
    let err = Index::parse("not json").expect_err("malformed json must fail");
    assert_eq!(err.field, "json");
}

#[test]
fn missing_entry_field_is_rejected() {
    let json = r#"{ "packages": [ { "repo": "po5/evafast", "release_tag": "latest" } ] }"#;
    let err = Index::parse(json).expect_err("missing name must fail");
    assert_eq!(err.field, "name");
}
