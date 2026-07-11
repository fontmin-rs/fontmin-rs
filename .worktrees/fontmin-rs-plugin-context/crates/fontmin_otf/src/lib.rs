use fontmin_core::FontMetadata;
use fontmin_diagnostics::{FontminError, Result};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Otf2TtfOptions {
    pub preserve_hinting: bool,
}

pub fn inspect_otf(input: &[u8]) -> Result<FontMetadata> {
    fontmin_ttf::inspect_sfnt(input, fontmin_ttf::SfntFlavor::OpenTypeCff)
}

pub fn otf_to_ttf(_input: &[u8], _options: &Otf2TtfOptions) -> Result<Vec<u8>> {
    Err(FontminError::unsupported("otf to ttf"))
}
