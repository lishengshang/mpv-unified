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
