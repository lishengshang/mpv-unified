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
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mpv.conf")
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

/// The 110KB Windows-original config shipped in the repo root (1042 lines,
/// `##` documentation blocks, indented profile bodies, no trailing newline).
#[test]
fn real_repo_mpv_conf_roundtrip() {
    assert_real_file_roundtrip(&repo_mpv_conf_path());
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
    let text = fs::read_to_string(repo_mpv_conf_path()).unwrap();
    let doc = parse(&text).unwrap();
    assert!(
        doc.entries.len() > 1000,
        "repo config should exceed 1000 lines"
    );
    assert!(
        !doc.ends_with_newline,
        "repo config is known to lack a trailing newline"
    );
    assert!(
        matches!(&doc.entries[0], Entry::Comment { text } if text.starts_with("##⇘⇘")),
        "first line must stay a comment"
    );
    assert!(
        doc.entries
            .iter()
            .any(|e| matches!(e, Entry::ProfileStart { name, .. } if name == "ICC")),
        "[ICC] profile header must be recognized"
    );
    assert!(
        doc.entries
            .iter()
            .any(|e| matches!(e, Entry::ProfileStart { name, .. } if name == "end")),
        "[end] profile header must be recognized"
    );
    assert!(
        doc.entries.iter().any(|e| matches!(
            e,
            Entry::KeyValue { key, value, .. }
            if key == "profile-cond" && value.contains("path:find('://')")
        )),
        "indented profile-cond line with apostrophes must parse as key/value"
    );
    assert!(
        doc.entries
            .iter()
            .any(|e| matches!(e, Entry::KeyValue { key, value, .. } if key == "icc-profile" && value == "\"\"")),
        "empty quoted value must survive"
    );
}

/// Only runs when `MPV_CONF_ROUNDTRIP_DUMP` points at a directory; writes the
/// serialized bytes of both real files there so `sha256sum` can compare them
/// against the originals (evidence collection).
#[test]
fn dump_serialized_for_sha256_evidence() {
    let Some(dir) = std::env::var_os("MPV_CONF_ROUNDTRIP_DUMP") else {
        return;
    };
    let dir = PathBuf::from(dir);
    fs::create_dir_all(&dir).unwrap();
    let mut sources = vec![("repo-mpv.conf", repo_mpv_conf_path())];
    if let Some(live) = live_mpv_conf_path() {
        if live.exists() {
            sources.push(("live-mpv.conf", live));
        }
    }
    for (name, path) in sources {
        let text = fs::read_to_string(&path).unwrap();
        let serialized = serialize(&parse(&text).unwrap());
        fs::write(dir.join(name), serialized).unwrap();
        println!("dumped {name} from {}", path.display());
    }
}
