//! `check-update` / `upgrade` subcommands (T22): update checking against the
//! remote index and the guided upgrade. The destructive upgrade requires an
//! explicit `--yes` (D17: never automatic).

use std::io::Write;

use pkg::fetch::{cache_dir, HttpFetcher};
use pkg::upgrade::{self, UpdateInfo};

/// Run `mpv-config check-update`: compare local `VERSION` with the remote
/// index and print the outcome. Exit code 1 when the check failed.
pub fn run_check_update(index_url: &str) -> i32 {
    let root = crate::doctor::check_root();
    let info = upgrade::check_update(&HttpFetcher, index_url, &root);
    print_update_info(&mut std::io::stdout(), &info);
    if info.error.is_some() {
        1
    } else {
        0
    }
}

/// Run `mpv-config upgrade [--yes]`: download → backup → overlay, user layer
/// preserved, rollback on failure. Refuses to run without `--yes`.
pub fn run_upgrade(index_url: &str, yes: bool) -> i32 {
    if !yes {
        eprintln!("错误:升级会替换 app 层文件(保留 user/ 层)。请确认后加 --yes 执行。");
        return 1;
    }
    let root = crate::doctor::check_root();
    let cache = match cache_dir() {
        Ok(cache) => cache,
        Err(error) => {
            eprintln!("错误:无法定位缓存目录:{error}");
            return 1;
        }
    };
    let result = upgrade::perform_upgrade(&HttpFetcher, index_url, &cache, &root);
    println!("{}", result.message);
    if result.applied_version.is_none() && result.message.contains("失败") {
        1
    } else {
        0
    }
}

fn print_update_info(out: &mut dyn Write, info: &UpdateInfo) {
    if let Some(error) = &info.error {
        let _ = writeln!(out, "检查更新失败:{error}");
        return;
    }
    let Some(latest) = &info.latest else {
        let _ = writeln!(out, "当前版本 {},索引未提供更新信息", info.current);
        return;
    };
    if info.has_update {
        let _ = writeln!(out, "发现新版本:{} → {latest}", info.current);
        if let Some(url) = &info.changelog_url {
            let _ = writeln!(out, "更新日志:{url}");
        }
    } else {
        let _ = writeln!(out, "当前版本 {latest} 已是最新");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> UpdateInfo {
        UpdateInfo {
            current: "0.1.0".to_owned(),
            latest: Some("0.2.0".to_owned()),
            has_update: true,
            changelog_url: Some("https://example.com/releases".to_owned()),
            error: None,
        }
    }

    fn render(info: &UpdateInfo) -> String {
        let mut out = Vec::new();
        print_update_info(&mut out, info);
        String::from_utf8(out).expect("utf8")
    }

    #[test]
    fn prints_discovered_update_with_changelog() {
        let text = render(&info());
        assert!(text.contains("0.1.0 → 0.2.0"), "{text}");
        assert!(text.contains("https://example.com/releases"), "{text}");
    }

    #[test]
    fn prints_readable_failure() {
        let text = render(&UpdateInfo {
            error: Some("网络失败".to_owned()),
            ..UpdateInfo::default()
        });
        assert!(text.contains("网络失败"), "{text}");
    }

    #[test]
    fn prints_no_update_information() {
        let text = render(&UpdateInfo {
            current: "0.1.0".to_owned(),
            latest: None,
            ..UpdateInfo::default()
        });
        assert!(text.contains("未提供更新信息"), "{text}");
    }
}
