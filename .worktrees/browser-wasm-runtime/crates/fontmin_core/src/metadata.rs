use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontMetadata {
    pub family_name: Option<String>,
    pub subfamily_name: Option<String>,
    pub full_name: Option<String>,
    pub post_script_name: Option<String>,
    pub glyph_count: u16,
    pub units_per_em: u16,
    pub ascender: i16,
    pub descender: i16,
    pub tables: Vec<String>,
}
