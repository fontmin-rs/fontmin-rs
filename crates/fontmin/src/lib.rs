pub use fontmin_config::FontminConfig;
pub use fontmin_core::{
    Asset, FontDeliverySlice, FontFormat, FontMetadata, OutputFormat, UnicodeRange,
    validate_delivery_slices,
};
pub use fontmin_css::{CssFontSource, CssGlyph, CssOptions, CssTarget};
pub use fontmin_diagnostics::{FontminError, Result};
pub use fontmin_eot::EotOptions;
pub use fontmin_otf::Otf2TtfOptions;
pub use fontmin_plugins::{
    CssPlugin, GlyphPlugin, Otf2TtfPlugin, SlicePlugin, Svg2TtfPlugin, Svgs2TtfPlugin,
    Ttf2EotPlugin, Ttf2SvgPlugin, Ttf2Woff2Plugin, Ttf2WoffPlugin,
};
pub use fontmin_subset::{LayoutSubsetMode, SubsetOptions};
pub use fontmin_svg::{Svg2TtfOptions, SvgIcon, Svgs2TtfOptions, Ttf2SvgOptions};
pub use fontmin_woff::WoffOptions;
pub use fontmin_woff2::Woff2Options;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontInfo {
    pub format: FontFormat,
    pub size: usize,
    pub metadata: FontMetadata,
}

pub fn subset_ttf(input: &[u8], options: SubsetOptions) -> Result<Vec<u8>> {
    fontmin_subset::subset_ttf(input, options)
}

pub fn ttf_to_woff(input: &[u8], options: &WoffOptions) -> Result<Vec<u8>> {
    fontmin_woff::encode_ttf_to_woff(input, options)
}

pub fn woff_to_ttf(input: &[u8]) -> Result<Vec<u8>> {
    fontmin_woff::decode_woff_to_ttf(input)
}

pub fn ttf_to_woff2(input: &[u8], options: &Woff2Options) -> Result<Vec<u8>> {
    fontmin_woff2::encode_ttf_to_woff2(input, options)
}

pub fn woff2_to_ttf(input: &[u8]) -> Result<Vec<u8>> {
    fontmin_woff2::decode_woff2_to_ttf(input)
}

pub fn validate_woff2(input: &[u8]) -> Result<()> {
    fontmin_woff2::validate_woff2(input)
}

pub fn ttf_to_eot(input: &[u8], options: &EotOptions) -> Result<Vec<u8>> {
    fontmin_eot::encode_ttf_to_eot(input, options)
}

pub fn ttf_to_svg(input: &[u8], options: &Ttf2SvgOptions) -> Result<String> {
    fontmin_svg::ttf_to_svg(input, options)
}

pub fn svg_font_to_ttf(input: &str, options: &Svg2TtfOptions) -> Result<Vec<u8>> {
    fontmin_svg::svg_font_to_ttf(input, options)
}

pub fn svgs_to_ttf(inputs: Vec<SvgIcon>, options: &Svgs2TtfOptions) -> Result<Vec<u8>> {
    fontmin_svg::svgs_to_ttf(inputs, options)
}

pub fn eot_to_ttf(input: &[u8]) -> Result<Vec<u8>> {
    fontmin_eot::decode_eot_to_ttf(input)
}

pub fn otf_to_ttf(input: &[u8], options: &Otf2TtfOptions) -> Result<Vec<u8>> {
    fontmin_otf::otf_to_ttf(input, options)
}

pub fn generate_font_face_css(sources: &[CssFontSource], options: &CssOptions) -> Result<String> {
    fontmin_css::generate_font_face_css(sources, options)
}

pub fn inspect(input: &[u8]) -> Result<FontInfo> {
    let format = fontmin_detect::detect_format(input);

    let metadata = match format {
        FontFormat::Ttf => fontmin_ttf::inspect_ttf(input)?,
        FontFormat::Woff => {
            let ttf = fontmin_woff::decode_woff_to_ttf(input)?;

            fontmin_ttf::inspect_ttf(&ttf)?
        }
        FontFormat::Eot => {
            let ttf = fontmin_eot::decode_eot_to_ttf(input)?;

            fontmin_ttf::inspect_ttf(&ttf)?
        }
        FontFormat::Otf => fontmin_otf::inspect_otf(input)?,
        FontFormat::Woff2 => fontmin_woff2::inspect_woff2(input)?,
        FontFormat::Svg => return Err(FontminError::unsupported("svg")),
        FontFormat::Css => return Err(FontminError::unsupported("css")),
        FontFormat::Unknown => return Err(FontminError::invalid_font("unknown font format")),
    };

    Ok(FontInfo {
        format,
        size: input.len(),
        metadata,
    })
}

pub fn convert(input: &[u8], target: OutputFormat) -> Result<Vec<u8>> {
    convert_with_options(input, target, &Otf2TtfOptions::default())
}

pub fn convert_with_options(
    input: &[u8],
    target: OutputFormat,
    otf_options: &Otf2TtfOptions,
) -> Result<Vec<u8>> {
    let source = fontmin_detect::detect_format(input);

    match (source, target) {
        (FontFormat::Ttf, OutputFormat::Woff) => ttf_to_woff(input, &WoffOptions::default()),
        (FontFormat::Ttf, OutputFormat::Woff2) => ttf_to_woff2(input, &Woff2Options::default()),
        (FontFormat::Ttf, OutputFormat::Eot) => ttf_to_eot(input, &EotOptions::default()),
        (FontFormat::Ttf, OutputFormat::Svg) => {
            ttf_to_svg(input, &Ttf2SvgOptions::default()).map(String::into_bytes)
        }
        (FontFormat::Svg, OutputFormat::Ttf) => {
            let svg = std::str::from_utf8(input).map_err(|error| {
                FontminError::invalid_font(format!("invalid SVG UTF-8: {error}"))
            })?;

            svg_font_to_ttf(svg, &Svg2TtfOptions::default())
        }
        (FontFormat::Woff, OutputFormat::Ttf) => woff_to_ttf(input),
        (FontFormat::Woff2, OutputFormat::Ttf) => woff2_to_ttf(input),
        (FontFormat::Eot, OutputFormat::Ttf) => eot_to_ttf(input),
        (FontFormat::Otf, OutputFormat::Ttf) => otf_to_ttf(input, otf_options),
        (FontFormat::Ttf, OutputFormat::Ttf)
        | (FontFormat::Woff, OutputFormat::Woff)
        | (FontFormat::Woff2, OutputFormat::Woff2)
        | (FontFormat::Eot, OutputFormat::Eot)
        | (FontFormat::Svg, OutputFormat::Svg) => Ok(input.to_vec()),
        (_, OutputFormat::Eot) => Err(FontminError::unsupported("eot")),
        (_, OutputFormat::Svg) => Err(FontminError::unsupported("svg")),
        (_, OutputFormat::Css) => Err(FontminError::unsupported("css")),
        (source, target) => Err(FontminError::unsupported(format!(
            "{} to {}",
            format!("{source:?}").to_ascii_lowercase(),
            format!("{target:?}").to_ascii_lowercase(),
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use fontmin_core::OutputFormat;
    use fontmin_diagnostics::FontminErrorKind;
    use fontmin_testing::{ROBOTO, SOURCE_SERIF_4_VARIABLE_CFF2, roboto_otf};

    use super::{
        CssFontSource, CssOptions, Otf2TtfOptions, Svg2TtfOptions, SvgIcon, Svgs2TtfOptions,
        convert, convert_with_options, generate_font_face_css, inspect, svg_font_to_ttf,
        svgs_to_ttf, woff_to_ttf,
    };

    const ICON_SVG: &str =
        r#"<svg viewBox="0 0 1000 1000"><path d="M100 100 L900 100 L900 900 L100 900 Z"/></svg>"#;
    const SVG_FONT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><defs><font id="icons" horiz-adv-x="1000"><font-face font-family="SVG Icons" units-per-em="1000" ascent="850" descent="-150" /><glyph glyph-name="home" unicode="&#xE101;" horiz-adv-x="1000" d="M100 100 L900 100 L900 900 L100 900 Z" /></font></defs></svg>"#;

    #[test]
    fn ttf_convert_keeps_bytes_for_ttf_input() {
        assert_eq!(convert(ROBOTO, OutputFormat::Ttf).unwrap(), ROBOTO);
    }

    #[test]
    fn otf_convert_rewrites_glyf_backed_wrapper_to_ttf() {
        let otf = roboto_otf();
        let output = convert(&otf, OutputFormat::Ttf).unwrap();

        assert!(output.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert_eq!(&output[4..], &ROBOTO[4..]);
    }

    #[test]
    fn convert_with_options_instantiates_cff2_coordinates() {
        let mut variation_coordinates = BTreeMap::new();
        variation_coordinates.insert("wght".to_owned(), 700.0);
        variation_coordinates.insert("opsz".to_owned(), 14.0);
        let output = convert_with_options(
            SOURCE_SERIF_4_VARIABLE_CFF2,
            OutputFormat::Ttf,
            &Otf2TtfOptions {
                preserve_hinting: false,
                variation_coordinates,
            },
        )
        .unwrap();

        assert!(output.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        let info = inspect(&output).unwrap();
        assert!(!info.metadata.tables.iter().any(|tag| tag == "CFF2"));
        assert!(!info.metadata.tables.iter().any(|tag| tag == "fvar"));
    }

    #[test]
    fn woff_conversion_returns_valid_smaller_woff() {
        let output = convert(ROBOTO, OutputFormat::Woff).unwrap();
        let declared_length = u32::from_be_bytes(output[8..12].try_into().unwrap()) as usize;

        assert!(output.starts_with(b"wOFF"));
        assert_eq!(declared_length, output.len());
        assert!(output.len() < ROBOTO.len());
    }

    #[test]
    fn woff_decode_returns_valid_ttf() {
        let woff = convert(ROBOTO, OutputFormat::Woff).unwrap();
        let output = woff_to_ttf(&woff).unwrap();

        assert!(output.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert_eq!(
            fontmin_detect::detect_format(&output),
            super::FontFormat::Ttf
        );
    }

    #[test]
    fn inspect_reads_woff_metadata() {
        let woff = convert(ROBOTO, OutputFormat::Woff).unwrap();
        let info = inspect(&woff).unwrap();

        assert_eq!(info.format, super::FontFormat::Woff);
        assert_eq!(info.size, woff.len());
        assert_eq!(info.metadata.family_name.as_deref(), Some("Roboto"));
        assert_eq!(info.metadata.full_name.as_deref(), Some("Roboto Regular"));
        assert_eq!(info.metadata.glyph_count, 3387);
    }

    #[test]
    fn inspect_reads_eot_metadata() {
        let eot = convert(ROBOTO, OutputFormat::Eot).unwrap();
        let info = inspect(&eot).unwrap();

        assert_eq!(info.format, super::FontFormat::Eot);
        assert_eq!(info.size, eot.len());
        assert_eq!(info.metadata.family_name.as_deref(), Some("Roboto"));
        assert_eq!(info.metadata.full_name.as_deref(), Some("Roboto Regular"));
        assert_eq!(info.metadata.glyph_count, 3387);
    }

    #[test]
    fn inspect_reads_otf_metadata() {
        let otf = roboto_otf();
        let info = inspect(&otf).unwrap();

        assert_eq!(info.format, super::FontFormat::Otf);
        assert_eq!(info.size, otf.len());
        assert_eq!(info.metadata.family_name.as_deref(), Some("Roboto"));
        assert_eq!(info.metadata.subfamily_name.as_deref(), Some("Regular"));
        assert_eq!(info.metadata.full_name.as_deref(), Some("Roboto Regular"));
        assert_eq!(
            info.metadata.post_script_name.as_deref(),
            Some("Roboto-Regular")
        );
        assert_eq!(info.metadata.glyph_count, 3387);
        assert_eq!(info.metadata.units_per_em, 2048);
        assert_eq!(info.metadata.ascender, 2146);
        assert_eq!(info.metadata.descender, -555);
        assert!(info.metadata.tables.contains(&"name".into()));
    }

    #[test]
    fn woff2_conversion_returns_valid_smaller_woff2() {
        let output = convert(ROBOTO, OutputFormat::Woff2).unwrap();
        let declared_length = u32::from_be_bytes(output[8..12].try_into().unwrap()) as usize;

        assert!(output.starts_with(b"wOF2"));
        assert_eq!(declared_length, output.len());
        assert!(output.len() < ROBOTO.len());
    }

    #[test]
    fn inspect_reads_woff2_table_metadata() {
        let woff2 = convert(ROBOTO, OutputFormat::Woff2).unwrap();
        let info = inspect(&woff2).unwrap();

        assert_eq!(info.format, super::FontFormat::Woff2);
        assert_eq!(info.size, woff2.len());
        assert_eq!(info.metadata.family_name.as_deref(), Some("Roboto"));
        assert_eq!(info.metadata.full_name.as_deref(), Some("Roboto Regular"));
        assert_eq!(info.metadata.glyph_count, 3387);
        assert_eq!(info.metadata.units_per_em, 2048);
        assert!(info.metadata.tables.contains(&"cmap".into()));
        assert!(info.metadata.tables.contains(&"name".into()));
    }

    #[test]
    fn woff2_decode_returns_valid_ttf() {
        let woff2 = convert(ROBOTO, OutputFormat::Woff2).unwrap();
        let ttf = convert(&woff2, OutputFormat::Ttf).unwrap();
        let info = inspect(&ttf).unwrap();

        assert!(ttf.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert_eq!(info.format, super::FontFormat::Ttf);
        assert_eq!(info.metadata.family_name.as_deref(), Some("Roboto"));
        assert_eq!(info.metadata.glyph_count, 3387);
    }

    #[test]
    fn validate_woff2_accepts_converted_output() {
        let woff2 = convert(ROBOTO, OutputFormat::Woff2).unwrap();

        super::validate_woff2(&woff2).unwrap();
    }

    #[test]
    fn validate_woff2_rejects_invalid_data() {
        let error = super::validate_woff2(b"not woff2").unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::InvalidFont);
    }

    #[test]
    fn eot_conversion_wraps_ttf_in_eot_header() {
        let output = convert(ROBOTO, OutputFormat::Eot).unwrap();
        let eot_size = u32::from_le_bytes(output[0..4].try_into().unwrap()) as usize;
        let font_data_size = u32::from_le_bytes(output[4..8].try_into().unwrap()) as usize;

        assert_eq!(
            fontmin_detect::detect_format(&output),
            super::FontFormat::Eot
        );
        assert_eq!(eot_size, output.len());
        assert_eq!(font_data_size, ROBOTO.len());
        assert_eq!(&output[8..12], &[0x01, 0x00, 0x02, 0x00]);
        assert_eq!(&output[34..36], &[0x4c, 0x50]);
        assert!(output.ends_with(ROBOTO));
    }

    #[test]
    fn eot_decode_returns_valid_ttf() {
        let eot = convert(ROBOTO, OutputFormat::Eot).unwrap();
        let output = convert(&eot, OutputFormat::Ttf).unwrap();
        let info = inspect(&output).unwrap();

        assert!(output.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert_eq!(info.format, super::FontFormat::Ttf);
        assert_eq!(info.metadata.family_name.as_deref(), Some("Roboto"));
        assert_eq!(info.metadata.glyph_count, 3387);
    }

    #[test]
    fn svg_conversion_emits_svg_font_with_glyphs() {
        let output = convert(ROBOTO, OutputFormat::Svg).unwrap();
        let svg = std::str::from_utf8(&output).unwrap();

        assert_eq!(
            fontmin_detect::detect_format(&output),
            super::FontFormat::Svg
        );
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<font "));
        assert!(svg.contains("font-family=\"Roboto\""));
        assert!(svg.contains("<font-face "));
        assert!(svg.contains("<glyph "));
        assert!(svg.contains("unicode=\"A\""));
        assert!(svg.contains("d=\"M"));
    }

    #[test]
    fn svgs_to_ttf_generates_inspectable_icon_font() {
        let output = svgs_to_ttf(
            vec![SvgIcon {
                name: "square".into(),
                contents: ICON_SVG.into(),
                unicode: None,
            }],
            &Svgs2TtfOptions {
                font_name: "Icons".into(),
                start_unicode: 0xE001,
                ascent: 850,
                descent: -150,
                normalize: true,
            },
        )
        .unwrap();
        let info = inspect(&output).unwrap();

        assert_eq!(info.format, super::FontFormat::Ttf);
        assert_eq!(info.metadata.family_name.as_deref(), Some("Icons"));
        assert_eq!(info.metadata.glyph_count, 2);
        assert_eq!(info.metadata.units_per_em, 1000);
    }

    #[test]
    fn svg_font_to_ttf_generates_inspectable_ttf() {
        let output = svg_font_to_ttf(
            SVG_FONT,
            &Svg2TtfOptions {
                normalize: true,
                hinting: false,
            },
        )
        .unwrap();
        let info = inspect(&output).unwrap();

        assert_eq!(info.format, super::FontFormat::Ttf);
        assert_eq!(info.metadata.family_name.as_deref(), Some("SVG Icons"));
        assert_eq!(info.metadata.glyph_count, 2);
        assert_eq!(info.metadata.units_per_em, 1000);
        assert_eq!(info.metadata.ascender, 850);
        assert_eq!(info.metadata.descender, -150);
    }

    #[test]
    fn svg_font_convert_writes_ttf() {
        let output = convert(SVG_FONT.as_bytes(), OutputFormat::Ttf).unwrap();
        let info = inspect(&output).unwrap();

        assert_eq!(info.format, super::FontFormat::Ttf);
        assert_eq!(info.metadata.family_name.as_deref(), Some("SVG Icons"));
        assert_eq!(info.metadata.glyph_count, 2);
    }

    #[test]
    fn css_generation_emits_font_face_sources() {
        let css = generate_font_face_css(
            &[
                CssFontSource::new("roboto.woff", OutputFormat::Woff),
                CssFontSource::new("roboto.woff2", OutputFormat::Woff2),
            ],
            &CssOptions {
                font_family: "Roboto".into(),
                font_path: "./fonts".into(),
                base64: false,
                glyph: false,
                icon_prefix: "icon".into(),
                as_file_name: false,
                local: true,
                font_display: "swap".into(),
                target: CssOptions::default().target,
                unicode_ranges: Vec::new(),
            },
        )
        .unwrap();

        assert!(css.contains("@font-face"));
        assert!(css.contains("font-family: 'Roboto';"));
        assert!(css.contains("local('Roboto')"));
        assert!(css.contains("url('./fonts/roboto.woff') format('woff')"));
        assert!(css.contains("url('./fonts/roboto.woff2') format('woff2')"));
        assert!(css.contains("font-display: swap;"));
    }

    #[test]
    fn unsupported_conversions_return_typed_errors() {
        let error = convert(b"abc", OutputFormat::Eot).unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::UnsupportedFormat);
    }
}
