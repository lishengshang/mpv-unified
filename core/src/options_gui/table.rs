//! The curated table: option definitions, the YAML shape and `parse_yaml`.

use super::validate::{
    parse_type, scalar_to_string, validate_default_type, validate_number_bounds, validate_select,
};
use super::{invalid, OptionsGuiError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// The fixed chapter set of the curated table (decision D8).
pub const CATEGORIES: [&str; 8] = [
    "general",
    "video",
    "audio",
    "subtitle",
    "performance",
    "network",
    "window",
    "other",
];

/// Control kind of one curated option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OptionType {
    /// Boolean `yes`/`no` (rendered as a toggle).
    Switch,
    /// Numeric value with `min`/`max` (rendered as slider + input).
    Number,
    /// One value from `choices` (rendered as a dropdown).
    Select,
    /// Free-form single-line text.
    String,
    /// A file/directory path (rendered as text input).
    Path,
}

/// One curated option definition.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GuiOption {
    /// mpv option name (must be a real mpv option, never invented).
    pub key: String,
    /// Chapter from [`CATEGORIES`].
    pub category: String,
    /// Control kind; serialized as `"switch" | "number" | ...`.
    #[serde(rename = "type")]
    pub type_: OptionType,
    /// Inclusive lower bound (`number` only).
    pub min: Option<f64>,
    /// Inclusive upper bound (`number` only).
    pub max: Option<f64>,
    /// Allowed values (`select` only).
    pub choices: Vec<String>,
    /// Baseline value: only differing values are written to `gui.conf`.
    pub default: String,
    /// Chinese description shown in the UI.
    pub desc_zh: String,
    /// English description shown in the UI.
    pub desc_en: String,
    /// mpv manual anchor URL.
    pub doc_url: String,
}

/// The curated table: an ordered list of options.
#[derive(Debug, Clone, PartialEq)]
pub struct OptionsTable {
    pub options: Vec<GuiOption>,
}

/// Untyped YAML shape; fields are validated by hand so errors can name the
/// offending key.
#[derive(Debug, Deserialize)]
struct RawOptions {
    options: Vec<RawOption>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawOption {
    key: Option<String>,
    category: Option<String>,
    #[serde(rename = "type")]
    type_: Option<String>,
    pub(crate) min: Option<serde_yaml::Value>,
    pub(crate) max: Option<serde_yaml::Value>,
    #[serde(default)]
    pub(crate) choices: Vec<String>,
    default: Option<serde_yaml::Value>,
    desc_zh: Option<String>,
    desc_en: Option<String>,
    doc_url: Option<String>,
}

/// Parse `config/options-gui.yaml` text into a validated table.
///
/// # Errors
///
/// YAML syntax errors, duplicate keys, unknown type/category, `number`
/// options missing `min`/`max`, `select` options missing `choices` or with a
/// default outside the choices, and defaults whose type does not match the
/// option are all reported with a Chinese message naming the offending key.
pub fn parse_yaml(text: &str) -> Result<OptionsTable, OptionsGuiError> {
    let raw: RawOptions = serde_yaml::from_str(text)
        .map_err(|error| invalid(format!("config/options-gui.yaml 解析失败:{error}")))?;
    let mut seen = HashSet::new();
    let mut options = Vec::with_capacity(raw.options.len());
    for (index, raw_option) in raw.options.iter().enumerate() {
        let position = index + 1;
        let key = raw_option
            .key
            .as_ref()
            .ok_or_else(|| invalid(format!("第 {position} 个选项缺少 key")))?
            .trim()
            .to_owned();
        if key.is_empty() {
            return Err(invalid(format!("第 {position} 个选项的 key 不能为空")));
        }
        if key
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '=' | '#' | '[' | ']'))
        {
            return Err(invalid(format!(
                "选项 key 不能包含空白或 = # [ ] 字符:\"{key}\""
            )));
        }
        if !seen.insert(key.clone()) {
            return Err(invalid(format!("选项 key 重复:{key}")));
        }
        let type_name = raw_option
            .type_
            .as_ref()
            .ok_or_else(|| invalid(format!("选项 \"{key}\" 缺少 type")))?
            .trim()
            .to_owned();
        let type_ = parse_type(&type_name).ok_or_else(|| {
            invalid(format!(
                "选项 \"{key}\" 的 type 非法:\"{type_name}\"(可选:switch/number/select/string/path)"
            ))
        })?;
        let category = raw_option
            .category
            .as_ref()
            .ok_or_else(|| invalid(format!("选项 \"{key}\" 缺少 category")))?
            .trim()
            .to_owned();
        if !CATEGORIES.contains(&category.as_str()) {
            return Err(invalid(format!(
                "选项 \"{key}\" 的 category 非法:\"{category}\"(可选:{})",
                CATEGORIES.join("/")
            )));
        }
        let default = raw_option
            .default
            .as_ref()
            .map(scalar_to_string)
            .ok_or_else(|| invalid(format!("选项 \"{key}\" 缺少 default")))?;

        let (min, max) = validate_number_bounds(key.as_str(), type_, raw_option)?;
        let choices = validate_select(key.as_str(), type_, raw_option, default.as_str())?;
        validate_default_type(key.as_str(), type_, &default, min, max, &choices)?;

        let desc_zh = raw_option
            .desc_zh
            .as_ref()
            .filter(|d| !d.trim().is_empty())
            .ok_or_else(|| invalid(format!("选项 \"{key}\" 缺少 desc_zh")))?
            .clone();
        let desc_en = raw_option
            .desc_en
            .as_ref()
            .filter(|d| !d.trim().is_empty())
            .ok_or_else(|| invalid(format!("选项 \"{key}\" 缺少 desc_en")))?
            .clone();
        let doc_url = raw_option
            .doc_url
            .as_ref()
            .filter(|d| !d.trim().is_empty())
            .ok_or_else(|| invalid(format!("选项 \"{key}\" 缺少 doc_url")))?
            .clone();

        options.push(GuiOption {
            key,
            category,
            type_,
            min,
            max,
            choices,
            default,
            desc_zh,
            desc_en,
            doc_url,
        });
    }
    Ok(OptionsTable { options })
}
