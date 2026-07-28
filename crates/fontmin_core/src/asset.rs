use std::path::PathBuf;

use fontmin_diagnostics::{FontminError, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{FontFormat, UnicodeRange};

#[derive(Debug, Clone)]
pub struct Asset {
    pub path: PathBuf,
    pub contents: Vec<u8>,
    pub format: FontFormat,
    pub source_format: FontFormat,
    pub meta: AssetMeta,
}

#[derive(Debug, Clone, Default)]
pub struct AssetMeta {
    pub font_family: Option<String>,
    pub glyph_count: Option<u32>,
    pub subset_count: Option<u32>,
    pub generated_by: Vec<String>,
    pub unicode: Option<u32>,
    pub css_glyphs: Vec<CssGlyph>,
    pub css_unicode_ranges: Vec<UnicodeRange>,
    pub custom: IndexMap<String, serde_json::Value>,
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

impl Asset {
    #[must_use]
    pub fn new(path: PathBuf, contents: Vec<u8>, format: FontFormat) -> Self {
        Self {
            path,
            contents,
            format,
            source_format: format,
            meta: AssetMeta::default(),
        }
    }

    pub fn rename_ext(&mut self, ext: &str) -> Result<()> {
        let ext = ext.trim_start_matches('.');

        if ext.is_empty() || ext == "." || ext == ".." || ext.contains('/') || ext.contains('\\') {
            return Err(FontminError::config(format!(
                "output extension must be a file extension, got `{ext}`"
            )));
        }

        self.path.set_extension(ext);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Asset, CssGlyph, FontFormat};

    #[test]
    fn rejects_path_components_in_extensions() {
        let mut asset = Asset::new("font.ttf".into(), Vec::new(), FontFormat::Ttf);

        assert!(asset.rename_ext("../escaped").is_err());
        assert_eq!(asset.path, std::path::Path::new("font.ttf"));
    }

    #[test]
    fn keeps_typed_metadata_separate_from_plugin_extensions() {
        let mut asset = Asset::new("font.ttf".into(), Vec::new(), FontFormat::Ttf);

        asset.meta.unicode = Some(0xE001);
        asset.meta.css_glyphs = vec![CssGlyph::new(Some("home".into()), 0xE001)];
        asset
            .meta
            .css_unicode_ranges
            .push("U+E000-E0FF".parse().unwrap());
        asset.meta.custom.insert("vendor".into(), json!("value"));

        assert_eq!(asset.meta.unicode, Some(0xE001));
        assert_eq!(asset.meta.css_glyphs[0].name.as_deref(), Some("home"));
        assert_eq!(asset.meta.css_unicode_ranges[0].to_string(), "U+E000-E0FF");
        assert_eq!(asset.meta.custom["vendor"], "value");
    }
}
