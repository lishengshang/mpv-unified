//! Failure types of the `gen` command.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// A failure of the `gen` command.
#[derive(Debug)]
pub enum GenError {
    /// A required layer file is missing.
    MissingLayer { path: PathBuf },
    /// Filesystem failure (read, directory creation, or write).
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    /// Parse failure inside one layer; `line` is 1-based.
    Parse {
        layer: String,
        line: usize,
        message: String,
    },
    /// Conditional-directive failure inside one layer.
    Cond {
        layer: String,
        line: usize,
        message: String,
    },
}

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingLayer { path } => write!(
                f,
                "缺少必需层文件 {}:请在本工具所在目录(解压后的 mpv-config 目录)内运行,且该文件必须存在",
                path.display()
            ),
            Self::Io {
                action,
                path,
                source,
            } => write!(f, "{action} {} 失败:{source}", path.display()),
            Self::Parse {
                layer,
                line,
                message,
            } => write!(f, "解析 {layer} 失败:line {line}: {message}"),
            Self::Cond {
                layer,
                line,
                message,
            } => write!(f, "求值 {layer} 条件指令失败:line {line}: {message}"),
        }
    }
}

impl std::error::Error for GenError {}

/// Wrap a [`core::cond::CondError`] with the layer it occurred in.
pub(crate) fn cond_error(layer: &str, error: core::cond::CondError) -> GenError {
    GenError::Cond {
        layer: layer.to_owned(),
        line: error.line,
        message: error.message,
    }
}
