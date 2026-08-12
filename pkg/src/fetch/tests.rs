//! Offline tests for index fetching and package fetching, all through
//! [`MockFetcher`] with hermetic zip/tar.gz fixtures. The single real-network
//! test is marked `#[ignore]`.

use std::fs;
use std::path::Path;
use std::sync::Mutex;

use super::package::{fetch_package, PackageArchive};
use super::*;
use crate::index::{Index, IndexError, PackageEntry};

use testutil::{manifest_yaml, tar_gz_bytes, zip_bytes, TempDir};

const RELEASE_BASE: &str = "https://api.github.com/repos";

fn entry(name: &str, repo: &str, tag: &str) -> PackageEntry {
    PackageEntry {
        name: name.to_owned(),
        repo: repo.to_owned(),
        release_tag: tag.to_owned(),
        homepage: None,
    }
}

fn release_body(assets: &[(&str, &str)]) -> String {
    let list: Vec<String> = assets
        .iter()
        .map(|(name, url)| {
            format!(r#"{{ "name": "{name}", "browser_download_url": "{url}", "size": 42 }}"#)
        })
        .collect();
    format!(
        r#"{{ "tag_name": "v1.0.0", "assets": [ {} ] }}"#,
        list.join(",")
    )
}

fn release_url(entry: &PackageEntry) -> String {
    let suffix = if entry.release_tag == "latest" {
        "latest".to_owned()
    } else {
        format!("tags/{}", entry.release_tag)
    };
    format!("{RELEASE_BASE}/{}/releases/{suffix}", entry.repo)
}

/// A fetcher answering a release listing plus one downloadable asset.
fn mock_release(
    entry: &PackageEntry,
    asset_name: &str,
    asset_bytes: Vec<u8>,
) -> (MockFetcher, String) {
    let download_url = format!("https://download.example/{asset_name}");
    let body = release_body(&[(asset_name, &download_url)]);
    let fetcher = MockFetcher::new()
        .with(&release_url(entry), body.into_bytes())
        .with(&download_url, asset_bytes);
    (fetcher, download_url)
}

/// No `.fetch-*` work directories may survive a fetch (success or failure).
fn assert_no_work_leftovers(cache: &Path) {
    let leftovers: Vec<String> = fs::read_dir(cache)
        .expect("cache dir readable")
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with(".fetch-"))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(leftovers.is_empty(), "work dirs left behind: {leftovers:?}");
}

#[test]
fn mock_fetcher_serves_preset_bytes() {
    let fetcher = MockFetcher::new().with("https://x.example/a", b"payload".to_vec());
    assert_eq!(
        fetcher.get("https://x.example/a").expect("configured"),
        b"payload"
    );
}

#[test]
fn mock_fetcher_reports_404_for_unconfigured_url() {
    let fetcher = MockFetcher::new();
    let err = fetcher
        .get("https://x.example/missing")
        .expect_err("unconfigured URL must fail");
    assert!(
        matches!(
            err,
            FetchError::Http {
                status: Some(404),
                ..
            }
        ),
        "{err:?}"
    );
}

#[test]
fn fetch_index_returns_document_text() {
    let fetcher = MockFetcher::new().with(
        "https://x.example/index.json",
        b"{\"packages\": []}".to_vec(),
    );
    let text = fetch_index(&fetcher, "https://x.example/index.json").expect("fetch");
    assert_eq!(text, r#"{"packages": []}"#);
}

#[test]
fn fetch_index_rejects_non_utf8_body() {
    let fetcher = MockFetcher::new().with("https://x.example/index.json", vec![0xFF, 0xFE]);
    let err =
        fetch_index(&fetcher, "https://x.example/index.json").expect_err("non-utf8 must fail");
    assert!(matches!(err, FetchError::Utf8 { .. }), "{err:?}");
}

#[test]
fn update_index_writes_validated_index_atomically() {
    let cache = TempDir::new("update");
    let json = r#"{ "packages": [ { "name": "evafast", "repo": "po5/evafast", "release_tag": "latest" } ] }"#;
    let fetcher = MockFetcher::new().with("https://x.example/index.json", json.as_bytes().to_vec());
    let target =
        update_index(&fetcher, "https://x.example/index.json", cache.path()).expect("update");
    assert_eq!(target, cache.path().join("index.json"));
    let written = fs::read_to_string(&target).expect("written index");
    assert_eq!(written, json);
    assert_eq!(
        Index::parse(&written).expect("round-trips").packages.len(),
        1
    );
}

#[test]
fn update_index_refuses_invalid_index_and_writes_nothing() {
    let cache = TempDir::new("update-bad");
    let fetcher = MockFetcher::new().with("https://x.example/index.json", b"not json".to_vec());
    let err = update_index(&fetcher, "https://x.example/index.json", cache.path())
        .expect_err("invalid index must fail");
    assert!(
        matches!(err, FetchError::Index(IndexError { field: "json", .. })),
        "{err:?}"
    );
    assert!(
        !cache.path().join("index.json").exists(),
        "no half-written index"
    );
}

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn cache_dir_prefers_override_env() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let override_dir = TempDir::new("override");
    for var in [
        "MPV_CONFIG_CACHE",
        "XDG_CACHE_HOME",
        "LOCALAPPDATA",
        "HOME",
        "USERPROFILE",
    ] {
        std::env::remove_var(var);
    }
    std::env::set_var("MPV_CONFIG_CACHE", override_dir.path());
    assert_eq!(cache_dir().expect("resolved"), override_dir.path());
    std::env::remove_var("MPV_CONFIG_CACHE");
}

#[test]
fn cache_dir_defaults_to_home_cache() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let home = TempDir::new("home");
    for var in [
        "MPV_CONFIG_CACHE",
        "XDG_CACHE_HOME",
        "LOCALAPPDATA",
        "USERPROFILE",
    ] {
        std::env::remove_var(var);
    }
    std::env::set_var("HOME", home.path());
    assert_eq!(
        cache_dir().expect("resolved"),
        home.path().join(".cache").join("mpv-config")
    );
}

#[test]
fn fetch_package_happy_path_zip() {
    let entry = entry("evafast", "po5/evafast", "latest");
    let archive = zip_bytes(&[
        ("package.yaml", manifest_yaml("evafast", "1.2.3").as_bytes()),
        ("file.lua", b"-- fixture script"),
    ]);
    let (fetcher, _) = mock_release(&entry, "evafast.zip", archive);
    let cache = TempDir::new("cache");

    let PackageArchive { dir, manifest } =
        fetch_package(&entry, &fetcher, cache.path()).expect("fetch package");

    assert_eq!(manifest.name, "evafast");
    assert_eq!(manifest.version.to_string(), "1.2.3");
    assert_eq!(dir, cache.path().join("packages").join("evafast-1.2.3"));
    assert!(dir.join("package.yaml").is_file());
    assert_eq!(
        fs::read(dir.join("file.lua")).expect("extracted"),
        b"-- fixture script"
    );
    // Cache layout is exactly `<name>-<version>/`, nothing else under packages/.
    let versions: Vec<String> = fs::read_dir(cache.path().join("packages"))
        .expect("packages dir")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(versions, ["evafast-1.2.3"]);
    assert_no_work_leftovers(cache.path());
}

#[test]
fn fetch_package_uses_tags_url_for_tagged_release() {
    let entry = entry("uosc", "tomasklaen/uosc", "v6.0.0");
    let archive = zip_bytes(&[("package.yaml", manifest_yaml("uosc", "6.0.0").as_bytes())]);
    let (fetcher, _) = mock_release(&entry, "uosc.zip", archive);
    let cache = TempDir::new("cache");

    fetch_package(&entry, &fetcher, cache.path()).expect("fetch package");

    let requested = fetcher.requested();
    assert_eq!(
        requested[0],
        format!("{RELEASE_BASE}/tomasklaen/uosc/releases/tags/v6.0.0")
    );
    assert!(requested[1].ends_with("uosc.zip"), "{requested:?}");
}

#[test]
fn fetch_package_rejects_name_mismatch_without_leftovers() {
    let entry = entry("evafast", "po5/evafast", "latest");
    let archive = zip_bytes(&[(
        "package.yaml",
        manifest_yaml("other-pkg", "1.0.0").as_bytes(),
    )]);
    let (fetcher, _) = mock_release(&entry, "evafast.zip", archive);
    let cache = TempDir::new("cache");

    let err = fetch_package(&entry, &fetcher, cache.path()).expect_err("name mismatch");
    assert!(
        matches!(&err, FetchError::NameMismatch { expected, actual } if expected == "evafast" && actual == "other-pkg"),
        "{err:?}"
    );
    assert!(!cache.path().join("packages").exists(), "no package cached");
    assert_no_work_leftovers(cache.path());
}

#[test]
fn fetch_package_rejects_missing_package_yaml_without_leftovers() {
    let entry = entry("evafast", "po5/evafast", "latest");
    let archive = zip_bytes(&[("file.lua", b"-- no manifest here")]);
    let (fetcher, _) = mock_release(&entry, "evafast.zip", archive);
    let cache = TempDir::new("cache");

    let err = fetch_package(&entry, &fetcher, cache.path()).expect_err("no manifest");
    assert!(matches!(err, FetchError::MissingManifest { .. }), "{err:?}");
    assert!(!cache.path().join("packages").exists());
    assert_no_work_leftovers(cache.path());
}

#[test]
fn fetch_package_rejects_invalid_manifest() {
    let entry = entry("evafast", "po5/evafast", "latest");
    let bad = "name: evafast\nversion: not-semver\ndescription: x\nfiles:\n  - src: a\n    dest: ~~/scripts\n";
    let archive = zip_bytes(&[("package.yaml", bad.as_bytes())]);
    let (fetcher, _) = mock_release(&entry, "evafast.zip", archive);
    let cache = TempDir::new("cache");

    let err = fetch_package(&entry, &fetcher, cache.path()).expect_err("invalid manifest");
    assert!(matches!(err, FetchError::Manifest(_)), "{err:?}");
}

#[test]
fn fetch_package_rejects_release_without_archive_asset() {
    let entry = entry("evafast", "po5/evafast", "latest");
    let body = release_body(&[("evafast.lua", "https://download.example/evafast.lua")]);
    let fetcher = MockFetcher::new().with(&release_url(&entry), body.into_bytes());
    let cache = TempDir::new("cache");

    let err = fetch_package(&entry, &fetcher, cache.path()).expect_err("no archive asset");
    match err {
        FetchError::NoAsset { available, .. } => {
            assert_eq!(available, ["evafast.lua"]);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn fetch_package_rejects_missing_release() {
    let entry = entry("evafast", "po5/evafast", "latest");
    let fetcher = MockFetcher::new(); // no release configured → mock 404
    let cache = TempDir::new("cache");

    let err = fetch_package(&entry, &fetcher, cache.path()).expect_err("release 404");
    assert!(
        matches!(
            err,
            FetchError::Http {
                status: Some(404),
                ..
            }
        ),
        "{err:?}"
    );
}

#[test]
fn fetch_package_rejects_unsafe_archive_entries() {
    let entry = entry("evafast", "po5/evafast", "latest");
    let archive = zip_bytes(&[
        ("../evil.txt", b"zip-slip"),
        ("package.yaml", manifest_yaml("evafast", "1.0.0").as_bytes()),
    ]);
    let (fetcher, _) = mock_release(&entry, "evafast.zip", archive);
    let cache = TempDir::new("cache");

    let err = fetch_package(&entry, &fetcher, cache.path()).expect_err("zip-slip");
    assert!(matches!(err, FetchError::UnsafeArchive { .. }), "{err:?}");
    // Nothing escaped outside the cache.
    assert!(!cache.path().join("evil.txt").exists());
    assert_no_work_leftovers(cache.path());
}

#[test]
fn fetch_package_rejects_ambiguous_manifests() {
    let entry = entry("evafast", "po5/evafast", "latest");
    let archive = zip_bytes(&[
        ("package.yaml", manifest_yaml("evafast", "1.0.0").as_bytes()),
        (
            "sub/package.yaml",
            manifest_yaml("evafast", "1.0.0").as_bytes(),
        ),
    ]);
    let (fetcher, _) = mock_release(&entry, "evafast.zip", archive);
    let cache = TempDir::new("cache");

    let err = fetch_package(&entry, &fetcher, cache.path()).expect_err("two manifests");
    assert!(
        matches!(err, FetchError::AmbiguousManifest { .. }),
        "{err:?}"
    );
}

#[test]
fn fetch_package_prefers_zip_over_tar_gz() {
    let entry = entry("evafast", "po5/evafast", "latest");
    let zip = zip_bytes(&[("package.yaml", manifest_yaml("evafast", "1.0.0").as_bytes())]);
    let body = release_body(&[
        ("evafast.tar.gz", "https://download.example/evafast.tar.gz"),
        ("evafast.zip", "https://download.example/evafast.zip"),
    ]);
    let fetcher = MockFetcher::new()
        .with(&release_url(&entry), body.into_bytes())
        .with("https://download.example/evafast.zip", zip);
    let cache = TempDir::new("cache");

    fetch_package(&entry, &fetcher, cache.path()).expect("fetch package");
    assert_eq!(
        fetcher.requested()[1],
        "https://download.example/evafast.zip"
    );
}

#[test]
fn fetch_package_handles_tar_gz_asset() {
    let Some(tarball) = tar_gz_bytes(&[
        ("package.yaml", manifest_yaml("evafast", "2.0.0").as_bytes()),
        ("file.lua", b"-- from tarball"),
    ]) else {
        eprintln!("system tar unavailable; skipping");
        return;
    };
    let entry = entry("evafast", "po5/evafast", "latest");
    let (fetcher, _) = mock_release(&entry, "evafast.tar.gz", tarball);
    let cache = TempDir::new("cache");

    let PackageArchive { dir, manifest } =
        fetch_package(&entry, &fetcher, cache.path()).expect("fetch tarball");
    assert_eq!(manifest.version.to_string(), "2.0.0");
    assert_eq!(
        fs::read(dir.join("file.lua")).expect("extracted"),
        b"-- from tarball"
    );
}

#[test]
fn fetch_package_replaces_stale_cache() {
    let entry = entry("evafast", "po5/evafast", "latest");
    let archive = zip_bytes(&[("package.yaml", manifest_yaml("evafast", "1.2.3").as_bytes())]);
    let (fetcher, _) = mock_release(&entry, "evafast.zip", archive);
    let cache = TempDir::new("cache");
    let final_dir = cache.path().join("packages").join("evafast-1.2.3");
    fs::create_dir_all(&final_dir).expect("seed stale cache");
    fs::write(final_dir.join("stale.txt"), b"old").expect("stale marker");

    let PackageArchive { dir, .. } =
        fetch_package(&entry, &fetcher, cache.path()).expect("fetch package");

    assert_eq!(dir, final_dir);
    assert!(!dir.join("stale.txt").exists(), "stale cache replaced");
    assert!(dir.join("package.yaml").is_file());
}

#[test]
fn http_fetcher_reports_connection_failure() {
    let err = HttpFetcher
        .get("http://127.0.0.1:1/refused")
        .expect_err("connection refused");
    assert!(matches!(err, FetchError::Http { .. }), "{err:?}");
}

#[test]
#[ignore = "requires network: run manually with `cargo test -p pkg -- --ignored`"]
fn http_fetcher_real_github_release() {
    let body = HttpFetcher
        .get("https://api.github.com/repos/po5/evafast/releases/latest")
        .expect("curl against the GitHub API");
    let release: serde_json::Value =
        serde_json::from_slice(&body).expect("release payload is JSON");
    assert!(release.get("tag_name").is_some(), "release has tag_name");
    assert!(release.get("assets").is_some(), "release has assets");
    eprintln!(
        "tag_name={:?} assets={}",
        release["tag_name"],
        release["assets"].as_array().map_or(0, Vec::len)
    );
}
