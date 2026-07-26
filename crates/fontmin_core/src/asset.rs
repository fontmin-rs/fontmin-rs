use std::path::PathBuf;

use fontmin_diagnostics::{FontminError, Result};
use indexmap::IndexMap;

use crate::FontFormat;

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
    pub custom: IndexMap<String, serde_json::Value>,
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
    use super::{Asset, FontFormat};

    #[test]
    fn rejects_path_components_in_extensions() {
        let mut asset = Asset::new("font.ttf".into(), Vec::new(), FontFormat::Ttf);

        assert!(asset.rename_ext("../escaped").is_err());
        assert_eq!(asset.path, std::path::Path::new("font.ttf"));
    }
}
