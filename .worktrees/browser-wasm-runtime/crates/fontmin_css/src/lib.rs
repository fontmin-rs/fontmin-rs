use std::{collections::BTreeSet, fmt::Write as _};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use fontmin_core::OutputFormat;
use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssOptions {
    pub font_family: String,
    pub font_path: String,
    pub base64: bool,
    pub glyph: bool,
    pub icon_prefix: String,
    pub as_file_name: bool,
    pub local: bool,
    pub font_display: String,
    pub target: CssTarget,
}

impl Default for CssOptions {
    fn default() -> Self {
        Self {
            font_family: "fontmin".into(),
            font_path: "./".into(),
            base64: false,
            glyph: false,
            icon_prefix: "icon".into(),
            as_file_name: false,
            local: true,
            font_display: "swap".into(),
            target: CssTarget::Css,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CssTarget {
    #[default]
    Css,
    Scss,
    Less,
}

impl CssTarget {
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Css => "css",
            Self::Scss => "scss",
            Self::Less => "less",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssGlyph {
    pub name: Option<String>,
    pub unicode: u32,
}

impl CssGlyph {
    #[must_use]
    pub fn new(name: Option<String>, unicode: u32) -> Self {
        Self { name, unicode }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssFontSource {
    pub file_name: String,
    pub format: OutputFormat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contents: Option<Vec<u8>>,
    #[serde(default)]
    pub glyphs: Vec<CssGlyph>,
}

impl CssFontSource {
    #[must_use]
    pub fn new(file_name: impl Into<String>, format: OutputFormat) -> Self {
        Self {
            file_name: file_name.into(),
            format,
            contents: None,
            glyphs: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_contents(mut self, contents: impl Into<Vec<u8>>) -> Self {
        self.contents = Some(contents.into());

        self
    }

    #[must_use]
    pub fn with_glyphs(mut self, glyphs: Vec<CssGlyph>) -> Self {
        self.glyphs = glyphs;

        self
    }
}

#[must_use]
pub fn css_glyphs_from_text(text: &str, unicodes: &[u32]) -> Vec<CssGlyph> {
    let mut seen = BTreeSet::new();
    let mut glyphs = Vec::new();

    for character in text.chars() {
        let unicode = u32::from(character);

        if seen.insert(unicode) {
            glyphs.push(CssGlyph::new(None, unicode));
        }
    }

    for unicode in unicodes {
        if seen.insert(*unicode) {
            glyphs.push(CssGlyph::new(None, *unicode));
        }
    }

    glyphs
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
        let url = css_string(&source_url(source, options)?);
        src_parts.push(format!("url('{url}') format('{format}')"));
    }

    let mut css = format!(
        "@font-face {{\n  font-family: '{font_family}';\n  src: {};\n  font-weight: normal;\n  font-style: normal;\n  font-display: {};\n}}\n",
        src_parts.join(",\n    "),
        options.font_display,
    );

    if options.glyph {
        append_glyph_css(&mut css, sources, options, &font_family);
    }

    Ok(css)
}

fn append_glyph_css(
    css: &mut String,
    sources: &[CssFontSource],
    options: &CssOptions,
    font_family: &str,
) {
    let glyphs = unique_glyphs(sources);

    if glyphs.is_empty() {
        return;
    }

    let prefix = css_class_part(&options.icon_prefix, "icon");

    write!(
        css,
        ".{prefix} {{\n  font-family: '{font_family}';\n  font-style: normal;\n  font-weight: normal;\n}}\n"
    )
    .expect("writing to string should not fail");

    for glyph in glyphs {
        let fallback = unicode_class_name(glyph.unicode);
        let class_name = if options.as_file_name {
            glyph
                .name
                .as_deref()
                .map_or_else(|| fallback.clone(), |name| css_class_part(name, &fallback))
        } else {
            fallback
        };
        let escape = unicode_escape(glyph.unicode);

        write!(
            css,
            ".{prefix}-{class_name}::before {{\n  content: '\\{escape}';\n}}\n"
        )
        .expect("writing to string should not fail");
    }
}

fn unique_glyphs(sources: &[CssFontSource]) -> Vec<CssGlyph> {
    let mut seen = BTreeSet::new();
    let mut glyphs = Vec::new();

    for source in sources {
        for glyph in &source.glyphs {
            let key = (glyph.name.clone(), glyph.unicode);

            if seen.insert(key) {
                glyphs.push(glyph.clone());
            }
        }
    }

    glyphs
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

fn source_url(source: &CssFontSource, options: &CssOptions) -> Result<String> {
    if options.base64 {
        return data_url(source);
    }

    Ok(join_font_path(&options.font_path, &source.file_name))
}

fn data_url(source: &CssFontSource) -> Result<String> {
    let Some(contents) = source.contents.as_deref() else {
        return Err(FontminError::config(format!(
            "CSS base64 generation requires contents for {}",
            source.file_name
        )));
    };
    let encoded = BASE64.encode(contents);

    Ok(format!(
        "data:{};base64,{encoded}",
        mime_type(source.format)
    ))
}

fn mime_type(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Ttf => "font/ttf",
        OutputFormat::Woff => "font/woff",
        OutputFormat::Woff2 => "font/woff2",
        OutputFormat::Eot => "application/vnd.ms-fontobject",
        OutputFormat::Svg => "image/svg+xml",
        OutputFormat::Css => "text/css",
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

fn css_class_part(value: &str, fallback: &str) -> String {
    let mut class_name = String::new();
    let mut previous_dash = false;

    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
            class_name.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            class_name.push('-');
            previous_dash = true;
        }
    }

    let class_name = class_name.trim_matches('-');

    if class_name.is_empty() {
        fallback.into()
    } else {
        class_name.into()
    }
}

fn unicode_class_name(unicode: u32) -> String {
    format!("u{}", unicode_escape(unicode))
}

fn unicode_escape(unicode: u32) -> String {
    if unicode <= 0xFFFF {
        format!("{unicode:04X}")
    } else {
        format!("{unicode:06X}")
    }
}
