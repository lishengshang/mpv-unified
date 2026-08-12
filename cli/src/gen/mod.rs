//! The `gen` command: evaluate, merge and emit the final configuration.
//!
//! Pipeline per target platform: read the layer files from the repository
//! root (`config/base.conf` → `config/{platform}.conf` → packages under
//! `config.d/packages/` → optional `user/user.conf`), evaluate each layer's
//! `#@if`/`#@else`/`#@endif` directives for the platform, merge the four
//! layers (later layer wins conflicts, every comment preserved) and
//! serialize the result as `mpv.conf`. `input.conf` is assembled the same
//! way by concatenating `config/input.conf` with the platform variant
//! `config/{platform}.input.conf`, when present. `--dry-run` reports the
//! planned files without touching the filesystem.
//!
//! Nothing here panics: every failure is a [`GenError`] with a
//! human-readable Chinese message, and the CLI maps it to exit code 1.

mod error;
mod input;
pub(crate) mod layers;
mod user;

pub use error::GenError;

use core::conf::{serialize, ConfDoc};
use core::merge::{merge_docs, MergeLayer};
use core::platform::{self, Platform};
use input::build_input;
use layers::{load_layer, load_packages, platform_name};
use std::fs;
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
}

/// Outcome of a `gen` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenReport {
    /// Platform the configuration was generated for.
    pub platform: Platform,
    /// Output files, in emission order (`mpv.conf` first, then `input.conf`).
    pub files: Vec<GeneratedFile>,
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
        .unwrap_or_else(|| {
            cli_dir
                .parent()
                .expect("cli crate sits inside the workspace")
        })
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

    let merged =
        merge_docs(&base, &platform_layer, &packages, user.as_ref()).map_err(|merge_error| {
            GenError::Parse {
                layer: merge_error.layer,
                line: merge_error.line,
                message: merge_error.message,
            }
        })?;

    let mut files = Vec::new();
    write_output(options, "mpv.conf", &serialize(&merged), &mut files)?;
    if let Some(input) = build_input(root, platform)? {
        write_output(options, "input.conf", &input, &mut files)?;
    }

    Ok(GenReport {
        platform,
        files,
        warnings,
    })
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
        fs::create_dir_all(&options.out).map_err(|source| GenError::Io {
            action: "创建输出目录",
            path: options.out.clone(),
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
    });
    Ok(())
}
