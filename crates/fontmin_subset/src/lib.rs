use font_subset::{Font, FontReader};
use std::collections::BTreeSet;

use fontmin_core::{
    CoverageOptions, CoverageReport, MissingGlyphPolicy, UnicodeRange, collect_chars_with_ranges,
};
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
#[serde(default, rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct SubsetOptions {
    pub text: Option<String>,
    pub unicodes: Vec<u32>,
    pub unicode_ranges: Vec<UnicodeRange>,
    pub basic_text: bool,
    pub preserve_hinting: bool,
    pub trim: bool,
    pub keep_notdef: bool,
    pub layout: LayoutSubsetMode,
    pub missing_glyphs: MissingGlyphPolicy,
}

impl Default for SubsetOptions {
    fn default() -> Self {
        Self {
            text: None,
            unicodes: Vec::new(),
            unicode_ranges: Vec::new(),
            basic_text: false,
            preserve_hinting: false,
            trim: true,
            keep_notdef: true,
            layout: LayoutSubsetMode::Conservative,
            missing_glyphs: MissingGlyphPolicy::Warn,
        }
    }
}

impl From<&SubsetOptions> for CoverageOptions {
    fn from(options: &SubsetOptions) -> Self {
        Self {
            text: options.text.clone(),
            unicodes: options.unicodes.clone(),
            unicode_ranges: options.unicode_ranges.clone(),
            basic_text: options.basic_text,
        }
    }
}

pub fn analyze_ttf_coverage(input: &[u8], options: &CoverageOptions) -> Result<CoverageReport> {
    let requested = collect_requested(options, "coverage")?;

    with_font(input, |font| {
        let (_, report) = partition_coverage(font, &requested);

        Ok(report)
    })
}

impl SubsetOptions {
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn subset_ttf(input: &[u8], options: SubsetOptions) -> Result<Vec<u8>> {
    let requested = collect_requested(&CoverageOptions::from(&options), "subset")?;

    with_font(input, |font| {
        let (chars, coverage) = partition_coverage(font, &requested);

        if options.missing_glyphs == MissingGlyphPolicy::Error {
            coverage.ensure_complete()?;
        }

        if chars.is_empty() {
            return Err(FontminError::config(
                "subset request has no characters supported by the input font",
            ));
        }

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
    })
}

fn collect_requested(options: &CoverageOptions, operation: &str) -> Result<BTreeSet<char>> {
    let chars = collect_chars_with_ranges(
        options.text.as_deref(),
        &options.unicodes,
        options.basic_text,
        &options.unicode_ranges,
    )?;

    if chars.is_empty() {
        return Err(FontminError::config(format!(
            "{operation} requires at least one character from text, unicodes, Unicode ranges, or basicText"
        )));
    }

    Ok(chars)
}

fn with_font<T>(input: &[u8], operation: impl FnOnce(&Font<'_>) -> Result<T>) -> Result<T> {
    let reader = FontReader::new(input)
        .map_err(|error| FontminError::invalid_font(format!("invalid font data: {error}")))?;
    let font = reader
        .read()
        .map_err(|error| FontminError::invalid_font(format!("invalid font data: {error}")))?;

    operation(&font)
}

fn partition_coverage(
    font: &Font<'_>,
    requested: &BTreeSet<char>,
) -> (BTreeSet<char>, CoverageReport) {
    let supported = requested
        .iter()
        .copied()
        .filter(|character| font.contains_char(*character))
        .collect::<BTreeSet<_>>();
    let missing = requested
        .difference(&supported)
        .copied()
        .map(u32::from)
        .collect::<Vec<_>>();
    let report = CoverageReport::new(
        requested.iter().copied().map(u32::from).collect(),
        supported.iter().copied().map(u32::from).collect(),
        missing,
    );

    (supported, report)
}

#[cfg(test)]
mod tests {
    use fontmin_testing::ROBOTO;

    use fontmin_core::{CoverageOptions, MissingGlyphPolicy};
    use fontmin_diagnostics::FontminErrorKind;

    use super::{LayoutSubsetMode, SubsetOptions, analyze_ttf_coverage, subset_ttf};

    #[test]
    fn subsets_ttf_to_a_smaller_valid_opentype_buffer() {
        let output = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                unicodes: Vec::new(),
                unicode_ranges: Vec::new(),
                basic_text: false,
                preserve_hinting: false,
                trim: true,
                keep_notdef: true,
                layout: LayoutSubsetMode::Conservative,
                missing_glyphs: MissingGlyphPolicy::Warn,
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
    fn subsets_ttf_from_unicode_ranges() {
        let output = subset_ttf(
            ROBOTO,
            SubsetOptions {
                unicode_ranges: vec!["U+0041-0042".parse().unwrap()],
                ..SubsetOptions::default()
            },
        )
        .unwrap();

        assert!(output.len() < ROBOTO.len());
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
        assert_eq!(output.as_slice(), ROBOTO);
    }

    #[test]
    fn rejects_empty_subset_requests() {
        let error = subset_ttf(ROBOTO, SubsetOptions::default()).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("subset requires at least one character")
        );
    }

    #[test]
    fn rejects_invalid_font_data() {
        let error = subset_ttf(b"not a font", SubsetOptions::with_text("Hello")).unwrap_err();

        assert!(error.to_string().contains("invalid font data"));
    }

    #[test]
    fn reports_supported_and_missing_requested_codepoints() {
        let report = analyze_ttf_coverage(
            ROBOTO,
            &CoverageOptions {
                text: Some("A𠮷".into()),
                ..CoverageOptions::default()
            },
        )
        .unwrap();

        assert_eq!(report.requested, vec![0x41, 0x20bb7]);
        assert_eq!(report.supported, vec![0x41]);
        assert_eq!(report.missing, vec![0x20bb7]);
        assert!((report.coverage_percent - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn strict_missing_glyph_policy_rejects_partial_coverage() {
        let error = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("A𠮷".into()),
                missing_glyphs: MissingGlyphPolicy::Error,
                ..SubsetOptions::default()
            },
        )
        .unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::MissingGlyph);
        assert!(error.to_string().contains("U+20BB7"));
    }

    #[test]
    fn strict_missing_glyph_policy_reports_fully_missing_coverage() {
        let error = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("𠮷".into()),
                missing_glyphs: MissingGlyphPolicy::Error,
                ..SubsetOptions::default()
            },
        )
        .unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::MissingGlyph);
        assert!(error.to_string().contains("U+20BB7"));
    }

    #[test]
    fn warning_policy_keeps_supported_characters() {
        let output = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("A𠮷".into()),
                ..SubsetOptions::default()
            },
        )
        .unwrap();

        assert!(output.len() < ROBOTO.len());
    }
}
