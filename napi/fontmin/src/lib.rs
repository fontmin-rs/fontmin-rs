#![allow(clippy::needless_pass_by_value)]

use std::collections::HashMap;

use fontmin::{
    CoverageOptions, CoverageReport, CssFontSource, CssGlyph, CssOptions, CssTarget, EotOptions,
    FontFormat, FontInfo, FontMetadata, LayoutSubsetMode, MissingGlyphPolicy, Otf2TtfOptions,
    OutputFormat, SubsetOptions, Svg2TtfOptions, SvgIcon, Svgs2TtfOptions, Ttf2SvgOptions,
    UnicodeRange, Woff2Options, WoffOptions,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi(object)]
pub struct JsSubsetOptions {
    pub text: Option<String>,
    pub unicodes: Option<Vec<u32>>,
    pub unicode_ranges: Option<Vec<String>>,
    pub basic_text: Option<bool>,
    pub preserve_hinting: Option<bool>,
    pub trim: Option<bool>,
    pub keep_notdef: Option<bool>,
    pub keep_layout: Option<String>,
    pub missing_glyphs: Option<String>,
}

#[napi(object)]
pub struct JsCoverageOptions {
    pub text: Option<String>,
    pub unicodes: Option<Vec<u32>>,
    pub unicode_ranges: Option<Vec<String>>,
    pub basic_text: Option<bool>,
}

#[napi(object)]
pub struct JsCoverageReport {
    pub requested: Vec<u32>,
    pub supported: Vec<u32>,
    pub missing: Vec<u32>,
    pub coverage_percent: f64,
}

#[napi(object)]
pub struct JsWoffOptions {
    pub deflate: Option<bool>,
    pub compression_level: Option<u32>,
    pub metadata: Option<String>,
    pub private_data: Option<Buffer>,
}

#[napi(object)]
pub struct JsWoff2Options {
    pub quality: Option<u8>,
}

#[napi(object)]
pub struct JsEotOptions {
    pub version: Option<u32>,
}

#[napi(object)]
pub struct JsOtf2TtfOptions {
    pub preserve_hinting: Option<bool>,
    pub variation_coordinates: Option<HashMap<String, f64>>,
}

#[napi(object)]
pub struct JsSvgOptions {
    pub font_family: Option<String>,
}

#[napi(object)]
pub struct JsSvg2TtfOptions {
    pub normalize: Option<bool>,
    pub hinting: Option<bool>,
}

#[napi(object)]
pub struct JsSvgIcon {
    pub name: String,
    pub contents: String,
    pub unicode: Option<u32>,
}

#[napi(object)]
pub struct JsSvgs2TtfOptions {
    pub font_name: Option<String>,
    pub start_unicode: Option<u32>,
    pub ascent: Option<i32>,
    pub descent: Option<i32>,
    pub normalize: Option<bool>,
}

#[napi(object)]
pub struct JsCssFontSource {
    pub file_name: String,
    pub format: String,
    pub contents: Option<Buffer>,
    pub glyphs: Option<Vec<JsCssGlyph>>,
    pub unicode_ranges: Option<Vec<String>>,
}

#[napi(object)]
pub struct JsCssGlyph {
    pub name: Option<String>,
    pub unicode: u32,
}

#[napi(object)]
pub struct JsCssOptions {
    pub font_family: Option<String>,
    pub font_path: Option<String>,
    pub base64: Option<bool>,
    pub glyph: Option<bool>,
    pub icon_prefix: Option<String>,
    pub as_file_name: Option<bool>,
    pub local: Option<bool>,
    pub font_display: Option<String>,
    pub target: Option<String>,
    pub unicode_ranges: Option<Vec<String>>,
}

#[napi(object)]
pub struct JsFontMetadata {
    pub family_name: Option<String>,
    pub subfamily_name: Option<String>,
    pub full_name: Option<String>,
    pub post_script_name: Option<String>,
    pub glyph_count: u32,
    pub units_per_em: u32,
    pub ascender: i32,
    pub descender: i32,
    pub tables: Vec<String>,
}

#[napi(object)]
pub struct JsFontInfo {
    pub format: String,
    pub size: u32,
    pub metadata: JsFontMetadata,
}

#[napi(js_name = "subsetTtf")]
pub fn subset_ttf(input: Buffer, options: Option<JsSubsetOptions>) -> napi::Result<Buffer> {
    let options = subset_options_from_js(options)?;
    let output = fontmin::subset_ttf(&input, options)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;

    Ok(output.into())
}

#[napi(js_name = "analyzeCoverage")]
pub fn analyze_coverage(
    input: Buffer,
    options: Option<JsCoverageOptions>,
) -> napi::Result<JsCoverageReport> {
    let options = coverage_options_from_js(options)?;
    let report = fontmin::analyze_coverage(&input, options)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;

    Ok(coverage_report_to_js(report))
}

#[napi(js_name = "inspectFont")]
pub fn inspect_font(input: Buffer) -> napi::Result<JsFontInfo> {
    let info =
        fontmin::inspect(&input).map_err(|error| napi::Error::from_reason(error.to_string()))?;

    font_info_to_js(info)
}

#[napi(js_name = "ttfToWoff")]
pub fn ttf_to_woff(input: Buffer, options: Option<JsWoffOptions>) -> napi::Result<Buffer> {
    let options = woff_options_from_js(options);
    let output = fontmin::ttf_to_woff(&input, &options)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;

    Ok(output.into())
}

#[napi(js_name = "woffToTtf")]
pub fn woff_to_ttf(input: Buffer) -> napi::Result<Buffer> {
    let output = fontmin::woff_to_ttf(&input)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;

    Ok(output.into())
}

#[napi(js_name = "ttfToWoff2")]
pub fn ttf_to_woff2(input: Buffer, options: Option<JsWoff2Options>) -> napi::Result<Buffer> {
    let options = woff2_options_from_js(options);
    let output = fontmin::ttf_to_woff2(&input, &options)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;

    Ok(output.into())
}

#[napi(js_name = "woff2ToTtf")]
pub fn woff2_to_ttf(input: Buffer) -> napi::Result<Buffer> {
    let output = fontmin::woff2_to_ttf(&input)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;

    Ok(output.into())
}

#[napi(js_name = "validateWoff2")]
#[allow(clippy::needless_pass_by_value)]
pub fn validate_woff2(input: Buffer) -> napi::Result<()> {
    fontmin::validate_woff2(&input).map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi(js_name = "ttfToEot")]
pub fn ttf_to_eot(input: Buffer, options: Option<JsEotOptions>) -> napi::Result<Buffer> {
    let options = eot_options_from_js(options);
    let output = fontmin::ttf_to_eot(&input, &options)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;

    Ok(output.into())
}

#[napi(js_name = "ttfToSvg")]
pub fn ttf_to_svg(input: Buffer, options: Option<JsSvgOptions>) -> napi::Result<String> {
    let options = svg_options_from_js(options);

    fontmin::ttf_to_svg(&input, &options)
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi(js_name = "svgFontToTtf")]
#[allow(clippy::needless_pass_by_value)]
pub fn svg_font_to_ttf(input: String, options: Option<JsSvg2TtfOptions>) -> napi::Result<Buffer> {
    let options = svg2ttf_options_from_js(options);
    let output = fontmin::svg_font_to_ttf(&input, &options)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;

    Ok(output.into())
}

#[napi(js_name = "svgsToTtf")]
pub fn svgs_to_ttf(
    inputs: Vec<JsSvgIcon>,
    options: Option<JsSvgs2TtfOptions>,
) -> napi::Result<Buffer> {
    let inputs = svg_icons_from_js(inputs);
    let options = svgs2ttf_options_from_js(options)?;
    let output = fontmin::svgs_to_ttf(inputs, &options)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;

    Ok(output.into())
}

#[napi(js_name = "eotToTtf")]
pub fn eot_to_ttf(input: Buffer) -> napi::Result<Buffer> {
    let output =
        fontmin::eot_to_ttf(&input).map_err(|error| napi::Error::from_reason(error.to_string()))?;

    Ok(output.into())
}

#[napi(js_name = "otfToTtf")]
pub fn otf_to_ttf(input: Buffer, options: Option<JsOtf2TtfOptions>) -> napi::Result<Buffer> {
    let options = otf2ttf_options_from_js(options);
    let output = fontmin::otf_to_ttf(&input, &options)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;

    Ok(output.into())
}

#[napi(js_name = "generateFontFaceCss")]
pub fn generate_font_face_css(
    sources: Vec<JsCssFontSource>,
    options: Option<JsCssOptions>,
) -> napi::Result<String> {
    let sources = css_sources_from_js(sources)?;
    let options = css_options_from_js(options)?;
    let css = fontmin::generate_font_face_css(&sources, &options)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;

    Ok(css)
}

fn subset_options_from_js(options: Option<JsSubsetOptions>) -> napi::Result<SubsetOptions> {
    let Some(options) = options else {
        return Ok(SubsetOptions::default());
    };

    Ok(SubsetOptions {
        text: options.text,
        unicodes: options.unicodes.unwrap_or_default(),
        unicode_ranges: unicode_ranges_from_js(options.unicode_ranges)?,
        basic_text: options.basic_text.unwrap_or(false),
        preserve_hinting: options.preserve_hinting.unwrap_or(false),
        trim: options.trim.unwrap_or(true),
        keep_notdef: options.keep_notdef.unwrap_or(true),
        layout: layout_mode_from_js(options.keep_layout)?,
        missing_glyphs: missing_glyph_policy_from_js(options.missing_glyphs)?,
    })
}

fn coverage_options_from_js(options: Option<JsCoverageOptions>) -> napi::Result<CoverageOptions> {
    let Some(options) = options else {
        return Ok(CoverageOptions::default());
    };

    Ok(CoverageOptions {
        text: options.text,
        unicodes: options.unicodes.unwrap_or_default(),
        unicode_ranges: unicode_ranges_from_js(options.unicode_ranges)?,
        basic_text: options.basic_text.unwrap_or(false),
    })
}

fn missing_glyph_policy_from_js(value: Option<String>) -> napi::Result<MissingGlyphPolicy> {
    match value.as_deref().unwrap_or("warn") {
        "ignore" => Ok(MissingGlyphPolicy::Ignore),
        "warn" => Ok(MissingGlyphPolicy::Warn),
        "error" => Ok(MissingGlyphPolicy::Error),
        other => Err(napi::Error::from_reason(format!(
            "unknown missingGlyphs value: {other}",
        ))),
    }
}

fn layout_mode_from_js(value: Option<String>) -> napi::Result<LayoutSubsetMode> {
    match value.as_deref().unwrap_or("conservative") {
        "drop" => Ok(LayoutSubsetMode::Drop),
        "conservative" => Ok(LayoutSubsetMode::Conservative),
        "preserve" => Ok(LayoutSubsetMode::Preserve),
        other => Err(napi::Error::from_reason(format!(
            "unknown keepLayout value: {other}",
        ))),
    }
}

fn woff_options_from_js(options: Option<JsWoffOptions>) -> WoffOptions {
    let Some(options) = options else {
        return WoffOptions::default();
    };

    WoffOptions {
        deflate: options.deflate.unwrap_or(true),
        compression_level: options.compression_level,
        metadata: options.metadata,
        private_data: options.private_data.map(|data| data.to_vec()),
    }
}

fn woff2_options_from_js(options: Option<JsWoff2Options>) -> Woff2Options {
    let Some(options) = options else {
        return Woff2Options::default();
    };

    Woff2Options {
        quality: options.quality,
    }
}

fn eot_options_from_js(options: Option<JsEotOptions>) -> EotOptions {
    let Some(options) = options else {
        return EotOptions::default();
    };

    EotOptions {
        version: options.version,
    }
}

#[allow(clippy::cast_possible_truncation)]
fn otf2ttf_options_from_js(options: Option<JsOtf2TtfOptions>) -> Otf2TtfOptions {
    let Some(options) = options else {
        return Otf2TtfOptions::default();
    };

    Otf2TtfOptions {
        preserve_hinting: options.preserve_hinting.unwrap_or(false),
        variation_coordinates: options
            .variation_coordinates
            .unwrap_or_default()
            .into_iter()
            .map(|(tag, value)| (tag, value as f32))
            .collect(),
    }
}

fn svg_options_from_js(options: Option<JsSvgOptions>) -> Ttf2SvgOptions {
    let Some(options) = options else {
        return Ttf2SvgOptions::default();
    };

    Ttf2SvgOptions {
        font_family: options.font_family,
    }
}

fn svg2ttf_options_from_js(options: Option<JsSvg2TtfOptions>) -> Svg2TtfOptions {
    let Some(options) = options else {
        return Svg2TtfOptions::default();
    };

    Svg2TtfOptions {
        normalize: options.normalize.unwrap_or(true),
        hinting: options.hinting.unwrap_or(false),
    }
}

fn svg_icons_from_js(inputs: Vec<JsSvgIcon>) -> Vec<SvgIcon> {
    inputs
        .into_iter()
        .map(|input| SvgIcon {
            name: input.name,
            contents: input.contents,
            unicode: input.unicode,
        })
        .collect()
}

fn svgs2ttf_options_from_js(options: Option<JsSvgs2TtfOptions>) -> napi::Result<Svgs2TtfOptions> {
    let Some(options) = options else {
        return Ok(Svgs2TtfOptions::default());
    };
    let default_options = Svgs2TtfOptions::default();

    Ok(Svgs2TtfOptions {
        font_name: options.font_name.unwrap_or(default_options.font_name),
        start_unicode: options
            .start_unicode
            .unwrap_or(default_options.start_unicode),
        ascent: i16_from_js(options.ascent, default_options.ascent, "ascent")?,
        descent: i16_from_js(options.descent, default_options.descent, "descent")?,
        normalize: options.normalize.unwrap_or(default_options.normalize),
    })
}

fn i16_from_js(value: Option<i32>, default_value: i16, name: &str) -> napi::Result<i16> {
    let Some(value) = value else {
        return Ok(default_value);
    };

    i16::try_from(value)
        .map_err(|_| napi::Error::from_reason(format!("{name} is outside the supported i16 range")))
}

fn css_sources_from_js(sources: Vec<JsCssFontSource>) -> napi::Result<Vec<CssFontSource>> {
    sources
        .into_iter()
        .map(|source| {
            let format = output_format_from_js(&source.format)?;
            let glyphs = css_glyphs_from_js(source.glyphs);
            let mut css_source = CssFontSource::new(source.file_name, format).with_glyphs(glyphs);

            let unicode_ranges = unicode_ranges_from_js(source.unicode_ranges)?;
            if !unicode_ranges.is_empty() {
                css_source = css_source.with_unicode_ranges(unicode_ranges);
            }

            if let Some(contents) = source.contents {
                css_source = css_source.with_contents(contents.to_vec());
            }

            Ok(css_source)
        })
        .collect()
}

fn css_glyphs_from_js(glyphs: Option<Vec<JsCssGlyph>>) -> Vec<CssGlyph> {
    glyphs.map_or_else(Vec::new, |glyphs| {
        glyphs
            .into_iter()
            .map(|glyph| CssGlyph::new(glyph.name, glyph.unicode))
            .collect()
    })
}

fn css_options_from_js(options: Option<JsCssOptions>) -> napi::Result<CssOptions> {
    let Some(options) = options else {
        return Ok(CssOptions::default());
    };
    let default_options = CssOptions::default();

    Ok(CssOptions {
        font_family: options.font_family.unwrap_or(default_options.font_family),
        font_path: options.font_path.unwrap_or(default_options.font_path),
        base64: options.base64.unwrap_or(default_options.base64),
        glyph: options.glyph.unwrap_or(default_options.glyph),
        icon_prefix: options.icon_prefix.unwrap_or(default_options.icon_prefix),
        as_file_name: options.as_file_name.unwrap_or(default_options.as_file_name),
        local: options.local.unwrap_or(default_options.local),
        font_display: options.font_display.unwrap_or(default_options.font_display),
        target: css_target_from_js(options.target)?.unwrap_or(default_options.target),
        unicode_ranges: unicode_ranges_from_js(options.unicode_ranges)?,
    })
}

fn unicode_ranges_from_js(values: Option<Vec<String>>) -> napi::Result<Vec<UnicodeRange>> {
    values
        .unwrap_or_default()
        .into_iter()
        .map(|value| {
            value
                .parse::<UnicodeRange>()
                .map_err(|error| napi::Error::from_reason(error.to_string()))
        })
        .collect()
}

fn css_target_from_js(value: Option<String>) -> napi::Result<Option<CssTarget>> {
    let Some(value) = value else {
        return Ok(None);
    };

    match value.as_str() {
        "css" => Ok(Some(CssTarget::Css)),
        "scss" => Ok(Some(CssTarget::Scss)),
        "less" => Ok(Some(CssTarget::Less)),
        other => Err(napi::Error::from_reason(format!(
            "unknown CSS target: {other}",
        ))),
    }
}

fn output_format_from_js(value: &str) -> napi::Result<OutputFormat> {
    match value {
        "ttf" => Ok(OutputFormat::Ttf),
        "woff" => Ok(OutputFormat::Woff),
        "woff2" => Ok(OutputFormat::Woff2),
        "eot" => Ok(OutputFormat::Eot),
        "svg" => Ok(OutputFormat::Svg),
        "css" => Ok(OutputFormat::Css),
        other => Err(napi::Error::from_reason(format!(
            "unknown font source format: {other}",
        ))),
    }
}

fn font_info_to_js(info: FontInfo) -> napi::Result<JsFontInfo> {
    let size = u32::try_from(info.size)
        .map_err(|_| napi::Error::from_reason("font size exceeds JavaScript number range"))?;

    Ok(JsFontInfo {
        format: font_format_to_js(info.format).into(),
        size,
        metadata: font_metadata_to_js(info.metadata),
    })
}

fn coverage_report_to_js(report: CoverageReport) -> JsCoverageReport {
    JsCoverageReport {
        requested: report.requested,
        supported: report.supported,
        missing: report.missing,
        coverage_percent: report.coverage_percent,
    }
}

fn font_metadata_to_js(metadata: FontMetadata) -> JsFontMetadata {
    JsFontMetadata {
        family_name: metadata.family_name,
        subfamily_name: metadata.subfamily_name,
        full_name: metadata.full_name,
        post_script_name: metadata.post_script_name,
        glyph_count: u32::from(metadata.glyph_count),
        units_per_em: u32::from(metadata.units_per_em),
        ascender: i32::from(metadata.ascender),
        descender: i32::from(metadata.descender),
        tables: metadata.tables,
    }
}

fn font_format_to_js(format: FontFormat) -> &'static str {
    match format {
        FontFormat::Ttf => "ttf",
        FontFormat::Otf => "otf",
        FontFormat::Woff => "woff",
        FontFormat::Woff2 => "woff2",
        FontFormat::Eot => "eot",
        FontFormat::Svg => "svg",
        FontFormat::Css => "css",
        FontFormat::Unknown => "unknown",
    }
}
