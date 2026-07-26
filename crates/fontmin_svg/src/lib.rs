use std::{collections::BTreeSet, fmt::Write as _};

use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};
use skrifa::{
    FontRef, GlyphId, MetadataProvider,
    instance::{LocationRef, Size},
    outline::{DrawSettings, OutlinePen},
};

mod icon;

pub use icon::{Svg2TtfOptions, SvgIcon, Svgs2TtfOptions, svg_font_to_ttf, svgs_to_ttf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Ttf2SvgOptions {
    pub font_family: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct GlyphMapping {
    character: char,
    glyph_id: GlyphId,
    has_outline: bool,
}

pub fn ttf_to_svg(input: &[u8], options: &Ttf2SvgOptions) -> Result<String> {
    if !is_ttf(input) {
        return Err(FontminError::invalid_font(
            "expected TrueType sfnt data for SVG encoding",
        ));
    }

    let font = FontRef::new(input)
        .map_err(|error| FontminError::invalid_font(format!("failed to parse TTF: {error}")))?;
    let raw_font = fontmin_ttf::read_ttf(input)?;
    let metadata = fontmin_ttf::inspect_ttf(input)?;
    let font_family = options
        .font_family
        .clone()
        .or(metadata.family_name)
        .unwrap_or_else(|| "fontmin".into());
    let font_id = font_id(&font_family);
    let glyph_metrics = font.glyph_metrics(Size::unscaled(), LocationRef::default());
    let units_per_em = metadata.units_per_em;
    let default_advance = glyph_metrics
        .advance_width(GlyphId::new(0))
        .unwrap_or(f32::from(units_per_em));
    let mappings = collect_glyph_mappings(&font, &raw_font, metadata.glyph_count)?;
    let mut svg = String::new();

    write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\"><defs><font id=\"{}\" horiz-adv-x=\"{}\"><font-face font-family=\"{}\" units-per-em=\"{}\" ascent=\"{}\" descent=\"{}\" /><missing-glyph horiz-adv-x=\"{}\" />",
        font_id,
        default_advance,
        escape_attribute(&font_family),
        units_per_em,
        metadata.ascender,
        metadata.descender,
        default_advance,
    )
    .expect("writing to string should not fail");

    for mapping in mappings {
        push_glyph(&mut svg, &font, &glyph_metrics, mapping, default_advance);
    }

    svg.push_str("</font></defs></svg>");

    Ok(svg)
}

fn is_ttf(input: &[u8]) -> bool {
    input.starts_with(&[0x00, 0x01, 0x00, 0x00]) || input.starts_with(b"true")
}

fn font_id(font_family: &str) -> String {
    let mut id = String::from("fontmin");

    for character in font_family.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':') {
            id.push(character);
        } else if !id.ends_with('-') {
            id.push('-');
        }
    }

    id.trim_end_matches('-').to_string()
}

fn collect_glyph_mappings(
    font: &FontRef<'_>,
    raw_font: &fontmin_ttf::TtfFont<'_>,
    glyph_count: u16,
) -> Result<Vec<GlyphMapping>> {
    let charmap = font.charmap();
    let mut codepoints = BTreeSet::new();

    for (codepoint, _) in charmap.mappings() {
        codepoints.insert(codepoint);
    }

    let mut mappings = Vec::new();

    for character in codepoints.into_iter().filter_map(char::from_u32) {
        let Some(glyph_id) = charmap.map(character) else {
            continue;
        };
        if glyph_id == GlyphId::new(0) {
            continue;
        }

        mappings.push(GlyphMapping {
            character,
            glyph_id,
            has_outline: glyph_has_outline(raw_font, glyph_count, glyph_id)?,
        });
    }

    Ok(mappings)
}

fn push_glyph(
    svg: &mut String,
    font: &FontRef<'_>,
    metrics: &skrifa::metrics::GlyphMetrics<'_>,
    mapping: GlyphMapping,
    default_advance: f32,
) {
    let advance = metrics
        .advance_width(mapping.glyph_id)
        .unwrap_or(default_advance);
    let path = glyph_path(font, mapping.glyph_id, mapping.has_outline);

    write!(
        svg,
        "<glyph glyph-name=\"glyph{}\" unicode=\"{}\" horiz-adv-x=\"{}\"",
        mapping.glyph_id.to_u32(),
        escape_unicode(mapping.character),
        advance,
    )
    .expect("writing to string should not fail");

    if let Some(path) = path {
        write!(svg, " d=\"{}\"", escape_attribute(&path))
            .expect("writing to string should not fail");
    }

    svg.push_str(" />");
}

fn glyph_path(font: &FontRef<'_>, glyph_id: GlyphId, has_outline: bool) -> Option<String> {
    if !has_outline {
        return None;
    }

    let mut builder = SvgPathBuilder::default();
    let glyph = font.outline_glyphs().get(glyph_id)?;

    glyph
        .draw(
            DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
            &mut builder,
        )
        .ok()?;

    if builder.path.is_empty() {
        None
    } else {
        Some(builder.path)
    }
}

fn glyph_has_outline(
    font: &fontmin_ttf::TtfFont<'_>,
    glyph_count: u16,
    glyph_id: GlyphId,
) -> Result<bool> {
    let glyph_index = usize::try_from(glyph_id.to_u32())
        .map_err(|_| FontminError::invalid_font("glyph identifier does not fit in memory"))?;
    if glyph_index >= usize::from(glyph_count) {
        return Err(FontminError::invalid_font(format!(
            "glyph identifier {} exceeds maxp glyph count {glyph_count}",
            glyph_id.to_u32(),
        )));
    }

    let head = required_ttf_table(font, "head")?;
    let loca = required_ttf_table(font, "loca")?;
    let glyf = required_ttf_table(font, "glyf")?;
    let index_to_loc_format = read_u16(head, 50, "head indexToLocFormat")?;
    let start = read_glyph_offset(loca, glyph_index, index_to_loc_format)?;
    let end = read_glyph_offset(loca, glyph_index + 1, index_to_loc_format)?;

    if start > end || end > glyf.len() {
        return Err(FontminError::invalid_font(format!(
            "glyph {} has an invalid loca range {start}..{end}",
            glyph_id.to_u32(),
        )));
    }

    let glyph = &glyf[start..end];
    if glyph.is_empty() {
        return Ok(false);
    }

    let contour_count = read_i16(glyph, 0, "glyf numberOfContours")?;
    match contour_count {
        -1 | 1.. => Ok(true),
        0 => validate_empty_glyph(glyph, glyph_id),
        _ => Err(FontminError::invalid_font(format!(
            "glyph {} has invalid contour count {contour_count}",
            glyph_id.to_u32(),
        ))),
    }
}

fn validate_empty_glyph(glyph: &[u8], glyph_id: GlyphId) -> Result<bool> {
    let instruction_length = usize::from(read_u16(glyph, 10, "glyf instructionLength")?);
    let expected_length = 12usize
        .checked_add(instruction_length)
        .ok_or_else(|| FontminError::invalid_font("glyf instruction length overflows"))?;
    let trailing = glyph.get(expected_length..).ok_or_else(|| {
        FontminError::invalid_font(format!(
            "glyph {} has truncated instructions",
            glyph_id.to_u32(),
        ))
    })?;

    if trailing.len() > 3 || trailing.iter().any(|byte| *byte != 0) {
        return Err(FontminError::invalid_font(format!(
            "glyph {} declares zero contours but contains point data",
            glyph_id.to_u32(),
        )));
    }

    Ok(false)
}

fn required_ttf_table<'a>(font: &fontmin_ttf::TtfFont<'a>, tag: &str) -> Result<&'a [u8]> {
    font.table(tag)
        .ok_or_else(|| FontminError::invalid_font(format!("missing required TTF table {tag}")))
}

fn read_glyph_offset(data: &[u8], index: usize, format: u16) -> Result<usize> {
    match format {
        0 => read_u16(
            data,
            index
                .checked_mul(2)
                .ok_or_else(|| FontminError::invalid_font("loca offset overflows"))?,
            "short loca offset",
        )
        .map(|offset| usize::from(offset) * 2),
        1 => read_u32(
            data,
            index
                .checked_mul(4)
                .ok_or_else(|| FontminError::invalid_font("loca offset overflows"))?,
            "long loca offset",
        )
        .and_then(|offset| {
            usize::try_from(offset)
                .map_err(|_| FontminError::invalid_font("loca offset does not fit in memory"))
        }),
        _ => Err(FontminError::invalid_font(format!(
            "unsupported head indexToLocFormat {format}",
        ))),
    }
}

fn read_i16(data: &[u8], offset: usize, context: &str) -> Result<i16> {
    read_array::<2>(data, offset, context).map(i16::from_be_bytes)
}

fn read_u16(data: &[u8], offset: usize, context: &str) -> Result<u16> {
    read_array::<2>(data, offset, context).map(u16::from_be_bytes)
}

fn read_u32(data: &[u8], offset: usize, context: &str) -> Result<u32> {
    read_array::<4>(data, offset, context).map(u32::from_be_bytes)
}

fn read_array<const SIZE: usize>(data: &[u8], offset: usize, context: &str) -> Result<[u8; SIZE]> {
    let end = offset
        .checked_add(SIZE)
        .ok_or_else(|| FontminError::invalid_font(format!("{context} offset overflows")))?;
    let bytes = data
        .get(offset..end)
        .ok_or_else(|| FontminError::invalid_font(format!("truncated {context}")))?;

    bytes
        .try_into()
        .map_err(|_| FontminError::invalid_font(format!("truncated {context}")))
}

#[derive(Debug, Default)]
struct SvgPathBuilder {
    path: String,
}

impl SvgPathBuilder {
    fn command(&mut self, command: char) {
        if !self.path.is_empty() {
            self.path.push(' ');
        }
        self.path.push(command);
    }

    fn point(&mut self, x: f32, y: f32) {
        self.path.push(' ');
        push_number(&mut self.path, x);
        self.path.push(' ');
        push_number(&mut self.path, y);
    }
}

impl OutlinePen for SvgPathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.command('M');
        self.point(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.command('L');
        self.point(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.command('Q');
        self.point(x1, y1);
        self.point(x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.command('C');
        self.point(x1, y1);
        self.point(x2, y2);
        self.point(x, y);
    }

    fn close(&mut self) {
        self.command('Z');
    }
}

fn push_number(output: &mut String, value: f32) {
    let rounded = value.round();

    if (value - rounded).abs() < f32::EPSILON {
        write!(output, "{rounded:.0}").expect("writing to string should not fail");
        return;
    }

    let mut value = format!("{value:.3}");

    while value.contains('.') && value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.pop();
    }

    output.push_str(&value);
}

fn escape_unicode(character: char) -> String {
    match character {
        '&' => "&amp;".into(),
        '<' => "&lt;".into(),
        '>' => "&gt;".into(),
        '"' => "&quot;".into(),
        '\'' => "&apos;".into(),
        character if character.is_control() => format!("&#x{:X};", u32::from(character)),
        character => character.to_string(),
    }
}

fn escape_attribute(value: &str) -> String {
    let mut escaped = String::new();

    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            character if character.is_control() => {
                write!(escaped, "&#x{:X};", u32::from(character))
                    .expect("writing to string should not fail");
            }
            character => escaped.push(character),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use fontmin_testing::{HOME_ICON, ROBOTO, SVG_FONT, USER_ICON};
    use skrifa::{FontRef, MetadataProvider};

    use super::{
        Svg2TtfOptions, SvgIcon, Svgs2TtfOptions, Ttf2SvgOptions, read_glyph_offset, read_u16,
        svg_font_to_ttf, svgs_to_ttf, ttf_to_svg,
    };

    #[test]
    fn converts_ttf_to_svg_font() {
        let svg = ttf_to_svg(ROBOTO, &Ttf2SvgOptions::default()).unwrap();

        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<font "));
        assert!(svg.contains("font-family=\"Roboto\""));
        assert!(svg.contains("unicode=\"A\""));
        assert!(svg.contains("d=\"M"));
    }

    #[test]
    fn allows_overriding_font_family() {
        let svg = ttf_to_svg(
            ROBOTO,
            &Ttf2SvgOptions {
                font_family: Some("Custom & Family".into()),
            },
        )
        .unwrap();

        assert!(svg.contains("font-family=\"Custom &amp; Family\""));
    }

    #[test]
    fn rejects_zero_contour_glyphs_with_point_data() {
        let face = FontRef::new(ROBOTO).unwrap();
        let glyph_id = face.charmap().map('A').unwrap();
        let font = fontmin_ttf::read_ttf(ROBOTO).unwrap();
        let format = read_u16(font.table("head").unwrap(), 50, "head indexToLocFormat").unwrap();
        let glyph_offset = read_glyph_offset(
            font.table("loca").unwrap(),
            usize::try_from(glyph_id.to_u32()).unwrap(),
            format,
        )
        .unwrap();
        let glyf_offset = font
            .tables
            .iter()
            .find(|record| record.tag == "glyf")
            .unwrap()
            .offset;
        let mut malformed = ROBOTO.to_vec();
        malformed[glyf_offset + glyph_offset..glyf_offset + glyph_offset + 2].fill(0);

        let error = ttf_to_svg(&malformed, &Ttf2SvgOptions::default()).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("declares zero contours but contains point data"),
        );
    }

    #[test]
    fn combines_svg_icons_into_ttf_font() {
        let ttf = svgs_to_ttf(
            vec![
                SvgIcon {
                    name: "home".into(),
                    contents: HOME_ICON.into(),
                    unicode: Some(0xE101),
                },
                SvgIcon {
                    name: "user".into(),
                    contents: USER_ICON.into(),
                    unicode: None,
                },
            ],
            &Svgs2TtfOptions {
                font_name: "Icon Set".into(),
                start_unicode: 0xE200,
                ascent: 850,
                descent: -150,
                normalize: true,
            },
        )
        .unwrap();
        let metadata = fontmin_ttf::inspect_ttf(&ttf).unwrap();
        let face = FontRef::new(&ttf).unwrap();

        assert!(ttf.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert_eq!(metadata.family_name.as_deref(), Some("Icon Set"));
        assert_eq!(metadata.glyph_count, 3);
        assert_eq!(metadata.units_per_em, 1000);
        assert_eq!(metadata.ascender, 850);
        assert_eq!(metadata.descender, -150);
        let charmap = face.charmap();
        let home = charmap.map('\u{E101}').unwrap();

        assert!(charmap.map('\u{E200}').is_some());
        assert!(face.outline_glyphs().get(home).is_some());
    }

    #[test]
    fn converts_svg_font_to_ttf_font() {
        let ttf = svg_font_to_ttf(
            SVG_FONT,
            &Svg2TtfOptions {
                normalize: true,
                hinting: false,
            },
        )
        .unwrap();
        let metadata = fontmin_ttf::inspect_ttf(&ttf).unwrap();
        let face = FontRef::new(&ttf).unwrap();

        assert!(ttf.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert_eq!(metadata.family_name.as_deref(), Some("SVG Icons"));
        assert_eq!(metadata.glyph_count, 3);
        assert_eq!(metadata.units_per_em, 1000);
        assert_eq!(metadata.ascender, 850);
        assert_eq!(metadata.descender, -150);
        let charmap = face.charmap();
        let home = charmap.map('\u{E101}').unwrap();

        assert!(charmap.map('\u{E102}').is_some());
        assert!(face.outline_glyphs().get(home).is_some());
    }
}
