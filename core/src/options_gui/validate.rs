//! Validation helpers for the curated table (shared by `parse_yaml` and the
//! runtime value check in [`crate::options_gui::io`]).

use super::table::RawOption;
use super::{OptionsGuiError, invalid};
use crate::options_gui::table::OptionType;

/// Map the YAML type string to an [`OptionType`], or `None` for unknown
/// spellings.
pub(crate) fn parse_type(raw: &str) -> Option<OptionType> {
    match raw {
        "switch" => Some(OptionType::Switch),
        "number" => Some(OptionType::Number),
        "select" => Some(OptionType::Select),
        "string" => Some(OptionType::String),
        "path" => Some(OptionType::Path),
        _ => None,
    }
}

/// A YAML scalar as a string; booleans become `yes`/`no` (YAML 1.1 spellings
/// of switches), numbers become their decimal form. Non-scalar values fall
/// back to their string form when available, else an empty string (such a
/// default is rejected by the type checks anyway).
pub(crate) fn scalar_to_string(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::Bool(true) => "yes".to_owned(),
        serde_yaml::Value::Bool(false) => "no".to_owned(),
        serde_yaml::Value::Number(number) => number.to_string(),
        serde_yaml::Value::String(text) => text.clone(),
        _ => value.as_str().unwrap_or_default().to_owned(),
    }
}

/// A YAML scalar as a float, or `None` when it is not numeric.
pub(crate) fn scalar_to_f64(value: &serde_yaml::Value) -> Option<f64> {
    match value {
        serde_yaml::Value::Number(number) => number.as_f64(),
        _ => None,
    }
}

/// `number` options must declare numeric `min` and `max` with `min <= max`.
pub(crate) fn validate_number_bounds(
    key: &str,
    type_: OptionType,
    raw: &RawOption,
) -> Result<(Option<f64>, Option<f64>), OptionsGuiError> {
    if type_ != OptionType::Number {
        return Ok((None, None));
    }
    let min = raw
        .min
        .as_ref()
        .and_then(scalar_to_f64)
        .ok_or_else(|| invalid(format!("number 选项 \"{key}\" 缺少数字 min")))?;
    let max = raw
        .max
        .as_ref()
        .and_then(scalar_to_f64)
        .ok_or_else(|| invalid(format!("number 选项 \"{key}\" 缺少数字 max")))?;
    if min > max {
        return Err(invalid(format!(
            "number 选项 \"{key}\" 的 min({min}) 大于 max({max})"
        )));
    }
    Ok((Some(min), Some(max)))
}

/// `select` options must declare non-empty, duplicate-free `choices`.
pub(crate) fn validate_select(
    key: &str,
    type_: OptionType,
    raw: &RawOption,
    default: &str,
) -> Result<Vec<String>, OptionsGuiError> {
    if type_ != OptionType::Select {
        return Ok(Vec::new());
    }
    if raw.choices.is_empty() {
        return Err(invalid(format!("select 选项 \"{key}\" 缺少 choices")));
    }
    let mut seen = std::collections::HashSet::new();
    for choice in &raw.choices {
        if !seen.insert(choice.as_str()) {
            return Err(invalid(format!(
                "select 选项 \"{key}\" 的 choices 重复:{choice}"
            )));
        }
    }
    if !raw.choices.iter().any(|c| c == default) {
        return Err(invalid(format!(
            "select 选项 \"{key}\" 的 default \"{default}\" 不在 choices 中"
        )));
    }
    Ok(raw.choices.clone())
}

/// The default's type must match the option's kind (and number defaults must
/// lie inside the declared range).
pub(crate) fn validate_default_type(
    key: &str,
    type_: OptionType,
    default: &str,
    min: Option<f64>,
    max: Option<f64>,
    choices: &[String],
) -> Result<(), OptionsGuiError> {
    match type_ {
        OptionType::Number => {
            let value = default.parse::<f64>().map_err(|_| {
                invalid(format!(
                    "number 选项 \"{key}\" 的 default \"{default}\" 不是数字"
                ))
            })?;
            let (Some(min), Some(max)) = (min, max) else {
                return Err(invalid(format!(
                    "number 选项 \"{key}\" 缺少 min/max(校验顺序错误)"
                )));
            };
            if !(min..=max).contains(&value) {
                return Err(invalid(format!(
                    "number 选项 \"{key}\" 的 default({value}) 超出范围 {min}~{max}"
                )));
            }
        }
        OptionType::Switch => {
            if default != "yes" && default != "no" {
                return Err(invalid(format!(
                    "switch 选项 \"{key}\" 的 default 必须是 yes 或 no:\"{default}\""
                )));
            }
        }
        OptionType::Select => {
            if !choices.iter().any(|c| c == default) {
                return Err(invalid(format!(
                    "select 选项 \"{key}\" 的 default \"{default}\" 不在 choices 中"
                )));
            }
        }
        OptionType::String | OptionType::Path => {}
    }
    Ok(())
}

/// Validate a free-form text value (string/path): non-empty, single line,
/// no inline-comment marker, balanced quotes.
pub(crate) fn validate_text_value(key: &str, value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err(format!("选项 \"{key}\" 的值不能为空"));
    }
    if value.contains('\n') {
        return Err(format!("选项 \"{key}\" 的值不能包含换行"));
    }
    if value.contains(" #") {
        return Err(format!("选项 \"{key}\" 的值不能包含 \" #\"(会被解析为行内注释)"));
    }
    if value.matches('"').count() % 2 != 0 || value.matches('\'').count() % 2 != 0 {
        return Err(format!("选项 \"{key}\" 的值引号未闭合"));
    }
    Ok(value.to_owned())
}
