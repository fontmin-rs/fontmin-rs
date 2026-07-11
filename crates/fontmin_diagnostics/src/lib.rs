use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, FontminError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontminErrorKind {
    Io,
    Config,
    UnsupportedFormat,
    InvalidFont,
    ConvertFailed,
    PluginFailed,
    NapiBridgeFailed,
}

#[derive(Debug, Diagnostic, Error)]
pub enum FontminError {
    #[error("I/O error while accessing {path}: {source}")]
    #[diagnostic(code(fontmin::io))]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("configuration error: {message}")]
    #[diagnostic(code(fontmin::config))]
    Config { message: String },

    #[error("unsupported font format: {format}")]
    #[diagnostic(code(fontmin::unsupported_format))]
    UnsupportedFormat { format: String },

    #[error("invalid font data: {message}")]
    #[diagnostic(code(fontmin::invalid_font))]
    InvalidFont { message: String },

    #[error("conversion failed: {message}")]
    #[diagnostic(code(fontmin::convert_failed))]
    ConvertFailed { message: String },

    #[error("plugin failed: {plugin}: {message}")]
    #[diagnostic(code(fontmin::plugin_failed))]
    PluginFailed { plugin: String, message: String },

    #[error("napi bridge failed: {message}")]
    #[diagnostic(code(fontmin::napi_bridge_failed))]
    NapiBridgeFailed { message: String },
}

impl FontminError {
    pub fn kind(&self) -> FontminErrorKind {
        match self {
            Self::Io { .. } => FontminErrorKind::Io,
            Self::Config { .. } => FontminErrorKind::Config,
            Self::UnsupportedFormat { .. } => FontminErrorKind::UnsupportedFormat,
            Self::InvalidFont { .. } => FontminErrorKind::InvalidFont,
            Self::ConvertFailed { .. } => FontminErrorKind::ConvertFailed,
            Self::PluginFailed { .. } => FontminErrorKind::PluginFailed,
            Self::NapiBridgeFailed { .. } => FontminErrorKind::NapiBridgeFailed,
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    pub fn invalid_font(message: impl Into<String>) -> Self {
        Self::InvalidFont {
            message: message.into(),
        }
    }

    pub fn convert_failed(message: impl Into<String>) -> Self {
        Self::ConvertFailed {
            message: message.into(),
        }
    }

    pub fn unsupported(format: impl Into<String>) -> Self {
        Self::UnsupportedFormat {
            format: format.into(),
        }
    }
}
