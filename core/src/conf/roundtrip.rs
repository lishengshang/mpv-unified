//! Round-trip acceptance tests: `parse` -> `serialize` must reproduce the
//! input byte for byte, including the trailing-newline state.

use super::*;
use std::fs;
use std::path::{Path, PathBuf};

fn assert_roundtrip(input: &str) {
    let doc = parse(input).unwrap_or_else(|e| panic!("parse failed for {input:?}: {e}"));
    assert_eq!(serialize(&doc), input);
}

#[test]
fn synthetic_corpus_roundtrips() {
    let cases = [
        "",
        "\n",
        "\n\n\n",
        "# only a comment",
        "key=value",
        "key=value\n",
        "#\n\n\n[profile]\n key=v # c\n   \nflag\n",
        "vo=gpu-next                           # <gpu/gpu-next/libmpv> 视频输出驱动\n\
                                              ## gpu 是旧版渲染后端，已缺失很多新特性\n",
        "title=${?pause==yes:⏸}${?mute==yes:🔇}${?ontop==yes:📌}\n",
        " profile-cond=path:find('://') ~= nil or path:find('^magnet:') ~= nil\n",
        "icc-profile=\"\"\nosd-color=\"#ef14d5\"\n",
        "#@if platform==windows\ngpu-api=d3d11\n#@else\ngpu-api=vulkan\n#@endif",
        "no-trailing-newline",
        "  \n\t\n \t \n",
        "key=value\r\nwindows-line\r\n",
        "a=b\nc=d",
        "#[image]\n# profile-cond=vid and not get(\"current-tracks/video/albumart\")\n",
        "[end]\n profile-desc=播放列表结束后退出全屏\n no-fullscreen",
    ];
    for case in cases {
        assert_roundtrip(case);
    }
    assert_roundtrip(&format!("{}\n", "x".repeat(5000)));
    assert_roundtrip(&format!("long={}\nno-newline-end", "值".repeat(1000)));
}

fn repo_mpv_conf_path() -> PathBuf {
    // Task 7 moved the 110KB Windows original into archive/mpv.conf.orig and
    // rebuilt it as four-layer sources under config/; the round-trip contract
    // now guards every layer file (they are what the generator consumes).
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../config/base.conf")
}

fn layer_mpv_conf_paths() -> Vec<(String, PathBuf)> {
    // mpv.conf layers only. input.conf is not part of the Rust round-trip
    // contract: it binds the bare `[` key, which the mpv.conf parser treats
    // as a profile header. input.conf equivalence is verified by
    // tools/verify-equivalence.sh instead.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    [
        ("base.conf", "config/base.conf"),
        ("windows.conf", "config/windows.conf"),
        ("linux.conf", "config/linux.conf"),
    ]
    .into_iter()
    .map(|(name, rel)| (name.to_owned(), root.join(rel)))
    .collect()
}

fn live_mpv_conf_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/mpv/mpv.conf"))
}

fn assert_real_file_roundtrip(path: &Path) {
    let original = fs::read(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let text = String::from_utf8(original.clone())
        .unwrap_or_else(|_| panic!("{} is not valid UTF-8", path.display()));
    let doc = parse(&text).unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()));
    assert!(
        !doc.entries.is_empty(),
        "{} parsed to an empty doc",
        path.display()
    );
    let serialized = serialize(&doc);
    assert_eq!(
        serialized.as_bytes(),
        original.as_slice(),
        "round-trip must be byte-identical for {}",
        path.display()
    );
}

/// Every config-layer source file shipped in `config/` must round-trip
/// byte-identically. The 110KB Windows original moved to archive/mpv.conf.orig
/// in task 7 and is verified semantically by tools/verify-equivalence.sh.
#[test]
fn real_repo_mpv_conf_roundtrip() {
    for (name, path) in layer_mpv_conf_paths() {
        eprintln!("roundtrip: {name}");
        assert_real_file_roundtrip(&path);
    }
}

/// The live 346-line Linux config. Absent on CI runners, so the test skips
/// when the file does not exist instead of failing.
#[test]
fn real_live_mpv_conf_roundtrip() {
    let Some(path) = live_mpv_conf_path() else {
        return;
    };
    if !path.exists() {
        eprintln!("skipping: {} not present on this machine", path.display());
        return;
    }
    assert_real_file_roundtrip(&path);
}

#[test]
fn real_repo_mpv_conf_structure_spot_checks() {
    let base_text = fs::read_to_string(repo_mpv_conf_path()).unwrap();
    let base = parse(&base_text).unwrap();
    assert!(
        base.entries.len() > 500,
        "config/base.conf should exceed 500 lines"
    );
    assert!(
        base.ends_with_newline,
        "config/base.conf keeps the original's trailing newline state"
    );
    assert!(
        matches!(&base.entries[0], Entry::Comment { text } if text.starts_with("##⇘⇘")),
        "first line must stay a comment"
    );
    assert!(
        !base
            .entries
            .iter()
            .any(|e| matches!(e, Entry::ProfileStart { .. })),
        "config/base.conf must be top-level only (profiles live in windows.conf)"
    );
    assert!(
        base.entries.iter().any(|e| matches!(
            e,
            Entry::KeyValue { key, value, .. } if key == "vo" && value == "gpu-next"
        )),
        "top-level vo=gpu-next must be recognized"
    );

    // The 41 profile blocks moved to windows.conf (task 7 layout).
    let win_text =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../config/windows.conf"))
            .unwrap();
    let win = parse(&win_text).unwrap();
    assert!(
        win.entries
            .iter()
            .any(|e| matches!(e, Entry::ProfileStart { name, .. } if name == "ICC")),
        "[ICC] profile header must be recognized"
    );
    assert!(
        win.entries
            .iter()
            .any(|e| matches!(e, Entry::ProfileStart { name, .. } if name == "end")),
        "[end] profile header must be recognized"
    );
    assert!(
        win.entries.iter().any(|e| matches!(
            e,
            Entry::KeyValue { key, value, .. }
            if key == "profile-cond" && value.contains("path:find('://')")
        )),
        "indented profile-cond line with apostrophes must parse as key/value"
    );
    assert!(
        win.entries
            .iter()
            .any(|e| matches!(e, Entry::KeyValue { key, value, .. } if key == "icc-profile" && value == "\"\"")),
        "empty quoted value must survive"
    );
}

/// Only runs when `MPV_CONF_ROUNDTRIP_DUMP` points at a directory; writes the
/// serialized bytes of every config layer there so `sha256sum` can compare
/// them against the sources (evidence collection).
#[test]
fn dump_serialized_for_sha256_evidence() {
    let Some(dir) = std::env::var_os("MPV_CONF_ROUNDTRIP_DUMP") else {
        return;
    };
    let dir = PathBuf::from(dir);
    fs::create_dir_all(&dir).unwrap();
    let mut sources: Vec<(String, PathBuf)> = vec![("base.conf".to_owned(), repo_mpv_conf_path())];
    sources.extend(layer_mpv_conf_paths());
    if let Some(live) = live_mpv_conf_path() {
        if live.exists() {
            sources.push(("live-mpv.conf".to_string(), live));
        }
    }
    for (name, path) in sources {
        let text = fs::read_to_string(&path).unwrap();
        let serialized = serialize(&parse(&text).unwrap());
        fs::write(dir.join(&name), serialized).unwrap();
        println!("dumped {name} from {}", path.display());
    }
}
