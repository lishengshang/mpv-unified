//! Curated option form for the GUI "配置" page (task 19, decision D13).
//!
//! Loads `config/options-gui.yaml` — a curated table of ~50-100 high-frequency
//! mpv options (never the full 400+) — and manages the GUI fragment file
//! `user/gui.conf`. The fragment follows the mpv.net "only write non-default
//! values" strategy: saving a form writes `key=value` lines only for values
//! that differ from the table's `default` baseline, and re-parsing +
//! serializing through [`crate::conf`] preserves every comment byte-for-byte.
//! The file carries the header comment [`HEADER_COMMENT`]; writes are atomic
//! (tmp + rename). `gui.conf` is a separate layer above `user.conf` in `gen`
//! (higher priority), so the GUI never touches manually-edited `user.conf`.
//!
//! Validation: [`table::parse_yaml`] rejects duplicate keys, unknown
//! types/categories, `number` options without `min`/`max`, `select` options
//! without `choices`, and defaults whose type does not match the option.
//! Every failure is a typed [`OptionsGuiError`] with a Chinese message naming
//! the offending key — nothing here panics.

mod gui_conf;
mod ops;
mod table;
mod validate;

pub use gui_conf::{read_gui_conf, write_gui_conf};
pub use ops::validate_value;
pub use table::{parse_yaml, GuiOption, OptionType, OptionsTable, CATEGORIES};

use std::fmt;
use std::io;
use std::path::PathBuf;

/// File name of the GUI fragment, inside the `user/` directory.
pub const GUI_CONF_FILE: &str = "gui.conf";

/// Header comment written at the top of every generated `gui.conf`.
pub const HEADER_COMMENT: &str = "# 由 mpv-config GUI 管理";

/// A failure of the options-gui module.
#[derive(Debug)]
pub enum OptionsGuiError {
    /// Filesystem failure (read, directory creation, or write).
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    /// YAML syntax error or a validation violation.
    Invalid { message: String },
}

impl fmt::Display for OptionsGuiError {
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

impl std::error::Error for OptionsGuiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Invalid { .. } => None,
        }
    }
}

/// Wrap a message as [`OptionsGuiError::Invalid`].
pub(crate) fn invalid(message: impl Into<String>) -> OptionsGuiError {
    OptionsGuiError::Invalid {
        message: message.into(),
    }
}
