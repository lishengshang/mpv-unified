//! First-run bootstrap of the user layer.
//!
//! The user layer (`user/`) is the personal-data layer of the app/user split
//! layout (see `docs/upgrade.md`): it is gitignored, never shipped in the
//! release zip, and never overwritten by upgrades. `gen` only creates it on
//! first run, from the `user.example.conf` template when one exists.

use super::error::GenError;
use std::fs;
use std::path::Path;

/// Fallback header for a template-less first run.
const DEFAULT_USER_CONF_HEADER: &str = "\
# user.conf — 个人配置层(首次运行自动创建)
#
# 本文件由 mpv-config gen 自动创建,因为仓库中缺少 user.example.conf 模板。
# user 层是四层合并(base → platform → package → user)的最后一层,优先级最高;
# 升级时此文件永不覆盖,可放心填写个人选项与 API 密钥(密钥已被 .gitignore 忽略)。
";

/// Ensure the user layer exists, without ever touching an existing
/// `user/user.conf`. Returns the bootstrap notice, or `None` when the layer
/// already exists.
pub(crate) fn ensure_user_layer(root: &Path) -> Result<Option<String>, GenError> {
    let user_conf = root.join("user").join("user.conf");
    if user_conf.is_file() {
        return Ok(None);
    }
    let user_dir = root.join("user");
    fs::create_dir_all(&user_dir).map_err(|source| GenError::Io {
        action: "创建 user 目录",
        path: user_dir.clone(),
        source,
    })?;
    let template = user_dir.join("user.example.conf");
    if template.is_file() {
        fs::copy(&template, &user_conf).map_err(|source| GenError::Io {
            action: "复制模板",
            path: user_conf.clone(),
            source,
        })?;
        Ok(Some(format!(
            "已自动创建 {} (首次运行,从 user.example.conf 复制)",
            user_conf.display()
        )))
    } else {
        fs::write(&user_conf, DEFAULT_USER_CONF_HEADER).map_err(|source| GenError::Io {
            action: "写入",
            path: user_conf.clone(),
            source,
        })?;
        Ok(Some(format!(
            "已自动创建 {} (首次运行,无模板,已写入默认头部注释)",
            user_conf.display()
        )))
    }
}
