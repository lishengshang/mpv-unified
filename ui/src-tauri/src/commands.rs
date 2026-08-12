//! Tauri commands proxying the workspace crates (core profiles + cli gen).
//!
//! No shell, no subprocess: every command calls the crate functions
//! directly. Files are resolved against the repository root found by
//! [`cli::gen::repo_root`].

use mpv_core::profiles;
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

/// One profile card as the frontend sees it: the definition plus whether
/// every required package is installed (from `packages.lock`).
#[derive(Debug, Serialize)]
pub struct ProfileInfo {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub icon: String,
    pub options: Vec<String>,
    pub requires: Vec<String>,
    pub requires_met: bool,
}

/// One generated file, as reported by `regenerate`.
#[derive(Debug, Serialize)]
pub struct RegenFile {
    pub path: String,
    pub line_count: usize,
}

/// Summary of a `regenerate` run.
#[derive(Debug, Serialize)]
pub struct RegenSummary {
    pub files: Vec<RegenFile>,
    pub warnings: Vec<String>,
}

fn profiles_with_meta() -> Result<(Vec<ProfileInfo>, Vec<profiles::Profile>), String> {
    let root = cli::gen::repo_root();
    let profiles = profiles::load(&root.join("config/profiles.yaml"))
        .map_err(|error| format!("读取方案失败:{error}"))?;
    let installed: HashSet<String> = pkg::lock::read(&root.join("packages.lock"))
        .map(|lock| {
            lock.packages
                .iter()
                .map(|entry| entry.name.clone())
                .collect()
        })
        .unwrap_or_default();
    let infos = profiles
        .iter()
        .map(|profile| ProfileInfo {
            id: profile.id.clone(),
            name: profile.name.clone(),
            desc: profile.desc.clone(),
            icon: profile.icon.clone(),
            options: profile.options.clone(),
            requires: profile.requires.clone(),
            requires_met: profile.requires.iter().all(|r| installed.contains(r)),
        })
        .collect();
    Ok((infos, profiles))
}

fn user_dir(root: &Path) -> std::path::PathBuf {
    root.join("user")
}

/// All profile cards plus their dependency status.
#[tauri::command]
pub fn list_profiles() -> Result<Vec<ProfileInfo>, String> {
    profiles_with_meta().map(|(infos, _)| infos)
}

/// The enabled profile ids; defaults to `["cinema"]` on first run (state
/// file missing).
#[tauri::command]
pub fn get_profile_state() -> Result<Vec<String>, String> {
    profiles::effective_enabled(&user_dir(&cli::gen::repo_root()))
        .map_err(|error| format!("读取方案状态失败:{error}"))
}

/// Persist the enabled profile set. Enabling a profile whose required
/// packages are not installed is rejected (nothing is written).
#[tauri::command]
pub fn set_profile_state(enabled_ids: Vec<String>) -> Result<(), String> {
    let root = cli::gen::repo_root();
    let (_, profiles) = profiles_with_meta().map_err(|error| format!("读取方案失败:{error}"))?;
    let installed: HashSet<String> = pkg::lock::read(&root.join("packages.lock"))
        .map(|lock| {
            lock.packages
                .iter()
                .map(|entry| entry.name.clone())
                .collect()
        })
        .unwrap_or_default();
    for id in &enabled_ids {
        let profile = profiles
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(|| format!("未知方案:{id}"))?;
        let missing: Vec<&String> = profile
            .requires
            .iter()
            .filter(|r| !installed.contains(*r))
            .collect();
        if !missing.is_empty() {
            return Err(format!(
                "方案「{}」缺少依赖:{},请先安装再启用",
                profile.name,
                missing
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("、")
            ));
        }
    }
    profiles::write_state(&user_dir(&root), &enabled_ids)
        .map_err(|error| format!("保存方案状态失败:{error}"))
}

/// Regenerate `dist/mpv.conf` (+ `input.conf`) with the current profile
/// state, reusing the cli `gen` engine.
#[tauri::command]
pub fn regenerate() -> Result<RegenSummary, String> {
    let root = cli::gen::repo_root();
    let options = cli::gen::GenOptions {
        platform: None,
        out: root.join("dist"),
        dry_run: false,
    };
    let report = cli::gen::run(&options).map_err(|error| format!("生成失败:{error}"))?;
    Ok(RegenSummary {
        files: report
            .files
            .into_iter()
            .map(|file| RegenFile {
                path: file.path.display().to_string(),
                line_count: file.line_count,
            })
            .collect(),
        warnings: report.warnings,
    })
}

/// Whether uosc ships with the repository (scripts/uosc.lua or
/// scripts/uosc/main.lua) — drives the "uosc 联动" status on the Help page.
#[tauri::command]
pub fn uosc_status() -> Result<bool, String> {
    Ok(mpv_core::uosc::detect_installed(&cli::gen::repo_root().join("scripts")))
}

/// Every curated option of the "配置" page (from `config/options-gui.yaml`).
#[tauri::command]
pub fn list_options() -> Result<Vec<mpv_core::options_gui::GuiOption>, String> {
    let root = cli::gen::repo_root();
    let table = mpv_core::options_gui::OptionsTable::load(&root.join("config/options-gui.yaml"))
        .map_err(|error| format!("读取选项表失败:{error}"))?;
    Ok(table.options)
}

/// The currently stored `user/gui.conf` values (empty on first run).
#[tauri::command]
pub fn get_gui_values() -> Result<Vec<(String, String)>, String> {
    let root = cli::gen::repo_root();
    mpv_core::options_gui::read_gui_conf(&root.join("user"))
        .map_err(|error| format!("读取已保存配置失败:{error}"))
}

/// Persist a complete form state to `user/gui.conf`.
///
/// Only values differing from the curated defaults are written (mpv.net
/// strategy); every value is validated first, so one invalid value aborts
/// the whole write.
#[tauri::command]
pub fn save_gui_values(values: Vec<(String, String)>) -> Result<(), String> {
    let root = cli::gen::repo_root();
    let table = mpv_core::options_gui::OptionsTable::load(&root.join("config/options-gui.yaml"))
        .map_err(|error| format!("读取选项表失败:{error}"))?;
    mpv_core::options_gui::write_gui_conf(&table, &root.join("user"), &values)
        .map_err(|error| format!("保存失败:{error}"))
}

/// Reset one option to its curated default (removes its line, if any).
#[tauri::command]
pub fn reset_gui_value(key: String) -> Result<(), String> {
    let root = cli::gen::repo_root();
    let user_dir = root.join("user");
    let table = mpv_core::options_gui::OptionsTable::load(&root.join("config/options-gui.yaml"))
        .map_err(|error| format!("读取选项表失败:{error}"))?;
    table
        .find(&key)
        .ok_or_else(|| format!("未知选项:{key}"))?;
    let values: Vec<(String, String)> = mpv_core::options_gui::read_gui_conf(&user_dir)
        .map_err(|error| format!("读取已保存配置失败:{error}"))?
        .into_iter()
        .filter(|(k, _)| *k != key)
        .collect();
    mpv_core::options_gui::write_gui_conf(&table, &user_dir, &values)
        .map_err(|error| format!("重置失败:{error}"))
}

/// Check the remote index for a newer version (T22). The result never
/// fails; every failure mode is carried in `UpdateInfo.error`.
#[tauri::command]
pub fn check_update() -> Result<pkg::upgrade::UpdateInfo, String> {
    let root = cli::gen::repo_root();
    Ok(pkg::upgrade::check_update(
        &pkg::fetch::HttpFetcher,
        pkg::fetch::DEFAULT_INDEX_URL,
        &root,
    ))
}

/// Execute the guided upgrade (T22): download → backup app layer → overlay
/// (user layer preserved) with automatic rollback on failure. Confirmation
/// is the frontend's job (explicit dialog before invoking this command).
#[tauri::command]
pub fn perform_upgrade() -> Result<pkg::upgrade::UpgradeResult, String> {
    let root = cli::gen::repo_root();
    let cache = pkg::fetch::cache_dir().map_err(|error| format!("无法定位缓存目录:{error}"))?;
    Ok(pkg::upgrade::perform_upgrade(
        &pkg::fetch::HttpFetcher,
        pkg::fetch::DEFAULT_INDEX_URL,
        &cache,
        &root,
    ))
}
