//! `package.yaml` manifest parsing and validation tests.

use super::*;

fn parse_err(yaml: &str) -> PackageError {
    Manifest::parse(yaml).expect_err("expected a PackageError")
}

#[test]
fn valid_full_manifest() {
    let yaml = r#"
name: evafast
version: 1.0.0
description: A test package
author: Jane
homepage: https://example.com
license: MIT
platform: linux
requires: [uosc]
conflicts: [other]
files:
  - src: path/to/file.lua
    dest: ~~/scripts
  - src: shaders/upscale.glsl
    dest: ~~/shaders
config:
  - "script-opts=1"
"#;
    let m = Manifest::parse(yaml).expect("valid manifest");
    assert_eq!(m.name, "evafast");
    assert_eq!(m.version.to_string(), "1.0.0");
    assert_eq!(m.description, "A test package");
    assert_eq!(m.author.as_deref(), Some("Jane"));
    assert_eq!(m.homepage.as_deref(), Some("https://example.com"));
    assert_eq!(m.license.as_deref(), Some("MIT"));
    assert_eq!(m.platform, Platform::Linux);
    assert_eq!(m.requires, ["uosc"]);
    assert_eq!(m.conflicts, ["other"]);
    assert_eq!(m.files.len(), 2);
    assert_eq!(m.files[0].src, "path/to/file.lua");
    assert_eq!(m.files[0].dest, "~~/scripts");
    assert_eq!(m.files[1].dest, "~~/shaders");
    assert_eq!(m.config, ["script-opts=1"]);
}

#[test]
fn minimal_manifest_applies_defaults() {
    let yaml = "name: minimal\nversion: 0.2.1\ndescription: tiny\nfiles:\n  - src: a.lua\n    dest: ~~/scripts\n";
    let m = Manifest::parse(yaml).expect("minimal manifest");
    assert_eq!(m.platform, Platform::All);
    assert!(m.author.is_none());
    assert!(m.homepage.is_none());
    assert!(m.license.is_none());
    assert!(m.requires.is_empty());
    assert!(m.conflicts.is_empty());
    assert!(m.config.is_empty());
    assert_eq!(m.version.to_string(), "0.2.1");
}

#[test]
fn missing_files_field_is_rejected() {
    let e = parse_err("name: x\nversion: 1.0.0\ndescription: d\n");
    assert_eq!(e.field, "files");
}

#[test]
fn empty_files_list_is_rejected() {
    let e = parse_err("name: x\nversion: 1.0.0\ndescription: d\nfiles: []\n");
    assert_eq!(e.field, "files");
}

#[test]
fn missing_name_field_is_rejected() {
    let e =
        parse_err("version: 1.0.0\ndescription: d\nfiles:\n  - src: a.lua\n    dest: ~~/scripts\n");
    assert_eq!(e.field, "name");
}

#[test]
fn invalid_version_is_rejected() {
    for bad in ["1", "1.0", "v1.0.0", "1.0.0-", "1..0", "abc"] {
        let e = parse_err(&format!(
                "name: x\nversion: {bad}\ndescription: d\nfiles:\n  - src: a.lua\n    dest: ~~/scripts\n"
            ));
        assert_eq!(e.field, "version", "version {bad:?}");
    }
}

#[test]
fn invalid_name_is_rejected() {
    for bad in ["EvaFast", "my_pkg", "my pkg", "-lead", "trail-", ""] {
        let e = parse_err(&format!(
                "name: {bad}\nversion: 1.0.0\ndescription: d\nfiles:\n  - src: a.lua\n    dest: ~~/scripts\n"
            ));
        assert_eq!(e.field, "name", "name {bad:?}");
    }
}

#[test]
fn invalid_platform_is_rejected() {
    let e = parse_err("name: x\nversion: 1.0.0\ndescription: d\nplatform: android\nfiles:\n  - src: a.lua\n    dest: ~~/scripts\n");
    assert_eq!(e.field, "platform");
}

#[test]
fn requires_self_reference_is_rejected() {
    let e = parse_err("name: evafast\nversion: 1.0.0\ndescription: d\nrequires: [evafast]\nfiles:\n  - src: a.lua\n    dest: ~~/scripts\n");
    assert_eq!(e.field, "requires");
}

#[test]
fn requires_invalid_name_is_rejected() {
    let e = parse_err("name: x\nversion: 1.0.0\ndescription: d\nrequires: [UPPER]\nfiles:\n  - src: a.lua\n    dest: ~~/scripts\n");
    assert_eq!(e.field, "requires");
}

#[test]
fn conflicts_with_valid_names_parse() {
    let m = Manifest::parse(
            "name: x\nversion: 1.0.0\ndescription: d\nconflicts: [other, third-party]\nfiles:\n  - src: a.lua\n    dest: ~~/scripts\n",
        )
        .expect("conflicts are fine");
    assert_eq!(m.conflicts, ["other", "third-party"]);
}

#[test]
fn invalid_dest_is_rejected() {
    for bad in [
        "scripts",
        "~~",
        "~~/",
        "~~/unknown",
        "~~/scripts/",
        "~~/scripts/../x",
        "http://x/scripts",
    ] {
        let e = parse_err(&format!(
            "name: x\nversion: 1.0.0\ndescription: d\nfiles:\n  - src: a.lua\n    dest: {bad}\n"
        ));
        assert_eq!(e.field, "dest", "dest {bad:?}");
    }
}

#[test]
fn nested_dest_subpath_is_valid() {
    let m = Manifest::parse(
            "name: x\nversion: 1.0.0\ndescription: d\nfiles:\n  - src: a.lua\n    dest: ~~/scripts/file-browser\n",
        )
        .expect("nested subpaths are allowed");
    assert_eq!(m.files[0].dest, "~~/scripts/file-browser");
}

#[test]
fn platform_mismatch_marks_skip() {
    let win = Manifest::parse("name: win-only\nversion: 1.0.0\ndescription: d\nplatform: windows\nfiles:\n  - src: a.lua\n    dest: ~~/scripts\n")
            .expect("parses despite platform constraint");
    assert!(win.platform_mismatch(Platform::Linux));
    assert!(win.platform_mismatch(Platform::MacOS));
    assert!(!win.platform_mismatch(Platform::Windows));

    let all = Manifest::parse(
        "name: any\nversion: 1.0.0\ndescription: d\nfiles:\n  - src: a.lua\n    dest: ~~/scripts\n",
    )
    .expect("all is the default");
    assert!(!all.platform_mismatch(Platform::Linux));
    assert!(!all.platform_mismatch(Platform::Windows));
}

#[test]
fn unknown_yaml_fields_are_ignored() {
    let m = Manifest::parse(
            "name: x\nversion: 1.0.0\ndescription: d\ncategory: video\nfiles:\n  - src: a.lua\n    dest: ~~/scripts\n",
        )
        .expect("extra fields are tolerated");
    assert_eq!(m.name, "x");
}

#[test]
fn malformed_yaml_is_rejected() {
    let e = parse_err("name: [unclosed\nversion: 1.0.0");
    assert_eq!(e.field, "yaml");
}

#[test]
fn display_reports_field_and_message() {
    let e = parse_err(
        "name: x\nversion: nope\ndescription: d\nfiles:\n  - src: a.lua\n    dest: ~~/scripts\n",
    );
    let text = e.to_string();
    assert!(text.starts_with("version: "), "{text}");
    assert!(text.contains("nope"), "{text}");
}
