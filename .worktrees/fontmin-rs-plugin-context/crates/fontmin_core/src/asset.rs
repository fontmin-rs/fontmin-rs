use std::path::PathBuf;

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
    pub fn new(path: PathBuf, contents: Vec<u8>, format: FontFormat) -> Self {
        Self {
            path,
            contents,
            format,
            source_format: format,
            meta: AssetMeta::default(),
        }
    }

    pub fn rename_ext(&mut self, ext: &str) {
        self.path.set_extension(ext.trim_start_matches('.'));
    }
}
