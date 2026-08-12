//! uosc integration: turn enabled profiles into uosc context-menu items.
//!
//! uosc (this repository vendors uosc 5.x under `scripts/uosc/`) builds its
//! right-click context menu from `input.conf` lines whose key is `#` and
//! whose command carries a `#!` / `#menu:` comment — see
//! `scripts/uosc/README.md` → "Adding items to menu". Enabled profiles
//! become one menu item each, an *unbound* binding running the native
//! `apply-profile <id>` command (mpv ≥ 0.32; the repository declares a
//! minimum of 0.36), grouped under a「方案」submenu so the profile can be
//! switched from the menu while playing:
//!
//! ```text
//! #  apply-profile cinema  #menu: 方案 > 高清观影
//! ```
//!
//! The gen pipeline (task 23) uses this module twice: [`generate_menu_lines`]
//! supplies the operative lines appended to the generated `dist/input.conf`,
//! and [`generate_uosc_conf`] renders the self-documenting patch fragment
//! appended to `script-opts/uosc.conf`. [`detect_installed`] probes the
//! scripts directory so the pipeline can degrade to a warning ("未检测到
//! uosc") instead of an error when uosc is absent. Nothing here panics and
//! nothing touches uosc's own scripts — only generated configuration.

use crate::profiles::Profile;
use std::path::Path;

/// uosc context-menu folder holding the generated profile items.
pub const MENU_GROUP: &str = "方案";

/// True when uosc ships with the repository: the single-file layout
/// `scripts/uosc.lua` or the uosc 5.x folder layout `scripts/uosc/main.lua`.
/// A stray `script-opts/uosc.conf` alone does not count — a config without
/// the script does nothing.
#[must_use]
pub fn detect_installed(scripts_dir: &Path) -> bool {
    scripts_dir.join("uosc.lua").is_file() || scripts_dir.join("uosc/main.lua").is_file()
}

/// The operative menu lines to append to the generated `input.conf`: one
/// unbound `apply-profile <id>` binding per enabled profile, grouped under
/// the「方案」menu folder and written in the same `#menu:` syntax the
/// repository already uses in `config/input.conf`. Unknown ids are skipped
/// silently; no matching profile yields an empty string.
#[must_use]
pub fn generate_menu_lines(profiles: &[Profile], enabled_ids: &[String]) -> String {
    let mut text = String::new();
    for profile in profiles {
        if !enabled_ids.iter().any(|id| id == &profile.id) {
            continue;
        }
        text.push_str(&format!(
            "#  apply-profile {}  #menu: {} > {}\n",
            profile.id, MENU_GROUP, profile.name
        ));
    }
    text
}

/// The uosc.conf patch fragment: an explanatory header (which also serves
/// as documentation for manual installs) followed by the operative menu
/// lines from [`generate_menu_lines`]. The fragment is safe to append to an
/// existing `script-opts/uosc.conf` — every line is a comment, so uosc
/// ignores it entirely; the menu itself lives in `input.conf`.
#[must_use]
pub fn generate_uosc_conf(profiles: &[Profile], enabled_ids: &[String]) -> String {
    let mut text = String::new();
    text.push_str("# ============================================================\n");
    text.push_str("# uosc 方案切换菜单补丁(由 mpv-config 自动生成,任务 23)\n");
    text.push_str("# 原理:uosc 5.x 的右键菜单由 input.conf 中以 # 为键、带\n");
    text.push_str("#       #menu:/#! 注释的绑定构成。启用方案后,每个启用\n");
    text.push_str("#       方案生成一个 apply-profile <id> 菜单项(无按键\n");
    text.push_str("#       绑定),归入「方案」子菜单,播放中可随时切换。\n");
    text.push_str("# 生效:菜单项行已由 gen 追加到生成的 dist/input.conf;\n");
    text.push_str("#       本文件仅供查看,或手工合并到 script-opts/uosc.conf。\n");
    text.push_str("# 要求:mpv >= 0.36(apply-profile 命令),uosc 5.x。\n");
    text.push_str("# 删除:停用全部方案后重新生成即可移除菜单项。\n");
    text.push_str("# ============================================================\n");
    let menu = generate_menu_lines(profiles, enabled_ids);
    if menu.is_empty() {
        text.push_str("# 无启用方案,未生成菜单项。\n");
    } else {
        text.push_str(&menu);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::parse_yaml;

    const VALID: &str = "\
profiles:
  - id: cinema
    name: 高清观影
    desc: 画质优先
    icon: 🎬
    options:
      - \"deband=yes\"
    requires: []
  - id: music
    name: 音乐模式
    desc: 纯音频
    icon: 🎧
    options:
      - \"vo=null\"
    requires: []
  - id: game
    name: 低延迟游戏
    desc: 延迟优先
    icon: 🎮
    options:
      - \"scale=bilinear\"
    requires: []
";

    fn profiles() -> Vec<Profile> {
        parse_yaml(VALID).expect("fixture parses")
    }

    fn ids(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    fn scripts_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("mpv-config-uosc-{label}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn no_enabled_profiles_yield_no_menu_lines() {
        let profiles = profiles();
        assert_eq!(generate_menu_lines(&profiles, &[]), "");
        assert_eq!(
            generate_menu_lines(&profiles, &ids(&["ghost"])),
            "",
            "unknown ids are skipped silently"
        );
        let patch = generate_uosc_conf(&profiles, &[]);
        assert!(patch.contains("无启用方案"), "{patch}");
        assert!(
            !patch.lines().any(|l| l.starts_with("#  apply-profile")),
            "no menu items: {patch}"
        );
    }

    #[test]
    fn single_enabled_profile_emits_apply_profile_menu_item() {
        let profiles = profiles();
        let menu = generate_menu_lines(&profiles, &ids(&["cinema"]));
        assert_eq!(menu, "#  apply-profile cinema  #menu: 方案 > 高清观影\n");
        let patch = generate_uosc_conf(&profiles, &ids(&["cinema"]));
        assert!(patch.contains("uosc 方案切换菜单补丁"), "{patch}");
        assert!(patch.contains("apply-profile cinema"), "{patch}");
        assert!(patch.contains("#menu: 方案 > 高清观影"), "{patch}");
    }

    #[test]
    fn multiple_enabled_profiles_follow_definition_order() {
        let profiles = profiles();
        let menu = generate_menu_lines(&profiles, &ids(&["game", "cinema", "music"]));
        let cinema = menu.find("cinema").expect("cinema line present");
        let music = menu.find("music").expect("music line present");
        let game = menu.find("game").expect("game line present");
        assert!(
            cinema < music && music < game,
            "definition order preserved: {menu}"
        );
        let menu_items = |text: &str| {
            text.lines()
                .filter(|l| l.starts_with("#  apply-profile"))
                .count()
        };
        assert_eq!(menu_items(&menu), 3, "{menu}");
        let patch = generate_uosc_conf(&profiles, &ids(&["cinema", "game"]));
        assert_eq!(menu_items(&patch), 2, "{patch}");
    }

    #[test]
    fn detect_installed_matches_file_and_folder_layouts() {
        let dir = scripts_dir("detect");
        assert!(!detect_installed(&dir), "empty dir is not installed");
        std::fs::create_dir_all(dir.join("uosc")).expect("create folder");
        assert!(
            !detect_installed(&dir),
            "bare uosc/ folder without main.lua is not installed"
        );
        std::fs::write(dir.join("uosc/main.lua"), "-- uosc").expect("write main.lua");
        assert!(detect_installed(&dir), "uosc 5.x folder layout detected");
        std::fs::write(dir.join("uosc.lua"), "-- uosc").expect("write uosc.lua");
        assert!(detect_installed(&dir), "single-file layout detected");
        // A stray uosc.conf without the script must not count.
        let conf_only = scripts_dir("conf-only");
        std::fs::create_dir_all(&conf_only).expect("create dir");
        std::fs::write(conf_only.join("uosc.conf"), "timeline_style=bar\n")
            .expect("write stray conf");
        assert!(
            !detect_installed(&conf_only),
            "conf without script is not installed"
        );
    }
}
