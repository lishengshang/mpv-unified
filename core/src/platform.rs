//! Platform detection and mpv config-directory resolution.
//!
//! [`detect`] identifies the host platform, [`config_root`] resolves the
//! directory mpv reads its configuration from, and [`mpv_available`] probes
//! `PATH` for the mpv executable without running it.
//!
//! Resolution follows the mpv manual (FILES section):
//! - `MPV_HOME` completely overrides the config directory on every platform
//!   (used verbatim; existence is deliberately not checked);
//! - Windows: `portable_config/` next to `mpv.exe` (portable mode), else
//!   `%APPDATA%\mpv`;
//! - Linux/macOS: `$XDG_CONFIG_HOME/mpv`, else `~/.config/mpv`.
//!
//! Nothing in this module panics: an unresolvable environment returns
//! [`ConfigRootError`].

use std::fmt;
use std::path::PathBuf;
use std::process::Command;

/// A platform this tool can generate configuration for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Linux,
    Windows,
    MacOS,
}

impl Platform {
    /// Human-readable name: `"Linux"`, `"Windows"`, or `"macOS"`.
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Linux => "Linux",
            Self::Windows => "Windows",
            Self::MacOS => "macOS",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

/// Detect the host platform from [`std::env::consts::OS`].
///
/// Unknown operating systems degrade to [`Platform::Linux`]: mpv's Unix
/// config layout (`~/.config/mpv`) is the closest match there.
#[must_use]
pub fn detect() -> Platform {
    from_os(std::env::consts::OS)
}

/// Resolve the directory mpv reads its configuration from on the host.
///
/// See the [module docs](self) for the platform-specific priority rules.
///
/// # Errors
///
/// Returns [`ConfigRootError`] when no base directory can be resolved
/// (e.g. neither `XDG_CONFIG_HOME` nor `HOME` is set on Unix).
pub fn config_root() -> Result<PathBuf, ConfigRootError> {
    let platform = detect();
    resolve_config_root(
        platform,
        &|name| std::env::var(name).ok(),
        portable_config_dir(platform),
    )
}

/// Whether an `mpv` executable can be found on `PATH`.
///
/// Probes with `which mpv` (Linux/macOS) or `where mpv` (Windows); mpv is
/// never executed. A missing probe tool counts as "not available".
#[must_use]
pub fn mpv_available() -> bool {
    locate_mpv(detect()).is_some()
}

/// Failure of [`config_root`] to find any base directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigRootError {
    /// Platform the resolution was attempted for.
    pub platform: Platform,
    /// Environment variables tried, in priority order.
    pub missing: &'static [&'static str],
}

impl ConfigRootError {
    fn new(platform: Platform, missing: &'static [&'static str]) -> Self {
        Self { platform, missing }
    }
}

impl fmt::Display for ConfigRootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cannot resolve the mpv config directory on {}: none of {} are set",
            self.platform,
            self.missing.join(", ")
        )
    }
}

impl std::error::Error for ConfigRootError {}

/// Pure, injectable core of [`config_root`].
///
/// Variables are read through `env` instead of the process environment, so
/// tests can simulate any platform. `portable_config_dir` is the Windows
/// portable-mode directory, when one exists next to `mpv.exe`.
///
/// # Errors
///
/// [`ConfigRootError`] when every source for the platform is unavailable.
fn resolve_config_root(
    platform: Platform,
    env: &dyn Fn(&str) -> Option<String>,
    portable_config_dir: Option<PathBuf>,
) -> Result<PathBuf, ConfigRootError> {
    // MPV_HOME completely overrides every platform default, used verbatim.
    if let Some(dir) = lookup(env, "MPV_HOME") {
        return Ok(PathBuf::from(dir));
    }
    match platform {
        Platform::Windows => {
            if let Some(dir) = portable_config_dir {
                return Ok(dir);
            }
            if let Some(appdata) = lookup(env, "APPDATA") {
                return Ok(PathBuf::from(appdata).join("mpv"));
            }
            if let Some(profile) = lookup(env, "USERPROFILE") {
                return Ok(PathBuf::from(profile)
                    .join("AppData")
                    .join("Roaming")
                    .join("mpv"));
            }
            Err(ConfigRootError::new(
                platform,
                &["MPV_HOME", "APPDATA", "USERPROFILE"],
            ))
        }
        Platform::Linux | Platform::MacOS => {
            if let Some(xdg) = lookup(env, "XDG_CONFIG_HOME") {
                return Ok(PathBuf::from(xdg).join("mpv"));
            }
            if let Some(home) = lookup(env, "HOME") {
                return Ok(PathBuf::from(home).join(".config").join("mpv"));
            }
            Err(ConfigRootError::new(
                platform,
                &["MPV_HOME", "XDG_CONFIG_HOME", "HOME"],
            ))
        }
    }
}

/// Environment lookup that treats empty values as unset, matching both the
/// XDG spec and mpv's own handling.
fn lookup(env: &dyn Fn(&str) -> Option<String>, name: &str) -> Option<String> {
    env(name).filter(|value| !value.is_empty())
}

fn from_os(os: &str) -> Platform {
    match os {
        "windows" => Platform::Windows,
        "macos" => Platform::MacOS,
        _ => Platform::Linux,
    }
}

/// Windows portable-mode config directory: `portable_config/` next to
/// `mpv.exe`, when mpv is on `PATH` and that directory exists.
fn portable_config_dir(platform: Platform) -> Option<PathBuf> {
    if platform != Platform::Windows {
        return None;
    }
    let mpv_exe = locate_mpv(platform)?;
    let dir = mpv_exe.parent()?.join("portable_config");
    dir.is_dir().then_some(dir)
}

/// Locate mpv on `PATH` by inspecting probe output; mpv is never executed.
fn locate_mpv(platform: Platform) -> Option<PathBuf> {
    let (probe, arg) = probe_command(platform);
    let output = Command::new(probe).arg(arg).output().ok()?;
    if !output.status.success() {
        return None;
    }
    first_path(&String::from_utf8_lossy(&output.stdout))
}

/// Probe used to find mpv on `PATH`: `which` on Unix, `where` on Windows.
fn probe_command(platform: Platform) -> (&'static str, &'static str) {
    match platform {
        Platform::Windows => ("where", "mpv"),
        Platform::Linux | Platform::MacOS => ("which", "mpv"),
    }
}

/// First non-empty line of probe output (`where` lists every match),
/// tolerant of `\r\n` line endings.
fn first_path(stdout: &str) -> Option<PathBuf> {
    stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Env stub: only the variables listed in `pairs` are "set".
    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_owned())
        }
    }

    fn resolve(
        platform: Platform,
        pairs: &[(&str, &str)],
        portable: Option<PathBuf>,
    ) -> Result<PathBuf, ConfigRootError> {
        resolve_config_root(platform, &env(pairs), portable)
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn detect_and_config_root_on_linux_host() {
        assert_eq!(detect(), Platform::Linux);
        assert_eq!(detect(), from_os(std::env::consts::OS));
        // QA anchor: the real environment must resolve via the documented
        // MPV_HOME > XDG_CONFIG_HOME > HOME priority.
        let root = config_root().expect("MPV_HOME/XDG_CONFIG_HOME/HOME set on this machine");
        let expected = match std::env::var_os("MPV_HOME").filter(|v| !v.is_empty()) {
            Some(dir) => PathBuf::from(dir),
            None => match std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
                Some(xdg) => PathBuf::from(xdg).join("mpv"),
                None => PathBuf::from(std::env::var_os("HOME").expect("HOME set"))
                    .join(".config")
                    .join("mpv"),
            },
        };
        assert_eq!(root, expected);
    }

    #[test]
    fn from_os_maps_known_platforms_and_degrades_unknown_to_linux() {
        for (os, expected) in [
            ("linux", Platform::Linux),
            ("windows", Platform::Windows),
            ("macos", Platform::MacOS),
            ("freebsd", Platform::Linux),
            ("redox", Platform::Linux),
            ("", Platform::Linux),
        ] {
            assert_eq!(from_os(os), expected, "os {os:?}");
        }
    }

    #[test]
    fn display_and_display_name_agree_for_every_platform() {
        for (platform, name) in [
            (Platform::Linux, "Linux"),
            (Platform::Windows, "Windows"),
            (Platform::MacOS, "macOS"),
        ] {
            assert_eq!(platform.display_name(), name);
            assert_eq!(platform.to_string(), name);
        }
    }

    #[test]
    fn linux_prefers_xdg_config_home_and_falls_back_to_home() {
        let root = resolve(
            Platform::Linux,
            &[("XDG_CONFIG_HOME", "/custom/xdg"), ("HOME", "/home/u")],
            None,
        )
        .unwrap();
        assert_eq!(root, PathBuf::from("/custom/xdg/mpv"));
        // XDG unset or empty -> ~/.config/mpv (empty counts as unset).
        for pairs in [
            &[("HOME", "/home/u")][..],
            &[("XDG_CONFIG_HOME", ""), ("HOME", "/home/u")][..],
        ] {
            let root = resolve(Platform::Linux, pairs, None).unwrap();
            assert_eq!(root, PathBuf::from("/home/u/.config/mpv"));
        }
    }

    #[test]
    fn mpv_home_overrides_everything_and_is_used_verbatim() {
        let root = resolve(
            Platform::Linux,
            &[
                ("MPV_HOME", "/nonexistent/mpv-home"),
                ("XDG_CONFIG_HOME", "/custom/xdg"),
                ("HOME", "/home/u"),
            ],
            None,
        )
        .unwrap();
        // No existence check: the override is returned as-is.
        assert_eq!(root, PathBuf::from("/nonexistent/mpv-home"));
    }

    #[test]
    fn windows_defaults_to_appdata_with_userprofile_fallback() {
        let appdata = r"C:\Users\u\AppData\Roaming";
        let root = resolve(Platform::Windows, &[("APPDATA", appdata)], None).unwrap();
        assert_eq!(root, PathBuf::from(appdata).join("mpv"));
        let root = resolve(Platform::Windows, &[("USERPROFILE", r"C:\Users\u")], None).unwrap();
        assert_eq!(
            root,
            PathBuf::from(r"C:\Users\u")
                .join("AppData")
                .join("Roaming")
                .join("mpv")
        );
    }

    #[test]
    fn windows_portable_config_beats_appdata_but_not_mpv_home() {
        let portable = PathBuf::from(r"C:\tools\mpv\portable_config");
        let root = resolve(
            Platform::Windows,
            &[("APPDATA", r"C:\Users\u\AppData\Roaming")],
            Some(portable.clone()),
        )
        .unwrap();
        assert_eq!(root, portable);
        let root = resolve(
            Platform::Windows,
            &[
                ("MPV_HOME", r"D:\custom\mpv"),
                ("APPDATA", r"C:\Users\u\AppData\Roaming"),
            ],
            Some(portable),
        )
        .unwrap();
        assert_eq!(root, PathBuf::from(r"D:\custom\mpv"));
    }

    #[test]
    fn macos_defaults_to_home_config_and_honours_xdg() {
        let root = resolve(Platform::MacOS, &[("HOME", "/Users/mio")], None).unwrap();
        assert_eq!(root, PathBuf::from("/Users/mio/.config/mpv"));
        let root = resolve(
            Platform::MacOS,
            &[("XDG_CONFIG_HOME", "/custom/xdg"), ("HOME", "/Users/mio")],
            None,
        )
        .unwrap();
        assert_eq!(root, PathBuf::from("/custom/xdg/mpv"));
    }

    #[test]
    fn unresolvable_environment_is_an_error_not_a_panic() {
        for (platform, pairs) in [
            (Platform::Linux, &[][..]),
            (Platform::Linux, &[("HOME", "")][..]),
            (Platform::MacOS, &[][..]),
            (Platform::Windows, &[][..]),
        ] {
            let err = resolve(platform, pairs, None).unwrap_err();
            assert_eq!(err.platform, platform, "platform {platform:?}");
        }
        let message = resolve(Platform::Linux, &[], None).unwrap_err().to_string();
        assert!(message.contains("Linux"), "{message}");
        assert!(message.contains("XDG_CONFIG_HOME"), "{message}");
        assert!(message.contains("HOME"), "{message}");
    }

    #[test]
    fn probe_command_matches_platform() {
        assert_eq!(probe_command(Platform::Linux), ("which", "mpv"));
        assert_eq!(probe_command(Platform::MacOS), ("which", "mpv"));
        assert_eq!(probe_command(Platform::Windows), ("where", "mpv"));
    }

    #[test]
    fn first_path_takes_the_first_nonempty_probe_line() {
        assert_eq!(
            first_path("C:\\a\\mpv.exe\r\nC:\\b\\mpv.exe\r\n"),
            Some(PathBuf::from("C:\\a\\mpv.exe"))
        );
        assert_eq!(
            first_path("/usr/bin/mpv\n"),
            Some(PathBuf::from("/usr/bin/mpv"))
        );
        assert_eq!(first_path("  \n\n"), None);
        assert_eq!(first_path(""), None);
    }

    #[test]
    fn mpv_available_does_not_panic() {
        let _ = mpv_available();
    }
}
