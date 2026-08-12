//! Profile cards: parse `config/profiles.yaml`, turn enabled profiles into
//! mpv profile blocks, and persist the enabled set in `user/profiles-state.json`.
//!
//! A profile is a named set of `key=value` option lines plus optional
//! package dependencies (`requires`). Enabling a profile appends an mpv
//! `[id]` block to the generated `mpv.conf` with a
//! `profile-restore=copy-equal` first line (decision D16) followed by the
//! option lines, and a `profile=id` line that applies the block at startup.
//! The blocks live at the end of the generated file only — they never touch
//! `user.conf` or any other layer, so clearing a profile is state removal +
//! regeneration.
//!
//! Validation: `id` and `name` are required, ids must be unique, and
//! `options` must be an array of non-empty single-line strings. Parse and
//! validation failures are typed [`ProfilesError`]s with Chinese messages —
//! nothing here panics. The state file is written atomically (tmp + rename);
//! a missing state file reads as empty, a *corrupt* one also reads as empty
//! (treat as "no profiles enabled", never an error).

use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// File name of the persisted enabled-set, inside the `user/` directory.
pub const STATE_FILE: &str = "profiles-state.json";

/// Profiles enabled by default on first run (when the state file does not
/// exist yet).
pub const DEFAULT_ENABLED: &[&str] = &["cinema"];

/// One profile definition from `config/profiles.yaml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// Unique machine id, also the mpv profile name (`[id]` block header).
    pub id: String,
    /// Human-readable name, shown on the card and in the block comment.
    pub name: String,
    /// One-line card description.
    pub desc: String,
    /// Card icon (emoji).
    pub icon: String,
    /// `key=value` option lines written inside the profile block.
    pub options: Vec<String>,
    /// Package names that must be installed for the profile to work.
    pub requires: Vec<String>,
}

/// Untyped YAML shape; `Option` fields are validated by hand so errors can
/// name the offending profile.
#[derive(Debug, Deserialize)]
struct RawProfiles {
    profiles: Vec<RawProfile>,
}

#[derive(Debug, Deserialize)]
struct RawProfile {
    id: Option<String>,
    name: Option<String>,
    #[serde(default)]
    desc: Option<String>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    options: Vec<serde_yaml::Value>,
    #[serde(default)]
    requires: Vec<String>,
}

/// A failure of the profiles module.
#[derive(Debug)]
pub enum ProfilesError {
    /// Filesystem failure (read, directory creation, or write).
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    /// YAML syntax error or a validation violation (missing/duplicate id,
    /// missing name, malformed option).
    Invalid { message: String },
}

impl fmt::Display for ProfilesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                action,
                path,
                source,
            } => write!(f, "{action} {} 失败:{source}", path.display()),
            Self::Invalid { message } => f.write_str(message),
        }
    }
}

impl std::error::Error for ProfilesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Invalid { .. } => None,
        }
    }
}

/// Wrap a message as [`ProfilesError::Invalid`].
fn invalid(message: impl Into<String>) -> ProfilesError {
    ProfilesError::Invalid {
        message: message.into(),
    }
}

/// Parse `config/profiles.yaml` text into validated profiles.
///
/// # Errors
///
/// YAML syntax errors, missing `profiles` key, missing/empty `id` or
/// `name`, duplicate ids, and malformed option lines are all reported with
/// a Chinese message naming the offending profile.
pub fn parse_yaml(text: &str) -> Result<Vec<Profile>, ProfilesError> {
    let raw: RawProfiles = serde_yaml::from_str(text)
        .map_err(|error| invalid(format!("config/profiles.yaml 解析失败:{error}")))?;
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(raw.profiles.len());
    for (index, raw_profile) in raw.profiles.iter().enumerate() {
        let position = index + 1;
        let mut validated_options = Vec::with_capacity(raw_profile.options.len());
        let id = raw_profile
            .id
            .as_ref()
            .ok_or_else(|| invalid(format!("第 {position} 个方案缺少 id")))?
            .trim()
            .to_owned();
        if id.is_empty() {
            return Err(invalid(format!("第 {position} 个方案的 id 不能为空")));
        }
        if id.contains(['[', ']', '\n']) {
            return Err(invalid(format!(
                "方案 id 不能包含 [ ] 或换行:第 {position} 个方案 (\"{id}\")"
            )));
        }
        if !seen.insert(id.clone()) {
            return Err(invalid(format!("方案 id 重复:{id}")));
        }
        let name = raw_profile
            .name
            .as_ref()
            .ok_or_else(|| invalid(format!("方案 \"{id}\" 缺少 name")))?
            .trim()
            .to_owned();
        if name.is_empty() {
            return Err(invalid(format!("方案 \"{id}\" 的 name 不能为空")));
        }
        for (option_index, option) in raw_profile.options.iter().enumerate() {
            let option = match option {
                serde_yaml::Value::String(text) => text,
                _ => {
                    return Err(invalid(format!(
                        "方案 \"{id}\" 的第 {} 个选项不是字符串",
                        option_index + 1
                    )));
                }
            };
            if option.is_empty() {
                return Err(invalid(format!(
                    "方案 \"{id}\" 的第 {} 个选项为空",
                    option_index + 1
                )));
            }
            if option.contains('\n') {
                return Err(invalid(format!(
                    "方案 \"{id}\" 的选项不能包含换行:{}",
                    option.lines().next().unwrap_or_default()
                )));
            }
            validated_options.push(option.clone());
        }
        out.push(Profile {
            id,
            name,
            desc: raw_profile.desc.clone().unwrap_or_default(),
            icon: raw_profile.icon.clone().unwrap_or_default(),
            options: validated_options,
            requires: raw_profile.requires.clone(),
        });
    }
    Ok(out)
}

/// Load and parse the profiles file at `path`.
///
/// # Errors
///
/// [`ProfilesError::Io`] on read failures, [`ProfilesError::Invalid`] on
/// parse/validation failures.
pub fn load(path: &Path) -> Result<Vec<Profile>, ProfilesError> {
    let text = fs::read_to_string(path).map_err(|source| ProfilesError::Io {
        action: "读取",
        path: path.to_path_buf(),
        source,
    })?;
    parse_yaml(&text)
}

/// Render the mpv profile blocks for every profile whose id appears in
/// `enabled_ids`, plus the applying `profile=id` lines.
///
/// Block order follows the definition order of `profiles` (ids in
/// `enabled_ids` that match no profile are skipped silently — stale state
/// is not an error). Each block is headed by
/// `# 方案:<name> (由 mpv-config 生成)`, opens with
/// `profile-restore=copy-equal`, then lists the option lines. After all
/// blocks one `profile=<id>` line per enabled profile applies them at
/// startup; `profile=` lines come after their blocks so mpv has registered
/// the profile before applying it. Returns an empty string when no enabled
/// profile matches.
#[must_use]
pub fn generate_profile_blocks(profiles: &[Profile], enabled_ids: &[String]) -> String {
    let mut text = String::new();
    let mut apply_lines = Vec::new();
    for profile in profiles {
        if !enabled_ids.iter().any(|id| id == &profile.id) {
            continue;
        }
        text.push_str(&format!("# 方案:{} (由 mpv-config 生成)\n", profile.name));
        text.push_str(&format!("[{}]\n", profile.id));
        text.push_str("profile-restore=copy-equal\n");
        for option in &profile.options {
            text.push_str(option);
            text.push('\n');
        }
        text.push('\n');
        apply_lines.push(format!("profile={}", profile.id));
    }
    if apply_lines.is_empty() {
        return String::new();
    }
    for line in apply_lines {
        text.push_str(&line);
        text.push('\n');
    }
    text
}

/// The persisted enabled ids, or `[]` when the state file is missing or
/// corrupt.
///
/// # Errors
///
/// Only filesystem failures (other than missing-file) surface as errors.
pub fn read_state(user_dir: &Path) -> Result<Vec<String>, ProfilesError> {
    Ok(read_state_opt(user_dir)?.unwrap_or_default())
}

/// The effective enabled ids: the persisted state when the file exists, or
/// [`DEFAULT_ENABLED`] on first run (no state file yet).
///
/// # Errors
///
/// Only filesystem failures (other than missing-file) surface as errors.
pub fn effective_enabled(user_dir: &Path) -> Result<Vec<String>, ProfilesError> {
    Ok(match read_state_opt(user_dir)? {
        Some(ids) => ids,
        None => DEFAULT_ENABLED.iter().map(|id| (*id).to_owned()).collect(),
    })
}

/// Persist `enabled_ids` to `<user_dir>/profiles-state.json` atomically
/// (tmp file + rename), creating `user_dir` when missing.
///
/// # Errors
///
/// [`ProfilesError::Io`] on any filesystem failure.
pub fn write_state(user_dir: &Path, enabled_ids: &[String]) -> Result<(), ProfilesError> {
    fs::create_dir_all(user_dir).map_err(|source| ProfilesError::Io {
        action: "创建目录",
        path: user_dir.to_path_buf(),
        source,
    })?;
    let path = user_dir.join(STATE_FILE);
    let tmp = user_dir.join(format!("{STATE_FILE}.tmp"));
    let json = serde_json::to_string(enabled_ids).map_err(|error| {
        invalid(format!(
            "序列化方案启用状态失败(不可能发生):{error}"
        ))
    })?;
    fs::write(&tmp, json).map_err(|source| ProfilesError::Io {
        action: "写入",
        path: tmp.clone(),
        source,
    })?;
    fs::rename(&tmp, &path).map_err(|source| {
        let _ = fs::remove_file(&tmp);
        ProfilesError::Io {
            action: "写入",
            path: path.clone(),
            source,
        }
    })
}

/// Read the raw state: `None` when the file does not exist, `Some(ids)`
/// otherwise. A corrupt file reads as an empty list — treated as "nothing
/// enabled", never an error (QA: damaged state must not panic).
fn read_state_opt(user_dir: &Path) -> Result<Option<Vec<String>>, ProfilesError> {
    let path = user_dir.join(STATE_FILE);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(ProfilesError::Io {
                action: "读取",
                path,
                source,
            });
        }
    };
    match serde_json::from_str::<Vec<String>>(&raw) {
        Ok(ids) => Ok(Some(ids)),
        Err(_) => Ok(Some(Vec::new())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "\
profiles:
  - id: cinema
    name: 高清观影
    desc: 画质优先
    icon: 🎬
    options:
      - \"deband=yes\"
      - \"scale=ewa_lanczossharp\"
    requires: []
  - id: music
    name: 音乐模式
    desc: 纯音频
    icon: 🎧
    options:
      - \"vo=null\"
    requires: [uosc]
";

    fn valid_profiles() -> Vec<Profile> {
        parse_yaml(VALID).expect("fixture parses")
    }

    fn ids(profiles: &[Profile]) -> Vec<String> {
        profiles.iter().map(|p| p.id.clone()).collect()
    }

    fn user_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mpv-config-profiles-{label}"));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    // ------------------------------------------------------------ parsing

    #[test]
    fn parses_valid_yaml_with_field_defaults() {
        let profiles = valid_profiles();
        assert_eq!(ids(&profiles), ["cinema", "music"]);
        let cinema = &profiles[0];
        assert_eq!(cinema.name, "高清观影");
        assert_eq!(cinema.desc, "画质优先");
        assert_eq!(cinema.icon, "🎬");
        assert_eq!(cinema.options, ["deband=yes", "scale=ewa_lanczossharp"]);
        assert!(cinema.requires.is_empty(), "requires defaults to empty");
        assert_eq!(profiles[1].requires, ["uosc"]);
    }

    #[test]
    fn missing_id_is_an_error() {
        let error = parse_yaml("profiles:\n  - name: 无名\n").expect_err("id required");
        assert!(error.to_string().contains("id"), "{error}");
    }

    #[test]
    fn missing_name_is_an_error() {
        let error = parse_yaml("profiles:\n  - id: a\n").expect_err("name required");
        assert!(error.to_string().contains("name"), "{error}");
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let error = parse_yaml(
            "profiles:\n  - id: dup\n    name: A\n  - id: dup\n    name: B\n",
        )
        .expect_err("duplicate id rejected");
        assert!(error.to_string().contains("重复"), "{error}");
    }

    #[test]
    fn non_string_option_is_a_yaml_error() {
        let error = parse_yaml(
            "profiles:\n  - id: a\n    name: A\n    options:\n      - 42\n",
        )
        .expect_err("integer option rejected");
        assert!(error.to_string().contains("不是字符串"), "{error}");
    }

    #[test]
    fn empty_or_multiline_options_are_rejected() {
        let error = parse_yaml(
            "profiles:\n  - id: a\n    name: A\n    options:\n      - \"\"\n",
        )
        .expect_err("empty option rejected");
        assert!(error.to_string().contains("选项为空"), "{error}");

        let error = parse_yaml(
            "profiles:\n  - id: a\n    name: A\n    options:\n      - \"x=1\\ny=2\"\n",
        )
        .expect_err("multiline option rejected");
        assert!(error.to_string().contains("换行"), "{error}");
    }

    // ------------------------------------------------ block generation

    #[test]
    fn single_enabled_profile_emits_block_with_comment_and_copy_equal() {
        let profiles = valid_profiles();
        let out = generate_profile_blocks(&profiles, &["cinema".to_owned()]);
        assert_eq!(
            out,
            "# 方案:高清观影 (由 mpv-config 生成)\n\
             [cinema]\n\
             profile-restore=copy-equal\n\
             deband=yes\n\
             scale=ewa_lanczossharp\n\
             \n\
             profile=cinema\n"
        );
    }

    #[test]
    fn blocks_follow_definition_order_not_enabled_order() {
        let profiles = valid_profiles();
        let out = generate_profile_blocks(&profiles, &["music".to_owned(), "cinema".to_owned()]);
        let cinema = out.find("[cinema]").expect("cinema block present");
        let music = out.find("[music]").expect("music block present");
        assert!(cinema < music, "definition order preserved");
        let apply_cinema = out.find("profile=cinema").expect("apply line present");
        let apply_music = out.find("profile=music").expect("apply line present");
        assert!(apply_cinema < apply_music, "apply lines follow block order");
        assert!(out.contains("# 方案:音乐模式 (由 mpv-config 生成)"));
    }

    #[test]
    fn no_enabled_profiles_produce_no_output() {
        let profiles = valid_profiles();
        assert_eq!(generate_profile_blocks(&profiles, &[]), "");
        assert_eq!(
            generate_profile_blocks(&profiles, &["ghost".to_owned()]),
            "",
            "unknown ids are skipped silently"
        );
    }

    // ------------------------------------------------------------ state

    #[test]
    fn missing_state_file_reads_as_empty() {
        let dir = user_dir("missing");
        assert_eq!(read_state(&dir).expect("read succeeds"), Vec::<String>::new());
        assert!(!dir.exists(), "read must not create the directory");
    }

    #[test]
    fn corrupt_state_file_reads_as_empty_without_panicking() {
        let dir = user_dir("corrupt");
        fs::create_dir_all(&dir).expect("create dir");
        fs::write(dir.join(STATE_FILE), "{ not json !!").expect("write corrupt state");
        assert_eq!(read_state(&dir).expect("no error on corrupt state"), Vec::<String>::new());
    }

    #[test]
    fn write_state_roundtrips_and_is_atomic() {
        let dir = user_dir("roundtrip");
        let ids = ["cinema".to_owned(), "music".to_owned()];
        write_state(&dir, &ids).expect("write succeeds");
        assert_eq!(read_state(&dir).expect("read succeeds"), ids);
        let entries: Vec<String> = fs::read_dir(&dir)
            .expect("list dir")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            !entries.iter().any(|name| name.contains(".tmp")),
            "tmp file must not remain: {entries:?}"
        );
        assert_eq!(serde_json::to_string(&ids).expect("json"), "[\"cinema\",\"music\"]");
    }

    #[test]
    fn write_state_overwrites_previous_state() {
        let dir = user_dir("overwrite");
        write_state(&dir, &["cinema".to_owned()]).expect("first write");
        write_state(&dir, &["music".to_owned()]).expect("second write");
        assert_eq!(read_state(&dir).expect("read"), ["music".to_owned()]);
    }

    #[test]
    fn effective_enabled_defaults_to_cinema_on_first_run() {
        let dir = user_dir("effective");
        assert_eq!(
            effective_enabled(&dir).expect("defaults apply"),
            ["cinema".to_owned()]
        );
        // An explicit empty state disables everything — no default.
        write_state(&dir, &[]).expect("write empty state");
        assert_eq!(effective_enabled(&dir).expect("explicit state wins"), Vec::<String>::new());
    }
}
