//! Offline tests for index fetching, index caching, and the cache-directory
//! resolution, all through [`MockFetcher`]. Package-fetch tests live in the
//! sibling [`package`] module; the single real-network test is `#[ignore]`.

mod package;

use std::fs;
use std::path::Path;
use std::sync::Mutex;

use super::*;
use crate::index::{Index, IndexError, PackageEntry};

use testutil::TempDir;

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

#[test]
fn ensure_index_url_rejects_placeholder_before_any_network_use() {
    let err = ensure_index_url(DEFAULT_INDEX_URL).expect_err("placeholder must fail");
    assert!(matches!(err, FetchError::IndexUnconfigured), "{err:?}");
    assert!(ensure_index_url("https://example.com/index.json").is_ok());
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
