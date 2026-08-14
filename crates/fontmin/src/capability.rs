use std::borrow::Cow;

use fontmin_core::FontFormat;
use fontmin_diagnostics::{FontminError, Result};
use fontmin_ttf::{SfntFlavor, SfntFont, read_sfnt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilitySupport {
    Subset,
    Passthrough,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorFontTechnology {
    ColrCpal,
    CbdtCblc,
    Sbix,
    Svg,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorFontTechnologyCapability {
    pub technology: ColorFontTechnology,
    pub tables: Vec<String>,
    pub subset_support: CapabilitySupport,
    pub version: Option<u16>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorFontCapabilityReport {
    pub is_color_font: bool,
    pub subset_support: Option<CapabilitySupport>,
    pub technologies: Vec<ColorFontTechnologyCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontCapabilityReport {
    pub format: FontFormat,
    pub color: ColorFontCapabilityReport,
}

pub fn inspect_capabilities(input: &[u8]) -> Result<FontCapabilityReport> {
    let format = fontmin_detect::detect_format(input);
    let sfnt = decode_sfnt(input, format)?;
    let flavor = if sfnt.starts_with(b"OTTO") {
        SfntFlavor::OpenTypeCff
    } else {
        SfntFlavor::TrueType
    };
    let font = read_sfnt(&sfnt, flavor)?;
    let technologies = color_capabilities(&font)?;
    let subset_support = technologies
        .iter()
        .map(|capability| capability.subset_support)
        .max_by_key(|support| support_priority(*support));

    Ok(FontCapabilityReport {
        format,
        color: ColorFontCapabilityReport {
            is_color_font: !technologies.is_empty(),
            subset_support,
            technologies,
        },
    })
}

fn decode_sfnt(input: &[u8], format: FontFormat) -> Result<Cow<'_, [u8]>> {
    match format {
        FontFormat::Ttf | FontFormat::Otf => Ok(Cow::Borrowed(input)),
        FontFormat::Woff => fontmin_woff::decode_woff_to_ttf(input).map(Cow::Owned),
        FontFormat::Woff2 => fontmin_woff2::decode_woff2_to_ttf(input).map(Cow::Owned),
        FontFormat::Eot => fontmin_eot::decode_eot_to_ttf(input).map(Cow::Owned),
        FontFormat::Svg => Err(FontminError::unsupported(
            "OpenType color capabilities for SVG font input",
        )),
        FontFormat::Css => Err(FontminError::unsupported("color capabilities for CSS")),
        FontFormat::Unknown if input.starts_with(b"ttcf") => Err(FontminError::unsupported(
            "color capabilities for a collection; extract a face first",
        )),
        FontFormat::Unknown => Err(FontminError::invalid_font("unknown font format")),
    }
}

fn color_capabilities(font: &SfntFont<'_>) -> Result<Vec<ColorFontTechnologyCapability>> {
    let mut capabilities = Vec::new();
    let colr = font.table("COLR");
    let cpal = font.table("CPAL");
    if colr.is_some() || cpal.is_some() {
        capabilities.push(colr_capability(colr, cpal)?);
    }

    let cbdt = font.table("CBDT");
    let cblc = font.table("CBLC");
    if cbdt.is_some() || cblc.is_some() {
        capabilities.push(paired_capability(
            ColorFontTechnology::CbdtCblc,
            "CBDT",
            cbdt,
            "CBLC",
            cblc,
        ));
    }

    if font.table("sbix").is_some() {
        capabilities.push(ColorFontTechnologyCapability {
            technology: ColorFontTechnology::Sbix,
            tables: vec!["sbix".into()],
            subset_support: CapabilitySupport::Subset,
            version: None,
            detail: "glyph bitmap offsets are rewritten during subsetting".into(),
        });
    }
    if font.table("SVG ").is_some() {
        capabilities.push(ColorFontTechnologyCapability {
            technology: ColorFontTechnology::Svg,
            tables: vec!["SVG ".into()],
            subset_support: CapabilitySupport::Subset,
            version: None,
            detail: "SVG document glyph ranges are rewritten during subsetting".into(),
        });
    }

    Ok(capabilities)
}

fn colr_capability(
    colr: Option<&[u8]>,
    cpal: Option<&[u8]>,
) -> Result<ColorFontTechnologyCapability> {
    let tables = [colr.map(|_| "COLR"), cpal.map(|_| "CPAL")]
        .into_iter()
        .flatten()
        .map(str::to_owned)
        .collect();
    let Some(colr) = colr else {
        return Ok(ColorFontTechnologyCapability {
            technology: ColorFontTechnology::ColrCpal,
            tables,
            subset_support: CapabilitySupport::Unsupported,
            version: None,
            detail: "CPAL is present without the required COLR table".into(),
        });
    };
    if cpal.is_none() {
        return Ok(ColorFontTechnologyCapability {
            technology: ColorFontTechnology::ColrCpal,
            tables,
            subset_support: CapabilitySupport::Unsupported,
            version: None,
            detail: "COLR is present without the required CPAL table".into(),
        });
    }
    let version = u16::from_be_bytes(
        colr.get(..2)
            .ok_or_else(|| FontminError::invalid_font("COLR table is truncated"))?
            .try_into()
            .map_err(|_| FontminError::invalid_font("COLR table is truncated"))?,
    );
    let (subset_support, detail) = match version {
        0 => (
            CapabilitySupport::Subset,
            "COLR v0 glyph and layer references are rewritten during subsetting",
        ),
        1 => (
            CapabilitySupport::Passthrough,
            "COLR v1 paint graphs are retained verbatim; use retained GIDs for safe output",
        ),
        _ => (
            CapabilitySupport::Unsupported,
            "the COLR table version is not supported",
        ),
    };

    Ok(ColorFontTechnologyCapability {
        technology: ColorFontTechnology::ColrCpal,
        tables,
        subset_support,
        version: Some(version),
        detail: detail.into(),
    })
}

fn paired_capability(
    technology: ColorFontTechnology,
    first_tag: &str,
    first: Option<&[u8]>,
    second_tag: &str,
    second: Option<&[u8]>,
) -> ColorFontTechnologyCapability {
    let tables = [first.map(|_| first_tag), second.map(|_| second_tag)]
        .into_iter()
        .flatten()
        .map(str::to_owned)
        .collect();
    let complete = first.is_some() && second.is_some();

    ColorFontTechnologyCapability {
        technology,
        tables,
        subset_support: if complete {
            CapabilitySupport::Subset
        } else {
            CapabilitySupport::Unsupported
        },
        version: None,
        detail: if complete {
            "bitmap glyph data and location records are rewritten during subsetting".into()
        } else {
            format!("{first_tag} and {second_tag} must be present together")
        },
    }
}

const fn support_priority(support: CapabilitySupport) -> u8 {
    match support {
        CapabilitySupport::Subset => 0,
        CapabilitySupport::Passthrough => 1,
        CapabilitySupport::Unsupported => 2,
    }
}

#[cfg(test)]
mod tests {
    use fontmin_testing::ROBOTO;

    use super::{CapabilitySupport, ColorFontTechnology, inspect_capabilities};

    fn with_tables(tables: &[(&str, &str)], colr_version: Option<u16>) -> Vec<u8> {
        let mut font = ROBOTO.to_vec();
        let table_count = usize::from(u16::from_be_bytes([font[4], font[5]]));

        for (source, target) in tables {
            let record = (0..table_count)
                .map(|index| 12 + index * 16)
                .find(|record| &font[*record..*record + 4] == source.as_bytes())
                .expect("source test table is present");
            font[record..record + 4].copy_from_slice(target.as_bytes());
            if target == &"COLR" {
                let offset = usize::try_from(u32::from_be_bytes(
                    font[record + 8..record + 12].try_into().unwrap(),
                ))
                .unwrap();
                font[offset..offset + 2]
                    .copy_from_slice(&colr_version.unwrap_or_default().to_be_bytes());
            }
        }

        font
    }

    #[test]
    fn reports_fonts_without_color_tables() {
        let report = inspect_capabilities(ROBOTO).unwrap();

        assert!(!report.color.is_color_font);
        assert_eq!(report.color.subset_support, None);
        assert!(report.color.technologies.is_empty());
    }

    #[test]
    fn distinguishes_colr_subset_passthrough_and_unsupported_versions() {
        for (version, expected) in [
            (0, CapabilitySupport::Subset),
            (1, CapabilitySupport::Passthrough),
            (2, CapabilitySupport::Unsupported),
        ] {
            let font = with_tables(&[("cvt ", "COLR"), ("fpgm", "CPAL")], Some(version));
            let report = inspect_capabilities(&font).unwrap();
            let capability = &report.color.technologies[0];

            assert_eq!(capability.technology, ColorFontTechnology::ColrCpal);
            assert_eq!(capability.version, Some(version));
            assert_eq!(capability.subset_support, expected);
            assert_eq!(report.color.subset_support, Some(expected));
        }
    }

    #[test]
    fn reports_rewritten_and_incomplete_color_tables() {
        let complete = with_tables(&[("cvt ", "CBDT"), ("fpgm", "CBLC")], None);
        let incomplete = with_tables(&[("cvt ", "CBDT")], None);
        let complete = inspect_capabilities(&complete).unwrap();
        let incomplete = inspect_capabilities(&incomplete).unwrap();

        assert_eq!(
            complete.color.technologies[0].subset_support,
            CapabilitySupport::Subset
        );
        assert_eq!(
            incomplete.color.technologies[0].subset_support,
            CapabilitySupport::Unsupported
        );
    }
}
