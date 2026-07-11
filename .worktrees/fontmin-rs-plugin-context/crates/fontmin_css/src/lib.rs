use fontmin_core::OutputFormat;
use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssOptions {
    pub font_family: String,
    pub font_path: String,
    pub local: bool,
    pub font_display: String,
}

impl Default for CssOptions {
    fn default() -> Self {
        Self {
            font_family: "fontmin".into(),
            font_path: "./".into(),
            local: true,
            font_display: "swap".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssFontSource {
    pub file_name: String,
    pub format: OutputFormat,
}

impl CssFontSource {
    #[must_use]
    pub fn new(file_name: impl Into<String>, format: OutputFormat) -> Self {
        Self {
            file_name: file_name.into(),
            format,
        }
    }
}

pub fn generate_font_face_css(sources: &[CssFontSource], options: &CssOptions) -> Result<String> {
    if sources.is_empty() {
        return Err(FontminError::config(
            "CSS generation requires at least one font source",
        ));
    }

    let font_family = css_string(&options.font_family);
    let mut src_parts = Vec::with_capacity(sources.len() + usize::from(options.local));

    if options.local {
        src_parts.push(format!("local('{font_family}')"));
    }

    for source in sources {
        let format = css_format(source.format)?;
        let url = css_string(&join_font_path(&options.font_path, &source.file_name));
        src_parts.push(format!("url('{url}') format('{format}')"));
    }

    Ok(format!(
        "@font-face {{\n  font-family: '{font_family}';\n  src: {};\n  font-weight: normal;\n  font-style: normal;\n  font-display: {};\n}}\n",
        src_parts.join(",\n    "),
        options.font_display,
    ))
}

fn css_format(format: OutputFormat) -> Result<&'static str> {
    match format {
        OutputFormat::Ttf => Ok("truetype"),
        OutputFormat::Woff => Ok("woff"),
        OutputFormat::Woff2 => Ok("woff2"),
        OutputFormat::Eot => Ok("embedded-opentype"),
        OutputFormat::Svg => Ok("svg"),
        OutputFormat::Css => Err(FontminError::config(
            "CSS output cannot be used as a @font-face source",
        )),
    }
}

fn join_font_path(font_path: &str, file_name: &str) -> String {
    if font_path.is_empty() {
        return file_name.into();
    }

    if font_path.ends_with('/') {
        return format!("{font_path}{file_name}");
    }

    format!("{font_path}/{file_name}")
}

fn css_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}
