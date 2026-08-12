//! Table lookups and per-value validation on a loaded [`OptionsTable`].

use super::table::{GuiOption, OptionType, OptionsTable};
use super::validate::validate_text_value;
use super::OptionsGuiError;
use std::fs;
use std::path::Path;

impl OptionsTable {
    /// Load and parse the curated table at `path`.
    ///
    /// # Errors
    ///
    /// [`OptionsGuiError::Io`] on read failures, [`OptionsGuiError::Invalid`]
    /// on parse/validation failures.
    pub fn load(path: &Path) -> Result<OptionsTable, OptionsGuiError> {
        let text = fs::read_to_string(path).map_err(|source| OptionsGuiError::Io {
            action: "读取",
            path: path.to_path_buf(),
            source,
        })?;
        super::table::parse_yaml(&text)
    }

    /// Look up one option by key.
    #[must_use]
    pub fn find(&self, key: &str) -> Option<&GuiOption> {
        self.options.iter().find(|option| option.key == key)
    }

    /// Whether `value` equals the option's baseline default.
    ///
    /// Numeric options compare numerically (`"55"` and `"55.0"` are the same);
    /// every other type compares trimmed strings.
    #[must_use]
    pub fn is_default(&self, option: &GuiOption, value: &str) -> bool {
        let value = value.trim();
        if option.type_ == OptionType::Number {
            match (value.parse::<f64>(), option.default.parse::<f64>()) {
                (Ok(value), Ok(default)) => value == default,
                _ => value == option.default,
            }
        } else {
            value == option.default
        }
    }
}

/// Validate a form value for `key` against `table`: type and range.
///
/// Returns the trimmed, normalized value on success and a Chinese error
/// message (naming the reason) on failure.
pub fn validate_value(table: &OptionsTable, key: &str, value: &str) -> Result<String, String> {
    let option = table.find(key).ok_or_else(|| format!("未知选项:{key}"))?;
    let value = value.trim();
    match option.type_ {
        OptionType::Switch => {
            if value == "yes" || value == "no" {
                Ok(value.to_owned())
            } else {
                Err(format!(
                    "选项 \"{key}\" 是开关,值只能是 yes 或 no,收到:\"{value}\""
                ))
            }
        }
        OptionType::Number => {
            let number = value
                .parse::<f64>()
                .map_err(|_| format!("选项 \"{key}\" 必须是数字,收到:\"{value}\""))?;
            let Some((min, max)) = option.min.zip(option.max) else {
                return Err(format!("选项表损坏:number 选项 \"{key}\" 缺少 min/max"));
            };
            if !(min..=max).contains(&number) {
                return Err(format!("选项 \"{key}\" 超出范围 {min}~{max},收到:{value}"));
            }
            Ok(value.to_owned())
        }
        OptionType::Select => {
            if option.choices.iter().any(|c| c == value) {
                Ok(value.to_owned())
            } else {
                Err(format!(
                    "选项 \"{key}\" 可选值:{}",
                    option.choices.join(" / ")
                ))
            }
        }
        OptionType::String | OptionType::Path => validate_text_value(key, value),
    }
}
