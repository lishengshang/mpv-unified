//! The package-layer config fragment `config.d/packages/<name>.conf`.
//!
//! Written by install and update; consumed by the generator's package layer
//! (T5 merge engine reads every `*.conf` under `config.d/packages/`).

use std::fs;
use std::path::Path;

use pkg::manifest::Manifest;

use super::error::{LifecycleError, Result};

/// Write the fragment for `manifest`, or remove it when the manifest has no
/// `config` lines. Returns whether a fragment exists after the call.
///
/// # Errors
///
/// [`LifecycleError`] on filesystem failures.
pub(crate) fn write_config_fragment(root: &Path, manifest: &Manifest) -> Result<bool> {
    let dir = root.join("config.d").join("packages");
    let fragment = dir.join(format!("{}.conf", manifest.name));
    if manifest.config.is_empty() {
        let _ = fs::remove_file(&fragment);
        return Ok(false);
    }
    fs::create_dir_all(&dir)
        .map_err(|e| LifecycleError::io(format!("创建 {}", dir.display()), e))?;
    let mut body = format!(
        "# 由 `mpv-config pkg` 管理 - {} v{}\n",
        manifest.name, manifest.version
    );
    for line in &manifest.config {
        body.push_str(line);
        body.push('\n');
    }
    fs::write(&fragment, body)
        .map_err(|e| LifecycleError::io(format!("写入 {}", fragment.display()), e))?;
    Ok(true)
}
