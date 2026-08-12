//! `doctor` subcommand: health checks for the repository layout.
//!
//! Currently ships `--upgrade-check`: read the local `VERSION` file and point
//! the user at `docs/upgrade.md`. The full doctor suite (layer syntax, option
//! validity, secret audit) lands in a later task.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
