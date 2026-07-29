use std::collections::BTreeSet;

use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};

mod markup;
mod path;
mod ttf;

use markup::{
    attribute_f32, attribute_value, decode_unicode_value, element_tags, path_data_values, view_box,
};
use path::{
    Bounds, Point, ViewBox, bounds_for_contours, clamp_i16, clamp_u16, parse_path_data,
    transform_contour, transform_font_contour,
};

const UNITS_PER_EM: u16 = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SvgIcon {
    pub name: String,
    pub contents: String,
    pub unicode: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Svgs2TtfOptions {
    pub font_name: String,
    pub start_unicode: u32,
    pub ascent: i16,
    pub descent: i16,
    pub normalize: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Svg2TtfOptions {
    pub normalize: bool,
    pub hinting: bool,
}

impl Default for Svgs2TtfOptions {
    fn default() -> Self {
        Self {
            font_name: "iconfont".into(),
            start_unicode: 0xE001,
            ascent: 850,
            descent: -150,
            normalize: true,
        }
    }
}

impl Default for Svg2TtfOptions {
    fn default() -> Self {
        Self {
            normalize: true,
            hinting: false,
        }
    }
}

#[derive(Debug, Clone)]
struct IconGlyph {
    advance_width: u16,
    bbox: Bounds,
    contours: Vec<Vec<Point>>,
    unicode: u32,
}

pub fn svg_font_to_ttf(svg: &str, options: &Svg2TtfOptions) -> Result<Vec<u8>> {
    if svg.trim().is_empty() {
        return Err(FontminError::invalid_font(
            "expected SVG font markup for TTF generation",
        ));
    }

    let font_tag = element_tags(svg, "font")
        .into_iter()
        .next()
        .ok_or_else(|| FontminError::invalid_font("SVG font does not contain a <font> element"))?;
    let font_face_tag = element_tags(svg, "font-face").into_iter().next();
    let units_per_em = font_face_tag
        .as_deref()
        .and_then(|tag| attribute_f32(tag, "units-per-em"))
        .filter(|units| *units > 0.0)
        .unwrap_or(f32::from(UNITS_PER_EM));
    let scale = if options.normalize {
        f32::from(UNITS_PER_EM) / units_per_em
    } else {
        1.0
    };
    let font_name = font_face_tag
        .as_deref()
        .and_then(|tag| attribute_value(tag, "font-family"))
        .or_else(|| attribute_value(&font_tag, "id"))
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "svgfont".into());
    let ascent = font_face_tag
        .as_deref()
        .and_then(|tag| attribute_f32(tag, "ascent"))
        .map_or(850, |value| clamp_i16((value * scale).round()));
    let descent = font_face_tag
        .as_deref()
        .and_then(|tag| attribute_f32(tag, "descent"))
        .map_or(-150, |value| clamp_i16((value * scale).round()));

    if ascent <= descent {
        return Err(FontminError::config(
            "SVG font ascent must be greater than descent",
        ));
    }

    let default_advance = attribute_f32(&font_tag, "horiz-adv-x")
        .map_or(UNITS_PER_EM, |value| clamp_u16((value * scale).round()));
    let glyphs = svg_font_glyphs(svg, default_advance, scale)?;

    ttf::encode(
        &glyphs,
        &Svgs2TtfOptions {
            font_name,
            start_unicode: 0,
            ascent,
            descent,
            normalize: options.normalize,
        },
    )
}

pub fn svgs_to_ttf(inputs: Vec<SvgIcon>, options: &Svgs2TtfOptions) -> Result<Vec<u8>> {
    if inputs.is_empty() {
        return Err(FontminError::invalid_font(
            "expected at least one SVG icon for TTF generation",
        ));
    }
    if options.font_name.trim().is_empty() {
        return Err(FontminError::config("icon font name cannot be empty"));
    }
    if options.ascent <= options.descent {
        return Err(FontminError::config(
            "icon font ascent must be greater than descent",
        ));
    }

    let glyphs = icon_glyphs(inputs, options)?;

    ttf::encode(&glyphs, options)
}

fn icon_glyphs(inputs: Vec<SvgIcon>, options: &Svgs2TtfOptions) -> Result<Vec<IconGlyph>> {
    let mut next_unicode = options.start_unicode;
    let mut used = BTreeSet::new();
    let mut glyphs = Vec::with_capacity(inputs.len());

    for icon in inputs {
        let unicode = if let Some(unicode) = icon.unicode {
            unicode
        } else {
            while used.contains(&next_unicode) {
                next_unicode = next_unicode.checked_add(1).ok_or_else(|| {
                    FontminError::config("ran out of unicode values for SVG icons")
                })?;
            }
            let unicode = next_unicode;
            next_unicode = next_unicode
                .checked_add(1)
                .ok_or_else(|| FontminError::config("ran out of unicode values for SVG icons"))?;
            unicode
        };

        if unicode > u32::from(u16::MAX) {
            return Err(FontminError::unsupported(
                "svgs_to_ttf currently supports BMP unicode values only",
            ));
        }
        if !used.insert(unicode) {
            return Err(FontminError::config(format!(
                "duplicate unicode value U+{unicode:04X} for SVG icon {}",
                icon.name,
            )));
        }

        glyphs.push(parse_icon(icon, unicode, options)?);
    }

    Ok(glyphs)
}

fn svg_font_glyphs(svg: &str, default_advance: u16, scale: f32) -> Result<Vec<IconGlyph>> {
    let mut used = BTreeSet::new();
    let mut glyphs = Vec::new();

    for tag in element_tags(svg, "glyph") {
        let Some(unicode_value) = attribute_value(&tag, "unicode") else {
            continue;
        };
        let Some(unicode) = decode_unicode_value(&unicode_value) else {
            continue;
        };

        if unicode > u32::from(u16::MAX) {
            return Err(FontminError::unsupported(
                "svg_font_to_ttf currently supports BMP unicode values only",
            ));
        }
        if !used.insert(unicode) {
            return Err(FontminError::config(format!(
                "duplicate unicode value U+{unicode:04X} in SVG font",
            )));
        }

        let glyph_name =
            attribute_value(&tag, "glyph-name").unwrap_or_else(|| format!("U+{unicode:04X}"));
        let path = attribute_value(&tag, "d").ok_or_else(|| {
            FontminError::invalid_font(format!(
                "SVG font glyph {glyph_name} does not contain path data",
            ))
        })?;
        let mut contours = Vec::new();

        for contour in parse_path_data(&path)? {
            let contour = transform_font_contour(&contour, scale);
            if contour.len() >= 2 {
                contours.push(contour);
            }
        }

        if contours.is_empty() {
            return Err(FontminError::invalid_font(format!(
                "SVG font glyph {glyph_name} did not produce any drawable contours",
            )));
        }

        let advance_width = attribute_f32(&tag, "horiz-adv-x")
            .map_or(default_advance, |value| clamp_u16((value * scale).round()));
        let bbox = bounds_for_contours(&contours);

        glyphs.push(IconGlyph {
            advance_width,
            bbox,
            contours,
            unicode,
        });
    }

    if glyphs.is_empty() {
        return Err(FontminError::invalid_font(
            "SVG font does not contain any drawable glyphs",
        ));
    }

    Ok(glyphs)
}

fn parse_icon(icon: SvgIcon, unicode: u32, options: &Svgs2TtfOptions) -> Result<IconGlyph> {
    let SvgIcon { contents, name, .. } = icon;
    let view_box = view_box(&contents).unwrap_or(ViewBox {
        x: 0.0,
        y: 0.0,
        width: f32::from(UNITS_PER_EM),
        height: f32::from(UNITS_PER_EM),
    });
    let paths = path_data_values(&contents);

    if paths.is_empty() {
        return Err(FontminError::invalid_font(format!(
            "SVG icon {name} does not contain any path data",
        )));
    }

    let mut contours = Vec::new();

    for path in paths {
        for contour in parse_path_data(&path)? {
            let contour = transform_contour(&contour, view_box, options);
            if contour.len() >= 2 {
                contours.push(contour);
            }
        }
    }

    if contours.is_empty() {
        return Err(FontminError::invalid_font(format!(
            "SVG icon {name} did not produce any drawable contours",
        )));
    }

    let bbox = bounds_for_contours(&contours);

    Ok(IconGlyph {
        advance_width: UNITS_PER_EM,
        bbox,
        contours,
        unicode,
    })
}
