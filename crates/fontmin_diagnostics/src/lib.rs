use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, FontminError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontminErrorKind {
    Io,
    Config,
    MissingGlyph,
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

    #[error("{message}")]
    #[diagnostic(code(fontmin::missing_glyph))]
    MissingGlyph { message: String },

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
    #[must_use]
    pub fn kind(&self) -> FontminErrorKind {
        match self {
            Self::Io { .. } => FontminErrorKind::Io,
            Self::Config { .. } => FontminErrorKind::Config,
            Self::MissingGlyph { .. } => FontminErrorKind::MissingGlyph,
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

    #[must_use]
    pub fn missing_glyphs(codepoints: &[u32]) -> Self {
        Self::MissingGlyph {
            message: format_missing_glyphs(codepoints),
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

fn format_missing_glyphs(codepoints: &[u32]) -> String {
    const LIMIT: usize = 16;

    let visible = codepoints
        .iter()
        .take(LIMIT)
        .map(|codepoint| format!("U+{codepoint:04X}"))
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = codepoints.len().saturating_sub(LIMIT);

    if remaining == 0 {
        format!("missing glyphs for requested Unicode code points: {visible}")
    } else {
        format!("missing glyphs for requested Unicode code points: {visible}, and {remaining} more")
    }
}

#[cfg(test)]
mod tests {
    use super::FontminError;

    #[test]
    fn bounds_human_readable_missing_glyph_diagnostics() {
        let codepoints = (0x41..=0x55).collect::<Vec<_>>();
        let message = FontminError::missing_glyphs(&codepoints).to_string();

        assert!(message.contains("U+0041"));
        assert!(message.contains("and 5 more"));
        assert!(!message.contains("U+0055"));
    }
}
