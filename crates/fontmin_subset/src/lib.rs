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
    /// Remove `GDEF`, `GPOS`, and `GSUB`.
    Drop,
    /// Remap supported layout data and discard subtables that no longer match.
    Conservative,
    /// Remap supported layout data and reject known contextual or variation loss.
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
    /// Retain the `cvt `, `fpgm`, and `prep` TrueType program tables.
    pub preserve_hinting: bool,
    /// Subset glyph data; `false` returns the validated source bytes unchanged.
    pub trim: bool,
    /// Retain the original glyph-zero outline instead of an empty required slot.
    pub keep_notdef: bool,
    /// Control OpenType layout-table retention and remapping.
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

        let subset_options = oxifont_subset::SubsetOptions::default()
            .strip_hints(!options.preserve_hinting)
            .retain_layout_tables(options.layout != LayoutSubsetMode::Drop);
        if options.layout == LayoutSubsetMode::Preserve {
            ensure_layout_can_be_preserved(input)?;
        }
        let (output, stats) =
            oxifont_subset::subset_font_with_options(input, &chars, &subset_options)
                .map_err(|error| FontminError::invalid_font(error.to_string()))?;
        if options.layout == LayoutSubsetMode::Preserve {
            ensure_layout_was_preserved(input, &output, stats.dropped_context_subtables)?;
        }

        apply_notdef_policy(output, options.keep_notdef)
    })
}

fn ensure_layout_can_be_preserved(input: &[u8]) -> Result<()> {
    let font = fontmin_ttf::read_ttf(input)?;

    for tag in ["GSUB", "GPOS"] {
        let Some(table) = font.table(tag) else {
            continue;
        };
        if read_u16_at(table, 0, "layout table major version")? == 1
            && read_u16_at(table, 2, "layout table minor version")? >= 1
            && read_u32_at(table, 10, "layout FeatureVariations offset")? != 0
        {
            return Err(FontminError::config(format!(
                "keepLayout preserve cannot retain {tag} FeatureVariations; use conservative or drop"
            )));
        }
    }

    Ok(())
}

fn ensure_layout_was_preserved(
    input: &[u8],
    output: &[u8],
    dropped_context_subtables: usize,
) -> Result<()> {
    if dropped_context_subtables != 0 {
        return Err(FontminError::config(format!(
            "keepLayout preserve could not retain {dropped_context_subtables} contextual layout subtables; use conservative or drop"
        )));
    }

    let input_font = fontmin_ttf::read_ttf(input)?;
    let output_font = fontmin_ttf::read_ttf(output)?;
    for tag in ["GDEF", "GPOS", "GSUB"] {
        if input_font.table(tag).is_some() && output_font.table(tag).is_none() {
            return Err(FontminError::config(format!(
                "keepLayout preserve could not retain the {tag} table; use conservative or drop"
            )));
        }
    }

    Ok(())
}

fn apply_notdef_policy(input: Vec<u8>, keep_notdef: bool) -> Result<Vec<u8>> {
    if keep_notdef {
        return Ok(input);
    }

    let font = fontmin_ttf::read_ttf(&input)?;
    let (empty_glyf, empty_loca) = empty_notdef_outline(&font)?;
    let tables = font
        .tables
        .iter()
        .map(|record| {
            let data = match record.tag.as_str() {
                "glyf" => empty_glyf.clone(),
                "loca" => empty_loca.clone(),
                _ => font
                    .table(&record.tag)
                    .ok_or_else(|| {
                        FontminError::invalid_font(format!(
                            "subset table {} points outside the font",
                            record.tag
                        ))
                    })?
                    .to_vec(),
            };

            Ok(fontmin_ttf::OwnedSfntTable {
                tag: record.tag.clone(),
                data,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    fontmin_ttf::write_ttf(&fontmin_ttf::OwnedTtfFont { tables })
}

fn empty_notdef_outline(font: &fontmin_ttf::TtfFont<'_>) -> Result<(Vec<u8>, Vec<u8>)> {
    let head = required_subset_table(font, "head")?;
    let maxp = required_subset_table(font, "maxp")?;
    let loca = required_subset_table(font, "loca")?;
    let glyf = required_subset_table(font, "glyf")?;
    let index_to_loc_format = read_i16_at(head, 50, "head indexToLocFormat")?;
    let glyph_count = usize::from(read_u16_at(maxp, 4, "maxp numGlyphs")?);
    let entry_count = glyph_count
        .checked_add(1)
        .ok_or_else(|| FontminError::invalid_font("loca entry count overflows"))?;
    let mut offsets = read_loca_offsets(loca, entry_count, index_to_loc_format)?;

    if offsets.len() < 2 {
        return Err(FontminError::invalid_font(
            "subset font does not contain a glyph zero loca entry",
        ));
    }
    if offsets.windows(2).any(|pair| pair[0] > pair[1])
        || offsets.iter().any(|offset| *offset > glyf.len())
    {
        return Err(FontminError::invalid_font(
            "subset font contains invalid loca offsets",
        ));
    }

    let start = offsets[0];
    let end = offsets[1];
    let removed_length = end - start;
    let mut rewritten_glyf = Vec::with_capacity(glyf.len() - removed_length);
    rewritten_glyf.extend_from_slice(&glyf[..start]);
    rewritten_glyf.extend_from_slice(&glyf[end..]);

    for offset in offsets.iter_mut().skip(1) {
        *offset = offset.checked_sub(removed_length).ok_or_else(|| {
            FontminError::invalid_font("subset font contains invalid loca offsets")
        })?;
    }

    let rewritten_loca = write_loca_offsets(loca, &offsets, index_to_loc_format)?;

    Ok((rewritten_glyf, rewritten_loca))
}

fn required_subset_table<'a>(font: &fontmin_ttf::TtfFont<'a>, tag: &str) -> Result<&'a [u8]> {
    font.table(tag)
        .ok_or_else(|| FontminError::invalid_font(format!("subset font is missing {tag} table")))
}

fn read_loca_offsets(input: &[u8], count: usize, format: i16) -> Result<Vec<usize>> {
    let entry_size = match format {
        0 => 2,
        1 => 4,
        _ => {
            return Err(FontminError::invalid_font(format!(
                "unsupported indexToLocFormat {format}"
            )));
        }
    };
    let required_length = count
        .checked_mul(entry_size)
        .ok_or_else(|| FontminError::invalid_font("loca table length overflows"))?;
    if input.len() < required_length {
        return Err(FontminError::invalid_font("loca table is truncated"));
    }

    (0..count)
        .map(|index| {
            let offset = index * entry_size;
            if format == 0 {
                Ok(usize::from(read_u16_at(input, offset, "loca offset")?) * 2)
            } else {
                usize::try_from(read_u32_at(input, offset, "loca offset")?)
                    .map_err(|_| FontminError::invalid_font("loca offset exceeds platform limits"))
            }
        })
        .collect()
}

fn write_loca_offsets(input: &[u8], offsets: &[usize], format: i16) -> Result<Vec<u8>> {
    let mut output = input.to_vec();

    for (index, offset) in offsets.iter().copied().enumerate() {
        if format == 0 {
            if offset % 2 != 0 {
                return Err(FontminError::invalid_font(
                    "short loca offset is not two-byte aligned",
                ));
            }
            let value = u16::try_from(offset / 2)
                .map_err(|_| FontminError::invalid_font("short loca offset is too large"))?;
            output[index * 2..index * 2 + 2].copy_from_slice(&value.to_be_bytes());
        } else {
            let value = u32::try_from(offset)
                .map_err(|_| FontminError::invalid_font("long loca offset is too large"))?;
            output[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
        }
    }

    Ok(output)
}

fn read_u16_at(input: &[u8], offset: usize, field: &str) -> Result<u16> {
    let bytes = input
        .get(offset..offset + 2)
        .ok_or_else(|| FontminError::invalid_font(format!("{field} is truncated")))?;

    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_i16_at(input: &[u8], offset: usize, field: &str) -> Result<i16> {
    let bytes = input
        .get(offset..offset + 2)
        .ok_or_else(|| FontminError::invalid_font(format!("{field} is truncated")))?;

    Ok(i16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u32_at(input: &[u8], offset: usize, field: &str) -> Result<u32> {
    let bytes = input
        .get(offset..offset + 4)
        .ok_or_else(|| FontminError::invalid_font(format!("{field} is truncated")))?;

    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
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
    fontmin_ttf::read_ttf(input)?;

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

    fn glyph_zero_data_length(input: &[u8]) -> usize {
        let font = fontmin_ttf::read_ttf(input).unwrap();
        let head = font.table("head").unwrap();
        let loca = font.table("loca").unwrap();
        let index_to_loc_format = i16::from_be_bytes(head[50..52].try_into().unwrap());

        match index_to_loc_format {
            0 => {
                let start = usize::from(u16::from_be_bytes(loca[0..2].try_into().unwrap())) * 2;
                let end = usize::from(u16::from_be_bytes(loca[2..4].try_into().unwrap())) * 2;

                end - start
            }
            1 => {
                let start =
                    usize::try_from(u32::from_be_bytes(loca[0..4].try_into().unwrap())).unwrap();
                let end =
                    usize::try_from(u32::from_be_bytes(loca[4..8].try_into().unwrap())).unwrap();

                end - start
            }
            _ => panic!("unsupported indexToLocFormat {index_to_loc_format}"),
        }
    }

    fn with_gsub_feature_variations(input: &[u8]) -> Vec<u8> {
        let font = fontmin_ttf::read_ttf(input).unwrap();
        let tables = font
            .tables
            .iter()
            .map(|record| {
                let mut data = font.table(&record.tag).unwrap().to_vec();
                if record.tag == "GSUB" {
                    data[0..4].copy_from_slice(&0x0001_0001_u32.to_be_bytes());
                    data[10..14].copy_from_slice(&14_u32.to_be_bytes());
                }

                fontmin_ttf::OwnedSfntTable {
                    tag: record.tag.clone(),
                    data,
                }
            })
            .collect();

        fontmin_ttf::write_ttf(&fontmin_ttf::OwnedTtfFont { tables }).unwrap()
    }

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
    fn preserve_hinting_controls_hint_program_tables() {
        let without_hinting = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                preserve_hinting: false,
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let with_hinting = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                preserve_hinting: true,
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let without_hinting_tables = fontmin_ttf::inspect_ttf(&without_hinting).unwrap().tables;
        let with_hinting_tables = fontmin_ttf::inspect_ttf(&with_hinting).unwrap().tables;

        for tag in ["cvt ", "fpgm", "prep"] {
            assert!(
                !without_hinting_tables.iter().any(|table| table == tag),
                "{tag} should be removed when hinting is not preserved",
            );
            assert!(
                with_hinting_tables.iter().any(|table| table == tag),
                "{tag} should remain when hinting is preserved",
            );
        }
    }

    #[test]
    fn keep_notdef_controls_glyph_zero_outline() {
        let without_notdef = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                keep_notdef: false,
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let with_notdef = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                keep_notdef: true,
                ..SubsetOptions::default()
            },
        )
        .unwrap();

        assert_eq!(glyph_zero_data_length(&without_notdef), 0);
        assert!(glyph_zero_data_length(&with_notdef) > 0);
    }

    #[test]
    fn layout_modes_control_layout_table_retention() {
        let subset_with_layout = |layout| {
            subset_ttf(
                ROBOTO,
                SubsetOptions {
                    text: Some("Hello".into()),
                    layout,
                    ..SubsetOptions::default()
                },
            )
        };
        let dropped = subset_with_layout(LayoutSubsetMode::Drop).unwrap();
        let conservative = subset_with_layout(LayoutSubsetMode::Conservative).unwrap();
        let preserve_error = subset_with_layout(LayoutSubsetMode::Preserve).unwrap_err();
        let dropped_tables = fontmin_ttf::inspect_ttf(&dropped).unwrap().tables;
        let conservative_tables = fontmin_ttf::inspect_ttf(&conservative).unwrap().tables;

        for tag in ["GDEF", "GPOS", "GSUB"] {
            assert!(!dropped_tables.iter().any(|table| table == tag));
            assert!(conservative_tables.iter().any(|table| table == tag));
        }
        assert!(
            preserve_error
                .to_string()
                .contains("keepLayout preserve could not retain 31 contextual layout subtables")
        );
    }

    #[test]
    fn preserve_layout_rejects_feature_variations_before_remapping() {
        let input = with_gsub_feature_variations(ROBOTO);
        let error = subset_ttf(
            &input,
            SubsetOptions {
                text: Some("Hello".into()),
                layout: LayoutSubsetMode::Preserve,
                ..SubsetOptions::default()
            },
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("keepLayout preserve cannot retain GSUB FeatureVariations")
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
