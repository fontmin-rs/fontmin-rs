use font_subset::{Font, FontReader};
use fontmin_core::collect_chars;
use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutSubsetMode {
    Drop,
    Conservative,
    Preserve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsetOptions {
    pub text: Option<String>,
    pub unicodes: Vec<u32>,
    pub basic_text: bool,
    pub preserve_hinting: bool,
    pub trim: bool,
    pub keep_notdef: bool,
    pub layout: LayoutSubsetMode,
}

impl Default for SubsetOptions {
    fn default() -> Self {
        Self {
            text: None,
            unicodes: Vec::new(),
            basic_text: false,
            preserve_hinting: false,
            trim: true,
            keep_notdef: true,
            layout: LayoutSubsetMode::Conservative,
        }
    }
}

impl SubsetOptions {
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }
}

pub fn subset_ttf(input: &[u8], options: SubsetOptions) -> Result<Vec<u8>> {
    let chars = collect_chars(
        options.text.as_deref(),
        &options.unicodes,
        options.basic_text,
    )?;

    if chars.is_empty() {
        return Err(FontminError::config(
            "subset requires at least one character from text, unicodes, or basicText",
        ));
    }

    let reader = FontReader::new(input)
        .map_err(|error| FontminError::invalid_font(format!("invalid font data: {error}")))?;
    let font: Font<'_> = reader
        .read()
        .map_err(|error| FontminError::invalid_font(format!("invalid font data: {error}")))?;

    let permissions = font.permissions();
    if !permissions.allow_subsetting {
        return Err(FontminError::invalid_font(
            "font license does not allow subsetting",
        ));
    }

    if !options.trim {
        return Ok(input.to_vec());
    }

    let subset = font
        .subset(&chars)
        .map_err(|error| FontminError::invalid_font(format!("invalid font data: {error}")))?;

    Ok(subset.to_opentype())
}

#[cfg(test)]
mod tests {
    use fontmin_testing::ROBOTO;

    use super::{LayoutSubsetMode, SubsetOptions, subset_ttf};

    #[test]
    fn subsets_ttf_to_a_smaller_valid_opentype_buffer() {
        let output = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                unicodes: Vec::new(),
                basic_text: false,
                preserve_hinting: false,
                trim: true,
                keep_notdef: true,
                layout: LayoutSubsetMode::Conservative,
            },
        )
        .unwrap();

        assert!(output.len() < ROBOTO.len());
        assert!(
            output.starts_with(&[0x00, 0x01, 0x00, 0x00]) || output.starts_with(b"OTTO"),
            "subset output must remain OpenType data",
        );
    }

    #[test]
    fn trim_false_keeps_original_font_data() {
        let output = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                trim: false,
                ..SubsetOptions::default()
            },
        )
        .unwrap();

        assert_eq!(output.len(), ROBOTO.len());
        assert!(output == ROBOTO);
    }

    #[test]
    fn rejects_empty_subset_requests() {
        let error = subset_ttf(ROBOTO, SubsetOptions::default()).unwrap_err();

        assert!(error.to_string().contains("at least one character"));
    }

    #[test]
    fn rejects_invalid_font_data() {
        let error = subset_ttf(b"not a font", SubsetOptions::with_text("Hello")).unwrap_err();

        assert!(error.to_string().contains("invalid font data"));
    }
}
