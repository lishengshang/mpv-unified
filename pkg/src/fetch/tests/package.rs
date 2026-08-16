//! Package-fetch tests: release lookup, asset picking, extraction safety,
//! manifest validation, and atomic cache placement, all through
//! [`MockFetcher`] with hermetic zip/tar.gz fixtures.

use super::*;
use crate::fetch::package::{fetch_package, PackageArchive, ZipTool};

use testutil::{manifest_yaml, tar_gz_bytes, zip_bytes};

#[test]
fn zip_tool_variants_shape_their_subprocess_arguments() {
    // Info-ZIP unzip (Unix): `-Z1` lists, `-o -q <archive> -d <dest>`.
    let unzip = ZipTool::Unzip;
    assert_eq!(unzip.command(), "unzip");
    assert_eq!(unzip.list_args("a.zip"), vec!["-Z1", "a.zip"]);
    assert_eq!(
        unzip.extract_args("a.zip", "out"),
        vec!["-o", "-q", "a.zip", "-d", "out"]
    );

    // bsdtar (Windows, no unzip available): `-tf` lists, zip auto-detected.
    let tar = ZipTool::Tar;
    assert_eq!(tar.command(), "tar");
    assert_eq!(tar.list_args("a.zip"), vec!["-tf", "a.zip"]);
    assert_eq!(
        tar.extract_args("a.zip", "out"),
        vec!["-xf", "a.zip", "-C", "out"]
    );
}

#[test]
fn zip_tool_selects_tar_only_on_windows() {
    assert_eq!(
        ZipTool::select(),
        if cfg!(windows) {
            ZipTool::Tar
        } else {
            ZipTool::Unzip
        }
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
