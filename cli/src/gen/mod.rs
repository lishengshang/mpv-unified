//! The `gen` command: evaluate, merge and emit the final configuration.
//!
//! Pipeline per target platform: read the layer files from the repository
//! root (`config/base.conf` → `config/{platform}.conf` → packages under
//! `config.d/packages/` → optional `user/user.conf` → optional
//! `user/gui.conf`), evaluate each layer's `#@if`/`#@else`/`#@endif`
//! directives for the platform, merge the layers (later layer wins
//! conflicts, every comment preserved) and serialize the result as
//! `mpv.conf`. `gui.conf` is the GUI-managed fragment written by the "配置"
//! page (only non-default values), so it sits above the manual `user.conf`.
//! `input.conf` is assembled the same way by concatenating
//! `config/input.conf` with the platform variant `config/{platform}.input.conf`,
//! when present. `profiles.conf` at the repository root is copied verbatim
//! (the include target of `include="~~/profiles.conf"` must sit next to
//! `mpv.conf`), the asset directories (`scripts/`, `shaders/`, `fonts/`,
//! `script-opts/`, `icc/`, `osc-style/`, `vs/`, `script-modules/`) are
//! copied verbatim so the output works as `mpv --config-dir`, and the
//! runtime `files/` directory is created for
//! `log-file="~~/files/mpv.log"`. `--dry-run` reports the planned files
//! without touching the filesystem.
//!
//! Nothing here panics: every failure is a [`GenError`] with a
//! human-readable Chinese message, and the CLI maps it to exit code 1.

mod assets;
mod error;
mod input;
pub(crate) mod layers;
mod user;

pub use assets::CopiedAssetDir;
pub use error::GenError;

use core::conf::{serialize, ConfDoc};
use core::merge::{merge, merge_docs, MergeLayer};
use core::platform::{self, Platform};
use input::build_input;
use layers::{load_layer, load_packages, platform_name};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Options for a single `gen` run.
#[derive(Debug, Clone)]
pub struct GenOptions {
    /// Target platform; `None` means auto-detect the host.
    pub platform: Option<Platform>,
    /// Output directory; created (recursively) when missing.
    pub out: PathBuf,
    /// Report planned outputs without writing anything.
    pub dry_run: bool,
}

/// One file the run would (or did) produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    /// Full output path of the file.
    pub path: PathBuf,
    /// Number of lines in the emitted content.
    pub line_count: usize,
    /// True when the file was copied verbatim from the repository root
    /// (`profiles.conf`) instead of generated from layers.
    pub copied: bool,
}

/// Outcome of a `gen` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenReport {
    /// Platform the configuration was generated for.
    pub platform: Platform,
    /// Output files, in emission order (`mpv.conf` first, then `input.conf`,
    /// then a copied `profiles.conf` when present).
    pub files: Vec<GeneratedFile>,
    /// Asset directories copied from the repository root (`scripts/`, ...),
    /// with per-directory file counts; empty when none exists.
    pub asset_dirs: Vec<CopiedAssetDir>,
    /// Non-fatal problems (e.g. a skipped optional layer), in order.
    pub warnings: Vec<String>,
}

/// Run `gen` against the repository root (the workspace root that contains
/// `config/`).
pub fn run(options: &GenOptions) -> Result<GenReport, GenError> {
    run_at_root(&repo_root(), options)
}

/// Resolve the repository root holding `config/`, `config.d/` and `user/`.
///
/// `CARGO_MANIFEST_DIR` points at the `cli` crate (`<repo>/cli`); the
/// repository root is its parent, one level up. Up to two ancestor levels
/// are probed for `config/base.conf` (tolerating a deeper nesting), and the
/// workspace root is the fallback when neither has one.
pub fn repo_root() -> PathBuf {
    let cli_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut ancestors = Vec::new();
    let mut dir = cli_dir.as_path();
    while let Some(parent) = dir.parent() {
        ancestors.push(parent);
        dir = parent;
        if ancestors.len() == 2 {
            break;
        }
    }
    ancestors
        .into_iter()
        .find(|dir| dir.join("config").join("base.conf").is_file())
        .or_else(|| cli_dir.parent())
        .unwrap_or(cli_dir.as_path())
        .to_path_buf()
}

/// Run `gen` with an explicit repository root (used by tests with fixture
/// directories).
pub fn run_at_root(root: &Path, options: &GenOptions) -> Result<GenReport, GenError> {
    let platform = options.platform.unwrap_or_else(platform::detect);
    let mut warnings = Vec::new();
    let platform_file = format!("{}.conf", platform_name(platform));

    let base = match load_layer(root, "config/base.conf", platform, true)? {
        Some(layer) => layer,
        // Unreachable: a required layer yields Err, not None.
        None => {
            return Err(GenError::MissingLayer {
                path: root.join("config/base.conf"),
            });
        }
    };

    // The macOS layer is experimental (no source material): a missing
    // macos.conf is a warning, a missing linux/windows layer is an error.
    let platform_layer = match load_layer(
        root,
        &format!("config/{platform_file}"),
        platform,
        platform != Platform::MacOS,
    )? {
        Some(layer) => layer,
        None => {
            warnings.push(format!("macOS 平台层(config/{platform_file})缺失,已跳过"));
            MergeLayer::new(
                platform_file.clone(),
                ConfDoc {
                    entries: Vec::new(),
                    ends_with_newline: true,
                },
            )
        }
    };

    let packages = load_packages(root, platform)?;

    if !options.dry_run {
        if let Some(notice) = user::ensure_user_layer(root)? {
            warnings.push(notice);
        }
    }

    let user = load_layer(root, "user/user.conf", platform, false)?;
    let gui = load_layer(root, "user/gui.conf", platform, false)?;

    let merged =
        merge_docs(&base, &platform_layer, &packages, user.as_ref()).map_err(|merge_error| {
            GenError::Parse {
                layer: merge_error.layer,
                line: merge_error.line,
                message: merge_error.message,
            }
        })?;

    // gui.conf(GUI「配置」页管理的独立片段)优先级高于 user.conf:存在时作为
    // 最后一个层叠加(user.conf 之后),同名键由 gui 层覆盖。手动编辑的
    // user.conf 内容原样保留;缺失时静默跳过。
    let merged = match gui.as_ref() {
        Some(gui) => merge(vec![MergeLayer::new("user/gui.conf", merged), gui.clone()]).map_err(
            |merge_error| GenError::Parse {
                layer: merge_error.layer,
                line: merge_error.line,
                message: merge_error.message,
            },
        )?,
        None => merged,
    };

    // 方案块追加(任务 18):启用方案 → 生成 mpv profile 块,追加到 mpv.conf
    // 末尾。profiles.yaml 缺失时静默跳过;损坏或状态读取失败时警告并跳过
    // (方案是可选增强,不阻断核心生成流程)。
    let mut mpv_text = serialize(&merged);
    let mut files = Vec::new();
    // 资产拷贝先于一切生成写入:script-opts/ 原文件先落到输出目录,后续
    // uosc 补丁在其上追加,保持 T23 语义(生成的 uosc.conf 含方案菜单补丁)。
    let asset_dirs = assets::copy_assets(root, options)?;
    // uosc 菜单补丁(任务 23):启用方案 ≥1 时,若检测到 uosc,生成
    // dist/script-opts/uosc.conf 补丁并把菜单项追加到 input.conf。
    let mut uosc_menu_lines: Option<String> = None;
    if let Some((profiles, enabled_ids)) = profile_blocks(root, &mut warnings)? {
        if !enabled_ids.is_empty() {
            let blocks = core::profiles::generate_profile_blocks(&profiles, &enabled_ids);
            if !blocks.is_empty() {
                if !mpv_text.ends_with('\n') {
                    mpv_text.push('\n');
                }
                mpv_text.push_str(&blocks);
                uosc_menu_lines = uosc_patch(
                    root,
                    options,
                    &profiles,
                    &enabled_ids,
                    &mut warnings,
                    &mut files,
                )?;
            }
        }
    }

    write_output(options, "mpv.conf", &mpv_text, &mut files)?;
    if let Some(menu_lines) = uosc_menu_lines {
        let mut text = build_input(root, platform)?.unwrap_or_default();
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str("# ===== uosc 方案切换菜单(由 mpv-config 生成,任务 23) =====\n");
        text.push_str(&menu_lines);
        write_output(options, "input.conf", &text, &mut files)?;
    } else if let Some(input) = build_input(root, platform)? {
        write_output(options, "input.conf", &input, &mut files)?;
    }

    copy_profiles_conf(root, options, &mut files)?;
    ensure_files_dir(options);

    Ok(GenReport {
        platform,
        files,
        asset_dirs,
        warnings,
    })
}

/// Copy the repository-root `profiles.conf` next to `mpv.conf` when present.
///
/// `config/{platform}.conf` may carry `include="~~/profiles.conf"`, and mpv's
/// `~~/` resolves to the output directory — the include target must exist
/// there or mpv fails with `Cannot open file .../profiles.conf`. A missing
/// source is skipped silently (it is an optional asset, like in
/// `tools/build-dist.sh`).
fn copy_profiles_conf(
    root: &Path,
    options: &GenOptions,
    files: &mut Vec<GeneratedFile>,
) -> Result<(), GenError> {
    let source = root.join("profiles.conf");
    if !source.is_file() {
        return Ok(());
    }
    let target = options.out.join("profiles.conf");
    if !options.dry_run {
        fs::copy(&source, &target).map_err(|source_error| GenError::Io {
            action: "复制",
            path: target.clone(),
            source: source_error,
        })?;
    }
    let line_count = fs::read_to_string(&source)
        .map(|text| text.lines().count())
        .unwrap_or(0);
    files.push(GeneratedFile {
        path: target,
        line_count,
        copied: true,
    });
    Ok(())
}

/// Create the runtime `files/` directory in the output (the `log-file` target
/// of `base.conf`), best-effort: an unwritable output is already reported by
/// the file writes, so a failure here is ignored rather than blocking
/// generation.
fn ensure_files_dir(options: &GenOptions) {
    if !options.dry_run {
        let _ = fs::create_dir_all(options.out.join("files"));
    }
}

/// The loaded profile definitions plus the enabled ids, as returned by
/// [`profile_blocks`].
type ProfileBundle = (Vec<core::profiles::Profile>, Vec<String>);

/// Load the profile definitions and the enabled set, when profile rendering
/// is possible at all.
///
/// Reads `config/profiles.yaml` plus the enabled set from
/// `user/profiles-state.json` (first run defaults to `[cinema]`, via
/// [`core::profiles::effective_enabled`]). A missing profiles file yields
/// `None` silently; a broken file or unreadable state yields a warning plus
/// `None` — the optional enhancement never blocks generation.
fn profile_blocks(
    root: &Path,
    warnings: &mut Vec<String>,
) -> Result<Option<ProfileBundle>, GenError> {
    let yaml_path = root.join("config/profiles.yaml");
    let raw = match fs::read_to_string(&yaml_path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(GenError::Io {
                action: "读取",
                path: yaml_path,
                source,
            });
        }
    };
    let profiles = match core::profiles::parse_yaml(&raw) {
        Ok(profiles) => profiles,
        Err(error) => {
            warnings.push(format!(
                "config/profiles.yaml 解析失败,已跳过方案块:{error}"
            ));
            return Ok(None);
        }
    };
    let enabled = match core::profiles::effective_enabled(&root.join("user")) {
        Ok(enabled) => enabled,
        Err(error) => {
            warnings.push(format!("读取方案启用状态失败,已跳过方案块:{error}"));
            return Ok(None);
        }
    };
    Ok(Some((profiles, enabled)))
}

/// Emit the uosc menu patch when uosc is installed: writes
/// `dist/script-opts/uosc.conf` (the repository's own `uosc.conf`, when
/// present, with the patch appended — never clobbered) and returns the
/// operative menu lines for `input.conf`. When uosc is absent, warns and
/// returns `None` — the integration degrades gracefully, never errors.
fn uosc_patch(
    root: &Path,
    options: &GenOptions,
    profiles: &[core::profiles::Profile],
    enabled_ids: &[String],
    warnings: &mut Vec<String>,
    files: &mut Vec<GeneratedFile>,
) -> Result<Option<String>, GenError> {
    if !core::uosc::detect_installed(&root.join("scripts")) {
        warnings.push(
            "未检测到 uosc(scripts/uosc.lua 或 scripts/uosc/main.lua 不存在),已跳过 uosc 方案切换菜单;安装 uosc 后重新生成即可。".to_owned(),
        );
        return Ok(None);
    }
    let patch = core::uosc::generate_uosc_conf(profiles, enabled_ids);
    let mut body = String::new();
    match fs::read_to_string(root.join("script-opts/uosc.conf")) {
        Ok(existing) => {
            body.push_str(&existing);
            if !body.ends_with('\n') {
                body.push('\n');
            }
            body.push('\n');
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => {}
        Err(source) => {
            warnings.push(format!(
                "读取 script-opts/uosc.conf 失败,仅输出补丁内容:{source}"
            ));
        }
    }
    body.push_str(&patch);
    write_output(options, "script-opts/uosc.conf", &body, files)?;
    Ok(Some(core::uosc::generate_menu_lines(profiles, enabled_ids)))
}

/// Write (or, in dry-run mode, only plan) one output file into `options.out`.
fn write_output(
    options: &GenOptions,
    name: &str,
    text: &str,
    files: &mut Vec<GeneratedFile>,
) -> Result<(), GenError> {
    let path = options.out.join(name);
    if !options.dry_run {
        // `out.join(name)` always has a parent (every output name is a
        // non-empty literal); the unreachable miss is reported as an error
        // instead of panicking.
        let parent = path.parent().ok_or_else(|| GenError::Io {
            action: "创建输出目录",
            path: options.out.clone(),
            source: io::Error::new(io::ErrorKind::InvalidInput, "输出路径缺少父目录"),
        })?;
        fs::create_dir_all(parent).map_err(|source| GenError::Io {
            action: "创建输出目录",
            path: parent.to_path_buf(),
            source,
        })?;
        fs::write(&path, text).map_err(|source| GenError::Io {
            action: "写入",
            path: path.clone(),
            source,
        })?;
    }
    files.push(GeneratedFile {
        path,
        line_count: text.lines().count(),
        copied: false,
    });
    Ok(())
}
