use std::{collections::BTreeSet, fmt::Write as _};

use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};
use ttf_parser::{Face, GlyphId, OutlineBuilder};

mod icon;

pub use icon::{Svg2TtfOptions, SvgIcon, Svgs2TtfOptions, svg_font_to_ttf, svgs_to_ttf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ttf2SvgOptions {
    pub font_family: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct GlyphMapping {
    character: char,
    glyph_id: GlyphId,
}

pub fn ttf_to_svg(input: &[u8], options: &Ttf2SvgOptions) -> Result<String> {
    if !is_ttf(input) {
        return Err(FontminError::invalid_font(
            "expected TrueType sfnt data for SVG encoding",
        ));
    }

    let face = Face::parse(input, 0)
        .map_err(|error| FontminError::invalid_font(format!("failed to parse TTF: {error}")))?;
    let metadata = fontmin_ttf::inspect_ttf(input)?;
    let font_family = options
        .font_family
        .clone()
        .or(metadata.family_name)
        .unwrap_or_else(|| "fontmin".into());
    let font_id = font_id(&font_family);
    let units_per_em = face.units_per_em();
    let default_advance = face.glyph_hor_advance(GlyphId(0)).unwrap_or(units_per_em);
    let mappings = collect_glyph_mappings(&face);
    let mut svg = String::new();

    write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\"><defs><font id=\"{}\" horiz-adv-x=\"{}\"><font-face font-family=\"{}\" units-per-em=\"{}\" ascent=\"{}\" descent=\"{}\" /><missing-glyph horiz-adv-x=\"{}\" />",
        font_id,
        default_advance,
        escape_attribute(&font_family),
        units_per_em,
        face.ascender(),
        face.descender(),
        default_advance,
    )
    .expect("writing to string should not fail");

    for mapping in mappings {
        push_glyph(&mut svg, &face, mapping, default_advance);
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

fn collect_glyph_mappings(face: &Face<'_>) -> Vec<GlyphMapping> {
    let Some(cmap) = face.tables().cmap else {
        return Vec::new();
    };

    let mut codepoints = BTreeSet::new();

    for subtable in cmap.subtables {
        if subtable.is_unicode() {
            subtable.codepoints(|codepoint| {
                codepoints.insert(codepoint);
            });
        }
    }

    codepoints
        .into_iter()
        .filter_map(char::from_u32)
        .filter_map(|character| {
            let glyph_id = face.glyph_index(character)?;

            if glyph_id == GlyphId(0) {
                return None;
            }

            Some(GlyphMapping {
                character,
                glyph_id,
            })
        })
        .collect()
}

fn push_glyph(svg: &mut String, face: &Face<'_>, mapping: GlyphMapping, default_advance: u16) {
    let advance = face
        .glyph_hor_advance(mapping.glyph_id)
        .unwrap_or(default_advance);
    let path = glyph_path(face, mapping.glyph_id);

    write!(
        svg,
        "<glyph glyph-name=\"glyph{}\" unicode=\"{}\" horiz-adv-x=\"{}\"",
        mapping.glyph_id.0,
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

fn glyph_path(face: &Face<'_>, glyph_id: GlyphId) -> Option<String> {
    let mut builder = SvgPathBuilder::default();

    face.outline_glyph(glyph_id, &mut builder)?;

    if builder.path.is_empty() {
        None
    } else {
        Some(builder.path)
    }
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

impl OutlineBuilder for SvgPathBuilder {
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
    use ttf_parser::Face;

    use super::{
        Svg2TtfOptions, SvgIcon, Svgs2TtfOptions, Ttf2SvgOptions, svg_font_to_ttf, svgs_to_ttf,
        ttf_to_svg,
    };

    const ROBOTO: &[u8] = include_bytes!("../../../fixtures/fonts/ttf/roboto-regular.ttf");
    const SVG_FONT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><defs><font id="icons" horiz-adv-x="1000"><font-face font-family="SVG Icons" units-per-em="1000" ascent="850" descent="-150" /><missing-glyph horiz-adv-x="1000" /><glyph glyph-name="home" unicode="&#xE101;" horiz-adv-x="1000" d="M100 100 L900 100 L900 900 L100 900 Z" /><glyph glyph-name="user" unicode="&#xE102;" horiz-adv-x="1000" d="M500 100 C620 100 700 180 700 300 C700 420 620 500 500 500 C380 500 300 420 300 300 C300 180 380 100 500 100 Z" /></font></defs></svg>"#;
    const HOME_SVG: &str = r#"<svg viewBox="0 0 1000 1000"><path d="M100 500 L500 100 L900 500 L900 900 L100 900 Z"/></svg>"#;
    const USER_SVG: &str = r#"<svg viewBox="0 0 1000 1000"><path d="M500 100 C620 100 700 180 700 300 C700 420 620 500 500 500 C380 500 300 420 300 300 C300 180 380 100 500 100 Z M250 900 Q500 650 750 900 Z"/></svg>"#;

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
    fn combines_svg_icons_into_ttf_font() {
        let ttf = svgs_to_ttf(
            vec![
                SvgIcon {
                    name: "home".into(),
                    contents: HOME_SVG.into(),
                    unicode: Some(0xE101),
                },
                SvgIcon {
                    name: "user".into(),
                    contents: USER_SVG.into(),
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
        let face = Face::parse(&ttf, 0).unwrap();

        assert!(ttf.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert_eq!(metadata.family_name.as_deref(), Some("Icon Set"));
        assert_eq!(metadata.glyph_count, 3);
        assert_eq!(metadata.units_per_em, 1000);
        assert_eq!(metadata.ascender, 850);
        assert_eq!(metadata.descender, -150);
        assert!(face.glyph_index('\u{E101}').is_some());
        assert!(face.glyph_index('\u{E200}').is_some());
        assert!(
            face.outline_glyph(
                face.glyph_index('\u{E101}').unwrap(),
                &mut super::SvgPathBuilder::default()
            )
            .is_some()
        );
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
        let face = Face::parse(&ttf, 0).unwrap();

        assert!(ttf.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert_eq!(metadata.family_name.as_deref(), Some("SVG Icons"));
        assert_eq!(metadata.glyph_count, 3);
        assert_eq!(metadata.units_per_em, 1000);
        assert_eq!(metadata.ascender, 850);
        assert_eq!(metadata.descender, -150);
        assert!(face.glyph_index('\u{E101}').is_some());
        assert!(face.glyph_index('\u{E102}').is_some());
        assert!(
            face.outline_glyph(
                face.glyph_index('\u{E101}').unwrap(),
                &mut super::SvgPathBuilder::default()
            )
            .is_some()
        );
    }
}
