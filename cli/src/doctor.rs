//! `doctor` subcommand: health checks for the repository layout.
//!
//! The full read-only suite — layer syntax, conditional directives,
//! platform completeness, option validity against `mpv --list-options`,
//! and a secret audit — is implemented by the [`files`], [`options`] and
//! [`secrets`] submodules and orchestrated by [`run_with`]; the exit code
//! (0 = healthy, 1 = errors, 2 = warnings only) is provided by
//! [`DoctorReport::exit_code`]. `--upgrade-check` additionally reports the
//! local `VERSION` file and points at `docs/upgrade.md`.

mod files;
mod options;
mod secrets;

use core::platform::{self, Platform};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub use options::mpv_list_options;

/// Local version read by the upgrade check.
#[derive(Debug, PartialEq, Eq)]
pub struct LocalVersion {
    /// Version string from the `VERSION` file, trimmed.
    pub version: String,
    /// Path the version was read from.
    pub path: PathBuf,
}

/// Failure of the upgrade check.
#[derive(Debug)]
pub enum UpgradeError {
    /// The `VERSION` file is missing from the checked root.
    MissingVersion { path: PathBuf },
    /// Filesystem failure while reading `VERSION`.
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for UpgradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingVersion { path } => {
                write!(f, "缺少 VERSION 文件 {}:无法确认当前版本", path.display())
            }
            Self::Io {
                action,
                path,
                source,
            } => write!(f, "{action} {} 失败:{source}", path.display()),
        }
    }
}

impl std::error::Error for UpgradeError {}

/// Root to run the check against: the working directory when it carries a
/// `VERSION` file (the released zip root), otherwise the repository root.
pub fn check_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.join("VERSION").is_file() {
        cwd
    } else {
        crate::gen::repo_root()
    }
}

/// Read the local `VERSION` file under `root`.
pub fn read_version(root: &Path) -> Result<LocalVersion, UpgradeError> {
    let path = root.join("VERSION");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Err(UpgradeError::MissingVersion { path });
        }
        Err(source) => {
            return Err(UpgradeError::Io {
                action: "读取",
                path,
                source,
            });
        }
    };
    Ok(LocalVersion {
        version: text.trim().to_owned(),
        path,
    })
}

/// Run the upgrade check: report the local version and where upgrade
/// instructions live. Returns the message the CLI prints.
pub fn upgrade_check(root: &Path) -> Result<String, UpgradeError> {
    let local = read_version(root)?;
    Ok(format!(
        "当前版本 {},升级说明见 docs/upgrade.md",
        local.version
    ))
}

/// Severity of one doctor finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// A blocking problem (exit code 1).
    Error,
    /// A non-blocking problem (exit code 2).
    Warning,
}

impl Severity {
    /// Chinese label used in the human-readable report.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Error => "错误",
            Self::Warning => "警告",
        }
    }
}

/// One problem found by a check, with its severity and message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Severity of the problem.
    pub severity: Severity,
    /// Human-readable description (layer, line, pattern, ...).
    pub message: String,
}

impl Finding {
    pub(crate) fn error(message: String) -> Self {
        Self {
            severity: Severity::Error,
            message,
        }
    }

    pub(crate) fn warning(message: String) -> Self {
        Self {
            severity: Severity::Warning,
            message,
        }
    }
}

/// Outcome of one doctor check: a summary line plus the problems found
/// (empty findings mean the check passed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    /// Check name, e.g. `语法检查`.
    pub name: &'static str,
    /// One-line outcome shown even when the check passed.
    pub summary: String,
    /// Problems found, in file order; empty when the check passed.
    pub findings: Vec<Finding>,
}

/// Full doctor report over all checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    /// Platform the checks were run against.
    pub platform: Platform,
    /// Check outcomes in report order.
    pub checks: Vec<CheckResult>,
}

impl DoctorReport {
    /// Number of error-severity findings across all checks.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.count(Severity::Error)
    }

    /// Number of warning-severity findings across all checks.
    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.count(Severity::Warning)
    }

    /// Process exit code: 0 = healthy, 1 = has errors, 2 = warnings only.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        if self.error_count() > 0 {
            1
        } else if self.warning_count() > 0 {
            2
        } else {
            0
        }
    }

    fn count(&self, severity: Severity) -> usize {
        self.checks
            .iter()
            .flat_map(|check| &check.findings)
            .filter(|finding| finding.severity == severity)
            .count()
    }
}

/// Run the full doctor suite against the repository root with the host
/// platform and `mpv` from `PATH`.
pub fn run(root: &Path) -> DoctorReport {
    run_with(root, platform::detect(), &|| {
        options::mpv_list_options("mpv")
    })
}

/// Run the full doctor suite with an explicit platform and an injectable
/// `mpv --list-options` source (tests substitute a mock).
pub fn run_with<F: Fn() -> Option<String>>(
    root: &Path,
    platform: Platform,
    options_source: &F,
) -> DoctorReport {
    DoctorReport {
        platform,
        checks: vec![
            files::syntax_check(root),
            files::cond_check(root, platform),
            files::platform_check(root, platform),
            options::check(root, platform, options_source),
            secrets::check(root),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("mpv-config-doctor-{id}"));
            fs::create_dir_all(&path).expect("create test directory");
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn read_version_returns_trimmed_version() {
        let dir = TestDir::new();
        fs::write(dir.path.join("VERSION"), "0.1.0-dev\n").expect("write VERSION");

        let local = read_version(&dir.path).expect("version read");

        assert_eq!(local.version, "0.1.0-dev");
        assert_eq!(local.path, dir.path.join("VERSION"));
    }

    #[test]
    fn read_version_handles_multiline_whitespace() {
        let dir = TestDir::new();
        fs::write(dir.path.join("VERSION"), "  1.2.3  \n").expect("write VERSION");

        assert_eq!(
            read_version(&dir.path).expect("version read").version,
            "1.2.3"
        );
    }

    #[test]
    fn missing_version_is_a_clear_error() {
        let dir = TestDir::new();

        let error = read_version(&dir.path).expect_err("missing VERSION must fail");

        assert!(error.to_string().contains("VERSION"), "{error}");
    }

    #[test]
    fn upgrade_check_reports_version_and_docs_pointer() {
        let dir = TestDir::new();
        fs::write(dir.path.join("VERSION"), "0.1.0-dev\n").expect("write VERSION");

        let message = upgrade_check(&dir.path).expect("upgrade check succeeds");

        assert!(message.contains("0.1.0-dev"), "{message}");
        assert!(message.contains("docs/upgrade.md"), "{message}");
    }
}
