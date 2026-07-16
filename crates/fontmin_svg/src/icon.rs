use std::collections::{BTreeMap, BTreeSet};

use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};

const UNITS_PER_EM: u16 = 1000;
const CURVE_STEPS: u16 = 8;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Point {
    x: i16,
    y: i16,
}

#[derive(Debug, Clone, Copy, Default)]
struct RawPoint {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct Bounds {
    x_min: i16,
    y_min: i16,
    x_max: i16,
    y_max: i16,
}

#[derive(Debug, Clone, Copy)]
struct ViewBox {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone)]
struct GlyphData {
    advance_width: u16,
    bbox: Bounds,
    contours: u16,
    data: Vec<u8>,
    lsb: i16,
    points: u16,
}

#[derive(Debug, Clone)]
struct Table {
    data: Vec<u8>,
    tag: [u8; 4],
}

#[derive(Debug, Clone, Copy)]
enum PathToken {
    Command(char),
    Number(f32),
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

    write_ttf(
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

    write_ttf(&glyphs, options)
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
                &icon.name,
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

fn view_box(svg: &str) -> Option<ViewBox> {
    let value = attribute_value(svg, "viewBox")?;
    let numbers = numbers(&value);

    if numbers.len() != 4 || numbers[2] <= 0.0 || numbers[3] <= 0.0 {
        return None;
    }

    Some(ViewBox {
        x: numbers[0],
        y: numbers[1],
        width: numbers[2],
        height: numbers[3],
    })
}

fn path_data_values(svg: &str) -> Vec<String> {
    let mut values = Vec::new();
    let bytes = svg.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index].eq_ignore_ascii_case(&b'd') && is_attribute_boundary(bytes, index) {
            let mut cursor = index + 1;

            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'=' {
                cursor += 1;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor < bytes.len() && (bytes[cursor] == b'"' || bytes[cursor] == b'\'') {
                    let quote = bytes[cursor];
                    cursor += 1;
                    let value_start = cursor;
                    while cursor < bytes.len() && bytes[cursor] != quote {
                        cursor += 1;
                    }
                    if cursor <= bytes.len() {
                        values.push(svg[value_start..cursor].to_string());
                    }
                    index = cursor;
                }
            }
        }
        index += 1;
    }

    values
}

fn attribute_value(svg: &str, name: &str) -> Option<String> {
    let bytes = svg.as_bytes();
    let name_bytes = name.as_bytes();
    let mut index = 0;

    while index + name_bytes.len() <= bytes.len() {
        if bytes[index..].starts_with(name_bytes) && is_attribute_boundary(bytes, index) {
            let mut cursor = index + name_bytes.len();

            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'=' {
                cursor += 1;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor < bytes.len() && (bytes[cursor] == b'"' || bytes[cursor] == b'\'') {
                    let quote = bytes[cursor];
                    cursor += 1;
                    let value_start = cursor;
                    while cursor < bytes.len() && bytes[cursor] != quote {
                        cursor += 1;
                    }
                    return Some(svg[value_start..cursor].to_string());
                }
            }
        }
        index += 1;
    }

    None
}

fn attribute_f32(svg: &str, name: &str) -> Option<f32> {
    attribute_value(svg, name)?.parse().ok()
}

fn element_tags(svg: &str, name: &str) -> Vec<String> {
    let pattern = format!("<{name}");
    let bytes = svg.as_bytes();
    let mut tags = Vec::new();
    let mut index = 0;

    while let Some(relative_start) = svg[index..].find(&pattern) {
        let start = index + relative_start;
        let after_name = start + pattern.len();

        if !is_element_name_boundary(bytes, after_name) {
            index = after_name;
            continue;
        }

        let Some(relative_end) = svg[after_name..].find('>') else {
            break;
        };
        let end = after_name + relative_end + 1;

        tags.push(svg[start..end].to_string());
        index = end;
    }

    tags
}

fn is_element_name_boundary(bytes: &[u8], index: usize) -> bool {
    index >= bytes.len() || matches!(bytes[index], b' ' | b'\t' | b'\n' | b'\r' | b'/' | b'>')
}

fn is_attribute_boundary(bytes: &[u8], index: usize) -> bool {
    if index > 0 {
        let previous = bytes[index - 1];
        if previous.is_ascii_alphanumeric() || previous == b'-' || previous == b'_' {
            return false;
        }
    }

    true
}

fn decode_unicode_value(value: &str) -> Option<u32> {
    let value = value.trim();

    if let Some(hex) = value
        .strip_prefix("&#x")
        .or_else(|| value.strip_prefix("&#X"))
        .and_then(|value| value.strip_suffix(';'))
    {
        return u32::from_str_radix(hex, 16).ok();
    }
    if let Some(decimal) = value
        .strip_prefix("&#")
        .and_then(|value| value.strip_suffix(';'))
    {
        return decimal.parse().ok();
    }

    decode_xml_entities(value).chars().next().map(u32::from)
}

fn decode_xml_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn numbers(value: &str) -> Vec<f32> {
    tokenize_path(value)
        .into_iter()
        .filter_map(|token| match token {
            PathToken::Number(number) => Some(number),
            PathToken::Command(_) => None,
        })
        .collect()
}

fn parse_path_data(path: &str) -> Result<Vec<Vec<RawPoint>>> {
    let tokens = tokenize_path(path);
    let mut parser = PathParser::new(tokens);

    parser.parse()
}

fn tokenize_path(path: &str) -> Vec<PathToken> {
    let bytes = path.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];

        if byte.is_ascii_whitespace() || byte == b',' {
            index += 1;
            continue;
        }
        if byte.is_ascii_alphabetic() {
            tokens.push(PathToken::Command(char::from(byte)));
            index += 1;
            continue;
        }

        let start = index;

        if matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index < bytes.len() && bytes[index] == b'.' {
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
        }
        if index < bytes.len() && matches!(bytes[index], b'e' | b'E') {
            let exponent = index;
            index += 1;
            if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
                index += 1;
            }
            let digit_start = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            if digit_start == index {
                index = exponent;
            }
        }

        if start == index {
            index += 1;
            continue;
        }

        if let Ok(number) = path[start..index].parse::<f32>() {
            tokens.push(PathToken::Number(number));
        }
    }

    tokens
}

struct PathParser {
    command: Option<char>,
    contours: Vec<Vec<RawPoint>>,
    current: RawPoint,
    index: usize,
    start: RawPoint,
    tokens: Vec<PathToken>,
}

impl PathParser {
    fn new(tokens: Vec<PathToken>) -> Self {
        Self {
            command: None,
            contours: Vec::new(),
            current: RawPoint::default(),
            index: 0,
            start: RawPoint::default(),
            tokens,
        }
    }

    fn parse(&mut self) -> Result<Vec<Vec<RawPoint>>> {
        let mut contour = Vec::new();

        while self.index < self.tokens.len() {
            if let Some(command) = self.read_command() {
                self.command = Some(command);
            }

            let Some(command) = self.command else {
                return Err(FontminError::invalid_font(
                    "SVG path data must start with a command",
                ));
            };

            match command {
                'M' | 'm' => self.parse_move(command, &mut contour)?,
                'L' | 'l' => self.parse_line(command, &mut contour)?,
                'H' | 'h' => self.parse_horizontal(command, &mut contour)?,
                'V' | 'v' => self.parse_vertical(command, &mut contour)?,
                'Q' | 'q' => self.parse_quadratic(command, &mut contour)?,
                'C' | 'c' => self.parse_cubic(command, &mut contour)?,
                'Z' | 'z' => {
                    close_contour(&mut contour, self.start);
                    push_contour(&mut self.contours, &mut contour);
                    self.current = self.start;
                    self.command = None;
                }
                other => {
                    return Err(FontminError::unsupported(format!(
                        "SVG path command {other}",
                    )));
                }
            }
        }

        push_contour(&mut self.contours, &mut contour);

        Ok(std::mem::take(&mut self.contours))
    }

    fn parse_move(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        let mut first = true;

        while self.has_number() {
            let point = self.read_point(command.is_ascii_lowercase())?;

            if first {
                push_contour(&mut self.contours, contour);
                self.current = point;
                self.start = point;
                contour.push(point);
                first = false;
            } else {
                self.current = point;
                contour.push(point);
            }
        }

        self.command = Some(if command == 'm' { 'l' } else { 'L' });

        Ok(())
    }

    fn parse_line(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        while self.has_number() {
            let point = self.read_point(command.is_ascii_lowercase())?;

            self.current = point;
            contour.push(point);
        }

        Ok(())
    }

    fn parse_horizontal(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        while self.has_number() {
            let value = self.read_number()?;
            let x = if command == 'h' {
                self.current.x + value
            } else {
                value
            };

            self.current = RawPoint {
                x,
                y: self.current.y,
            };
            contour.push(self.current);
        }

        Ok(())
    }

    fn parse_vertical(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        while self.has_number() {
            let value = self.read_number()?;
            let y = if command == 'v' {
                self.current.y + value
            } else {
                value
            };

            self.current = RawPoint {
                x: self.current.x,
                y,
            };
            contour.push(self.current);
        }

        Ok(())
    }

    fn parse_quadratic(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        while self.has_number() {
            let control = self.read_point(command.is_ascii_lowercase())?;
            let end = self.read_point(command.is_ascii_lowercase())?;
            let start = self.current;

            for step in 1..=CURVE_STEPS {
                contour.push(quadratic_point(
                    start,
                    control,
                    end,
                    f32::from(step) / f32::from(CURVE_STEPS),
                ));
            }
            self.current = end;
        }

        Ok(())
    }

    fn parse_cubic(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        while self.has_number() {
            let first = self.read_point(command.is_ascii_lowercase())?;
            let second = self.read_point(command.is_ascii_lowercase())?;
            let end = self.read_point(command.is_ascii_lowercase())?;
            let start = self.current;

            for step in 1..=CURVE_STEPS {
                contour.push(cubic_point(
                    start,
                    first,
                    second,
                    end,
                    f32::from(step) / f32::from(CURVE_STEPS),
                ));
            }
            self.current = end;
        }

        Ok(())
    }

    fn read_command(&mut self) -> Option<char> {
        let Some(PathToken::Command(command)) = self.tokens.get(self.index).copied() else {
            return None;
        };

        self.index += 1;

        Some(command)
    }

    fn read_number(&mut self) -> Result<f32> {
        let Some(PathToken::Number(number)) = self.tokens.get(self.index).copied() else {
            return Err(FontminError::invalid_font("expected SVG path number"));
        };

        self.index += 1;

        Ok(number)
    }

    fn read_point(&mut self, relative: bool) -> Result<RawPoint> {
        let x = self.read_number()?;
        let y = self.read_number()?;

        if relative {
            Ok(RawPoint {
                x: self.current.x + x,
                y: self.current.y + y,
            })
        } else {
            Ok(RawPoint { x, y })
        }
    }

    fn has_number(&self) -> bool {
        matches!(self.tokens.get(self.index), Some(PathToken::Number(_)))
    }
}

fn push_contour(contours: &mut Vec<Vec<RawPoint>>, contour: &mut Vec<RawPoint>) {
    if contour.len() >= 2 {
        contours.push(std::mem::take(contour));
    } else {
        contour.clear();
    }
}

fn close_contour(contour: &mut Vec<RawPoint>, start: RawPoint) {
    if contour.last().is_some_and(|point| {
        (point.x - start.x).abs() > f32::EPSILON || (point.y - start.y).abs() > f32::EPSILON
    }) {
        contour.push(start);
    }
}

fn quadratic_point(start: RawPoint, control: RawPoint, end: RawPoint, t: f32) -> RawPoint {
    let inverse = 1.0 - t;

    RawPoint {
        x: inverse.mul_add(inverse * start.x, 2.0 * inverse * t * control.x) + t * t * end.x,
        y: inverse.mul_add(inverse * start.y, 2.0 * inverse * t * control.y) + t * t * end.y,
    }
}

fn cubic_point(
    start: RawPoint,
    first: RawPoint,
    second: RawPoint,
    end: RawPoint,
    t: f32,
) -> RawPoint {
    let inverse = 1.0 - t;

    RawPoint {
        x: inverse.powi(3) * start.x
            + 3.0 * inverse.powi(2) * t * first.x
            + 3.0 * inverse * t.powi(2) * second.x
            + t.powi(3) * end.x,
        y: inverse.powi(3) * start.y
            + 3.0 * inverse.powi(2) * t * first.y
            + 3.0 * inverse * t.powi(2) * second.y
            + t.powi(3) * end.y,
    }
}

fn transform_contour(
    contour: &[RawPoint],
    view_box: ViewBox,
    options: &Svgs2TtfOptions,
) -> Vec<Point> {
    let units = f32::from(UNITS_PER_EM);
    let scale = if options.normalize {
        units / view_box.width.max(view_box.height)
    } else {
        1.0
    };
    let mut points = Vec::with_capacity(contour.len());

    for point in contour {
        let transformed = Point {
            x: clamp_i16(((point.x - view_box.x) * scale).round()),
            y: clamp_i16((f32::from(options.ascent) - (point.y - view_box.y) * scale).round()),
        };

        if points.last().copied() != Some(transformed) {
            points.push(transformed);
        }
    }
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }

    points
}

fn transform_font_contour(contour: &[RawPoint], scale: f32) -> Vec<Point> {
    let mut points = Vec::with_capacity(contour.len());

    for point in contour {
        let transformed = Point {
            x: clamp_i16((point.x * scale).round()),
            y: clamp_i16((point.y * scale).round()),
        };

        if points.last().copied() != Some(transformed) {
            points.push(transformed);
        }
    }
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }

    points
}

#[allow(clippy::cast_possible_truncation)]
fn clamp_i16(value: f32) -> i16 {
    value.clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn clamp_u16(value: f32) -> u16 {
    value.clamp(0.0, f32::from(u16::MAX)) as u16
}

fn bounds_for_contours(contours: &[Vec<Point>]) -> Bounds {
    let mut bounds = Bounds {
        x_min: i16::MAX,
        y_min: i16::MAX,
        x_max: i16::MIN,
        y_max: i16::MIN,
    };

    for point in contours.iter().flatten() {
        bounds.x_min = bounds.x_min.min(point.x);
        bounds.y_min = bounds.y_min.min(point.y);
        bounds.x_max = bounds.x_max.max(point.x);
        bounds.y_max = bounds.y_max.max(point.y);
    }

    bounds
}

fn write_ttf(glyphs: &[IconGlyph], options: &Svgs2TtfOptions) -> Result<Vec<u8>> {
    let glyph_data = glyph_data(glyphs)?;
    let font_bounds = font_bounds(&glyph_data);
    let glyph_count =
        u16::try_from(glyph_data.glyphs.len()).map_err(|_| FontminError::ConvertFailed {
            message: "too many glyphs for TrueType font".into(),
        })?;
    let last_offset = glyph_data.offsets.last().copied().unwrap_or_default();
    let index_to_loc_format = i16::from(u16::try_from(last_offset / 2).is_err());
    let mut tables = vec![
        Table {
            tag: *b"cmap",
            data: cmap_table(glyphs)?,
        },
        Table {
            tag: *b"glyf",
            data: glyph_data.glyf.clone(),
        },
        Table {
            tag: *b"head",
            data: head_table(options, font_bounds, index_to_loc_format),
        },
        Table {
            tag: *b"hhea",
            data: hhea_table(options, &glyph_data, glyph_count),
        },
        Table {
            tag: *b"hmtx",
            data: hmtx_table(&glyph_data),
        },
        Table {
            tag: *b"loca",
            data: loca_table(&glyph_data.offsets, index_to_loc_format)?,
        },
        Table {
            tag: *b"maxp",
            data: maxp_table(&glyph_data, glyph_count),
        },
        Table {
            tag: *b"name",
            data: name_table(&options.font_name)?,
        },
        Table {
            tag: *b"post",
            data: post_table(),
        },
    ];

    tables.sort_by_key(|table| table.tag);

    font_file(&tables)
}

struct GlyphDataSet {
    glyf: Vec<u8>,
    glyphs: Vec<GlyphData>,
    offsets: Vec<u32>,
}

fn glyph_data(glyphs: &[IconGlyph]) -> Result<GlyphDataSet> {
    let mut glyf = Vec::new();
    let mut offsets = Vec::with_capacity(glyphs.len() + 2);
    let mut data = Vec::with_capacity(glyphs.len() + 1);

    offsets.push(0);
    data.push(GlyphData {
        advance_width: UNITS_PER_EM,
        bbox: Bounds::default(),
        contours: 0,
        data: Vec::new(),
        lsb: 0,
        points: 0,
    });
    offsets.push(0);

    for glyph in glyphs {
        let simple = simple_glyph(glyph)?;

        glyf.extend(&simple.data);
        pad_to_even(&mut glyf);
        offsets.push(
            u32::try_from(glyf.len()).map_err(|_| FontminError::ConvertFailed {
                message: "generated glyf table is too large".into(),
            })?,
        );
        data.push(simple);
    }

    Ok(GlyphDataSet {
        glyf,
        glyphs: data,
        offsets,
    })
}

fn simple_glyph(glyph: &IconGlyph) -> Result<GlyphData> {
    let point_count = glyph.contours.iter().map(Vec::len).sum::<usize>();
    let contour_count = glyph.contours.len();
    let mut data = Vec::new();

    push_i16(
        &mut data,
        i16::try_from(contour_count).map_err(|_| FontminError::ConvertFailed {
            message: "SVG icon has too many contours".into(),
        })?,
    );
    push_i16(&mut data, glyph.bbox.x_min);
    push_i16(&mut data, glyph.bbox.y_min);
    push_i16(&mut data, glyph.bbox.x_max);
    push_i16(&mut data, glyph.bbox.y_max);

    let mut endpoint = 0usize;
    for contour in &glyph.contours {
        endpoint += contour.len();
        push_u16(
            &mut data,
            u16::try_from(endpoint - 1).map_err(|_| FontminError::ConvertFailed {
                message: "SVG icon has too many points".into(),
            })?,
        );
    }

    push_u16(&mut data, 0);

    let points = glyph.contours.iter().flatten().copied().collect::<Vec<_>>();
    let mut flags = Vec::with_capacity(points.len());
    let mut x_bytes = Vec::new();
    let mut y_bytes = Vec::new();
    let mut previous = Point::default();

    for point in &points {
        let dx = i32::from(point.x) - i32::from(previous.x);
        let dy = i32::from(point.y) - i32::from(previous.y);
        let mut flag = 0x01;

        encode_coordinate(dx, true, &mut flag, &mut x_bytes)?;
        encode_coordinate(dy, false, &mut flag, &mut y_bytes)?;
        flags.push(flag);
        previous = *point;
    }

    data.extend(flags);
    data.extend(x_bytes);
    data.extend(y_bytes);

    Ok(GlyphData {
        advance_width: glyph.advance_width,
        bbox: glyph.bbox,
        contours: u16::try_from(contour_count).map_err(|_| FontminError::ConvertFailed {
            message: "SVG icon has too many contours".into(),
        })?,
        data,
        lsb: glyph.bbox.x_min,
        points: u16::try_from(point_count).map_err(|_| FontminError::ConvertFailed {
            message: "SVG icon has too many points".into(),
        })?,
    })
}

fn encode_coordinate(delta: i32, is_x: bool, flag: &mut u8, bytes: &mut Vec<u8>) -> Result<()> {
    let short_flag = if is_x { 0x02 } else { 0x04 };
    let same_or_positive_flag = if is_x { 0x10 } else { 0x20 };

    if delta == 0 {
        *flag |= same_or_positive_flag;
    } else if (1..=255).contains(&delta) {
        *flag |= short_flag | same_or_positive_flag;
        bytes.push(u8::try_from(delta).expect("checked positive glyph delta fits u8"));
    } else if (-255..=-1).contains(&delta) {
        *flag |= short_flag;
        bytes.push(
            u8::try_from(delta.unsigned_abs()).expect("checked negative glyph delta fits u8"),
        );
    } else {
        push_i16(
            bytes,
            i16::try_from(delta).map_err(|_| FontminError::ConvertFailed {
                message: "SVG icon coordinate delta is too large for TrueType glyph data".into(),
            })?,
        );
    }

    Ok(())
}

fn cmap_table(glyphs: &[IconGlyph]) -> Result<Vec<u8>> {
    let mut mappings = BTreeMap::new();

    for (index, glyph) in glyphs.iter().enumerate() {
        mappings.insert(
            u16::try_from(glyph.unicode)
                .map_err(|_| FontminError::unsupported("non-BMP SVG icon unicode"))?,
            u16::try_from(index + 1).map_err(|_| FontminError::ConvertFailed {
                message: "too many SVG icons for cmap format 4".into(),
            })?,
        );
    }

    let seg_count = u16::try_from(mappings.len() + 1).map_err(|_| FontminError::ConvertFailed {
        message: "too many SVG icons for cmap format 4".into(),
    })?;
    let seg_count_x2 = seg_count
        .checked_mul(2)
        .ok_or_else(|| FontminError::ConvertFailed {
            message: "too many SVG icons for cmap format 4".into(),
        })?;
    let entry_selector = floor_log2(seg_count);
    let search_range = 2u16
        .pow(u32::from(entry_selector))
        .checked_mul(2)
        .ok_or_else(|| FontminError::ConvertFailed {
            message: "too many SVG icons for cmap format 4".into(),
        })?;
    let range_shift =
        seg_count_x2
            .checked_sub(search_range)
            .ok_or_else(|| FontminError::ConvertFailed {
                message: "invalid cmap search range".into(),
            })?;
    let length = u16::try_from(16 + usize::from(seg_count) * 8).map_err(|_| {
        FontminError::ConvertFailed {
            message: "too many SVG icons for cmap format 4".into(),
        }
    })?;
    let mut subtable = Vec::new();

    push_u16(&mut subtable, 4);
    push_u16(&mut subtable, length);
    push_u16(&mut subtable, 0);
    push_u16(&mut subtable, seg_count_x2);
    push_u16(&mut subtable, search_range);
    push_u16(&mut subtable, entry_selector);
    push_u16(&mut subtable, range_shift);

    for codepoint in mappings.keys() {
        push_u16(&mut subtable, *codepoint);
    }
    push_u16(&mut subtable, 0xFFFF);
    push_u16(&mut subtable, 0);
    for codepoint in mappings.keys() {
        push_u16(&mut subtable, *codepoint);
    }
    push_u16(&mut subtable, 0xFFFF);
    for (codepoint, glyph_id) in &mappings {
        push_u16(&mut subtable, glyph_id.wrapping_sub(*codepoint));
    }
    push_u16(&mut subtable, 1);
    for _ in 0..seg_count {
        push_u16(&mut subtable, 0);
    }

    let mut table = Vec::new();

    push_u16(&mut table, 0);
    push_u16(&mut table, 1);
    push_u16(&mut table, 3);
    push_u16(&mut table, 1);
    push_u32(&mut table, 12);
    table.extend(subtable);

    Ok(table)
}

fn floor_log2(value: u16) -> u16 {
    let mut selector = 0;
    let mut power = 1;

    while power <= value / 2 {
        power *= 2;
        selector += 1;
    }

    selector
}

fn head_table(options: &Svgs2TtfOptions, bounds: Bounds, index_to_loc_format: i16) -> Vec<u8> {
    let mut table = Vec::new();

    push_u32(&mut table, 0x0001_0000);
    push_u32(&mut table, 0x0001_0000);
    push_u32(&mut table, 0);
    push_u32(&mut table, 0x5F0F_3CF5);
    push_u16(&mut table, 0x000B);
    push_u16(&mut table, UNITS_PER_EM);
    push_u64(&mut table, 0);
    push_u64(&mut table, 0);
    push_i16(&mut table, bounds.x_min);
    push_i16(&mut table, bounds.y_min.min(options.descent));
    push_i16(&mut table, bounds.x_max);
    push_i16(&mut table, bounds.y_max.max(options.ascent));
    push_u16(&mut table, 0);
    push_u16(&mut table, 8);
    push_i16(&mut table, 2);
    push_i16(&mut table, index_to_loc_format);
    push_i16(&mut table, 0);

    table
}

fn hhea_table(options: &Svgs2TtfOptions, glyph_data: &GlyphDataSet, glyph_count: u16) -> Vec<u8> {
    let advance_width_max = glyph_data
        .glyphs
        .iter()
        .map(|glyph| glyph.advance_width)
        .max()
        .unwrap_or(UNITS_PER_EM);
    let min_left_side_bearing = glyph_data
        .glyphs
        .iter()
        .map(|glyph| glyph.lsb)
        .min()
        .unwrap_or_default();
    let min_right_side_bearing = glyph_data
        .glyphs
        .iter()
        .map(|glyph| {
            i16::try_from(i32::from(glyph.advance_width) - i32::from(glyph.bbox.x_max))
                .unwrap_or_default()
        })
        .min()
        .unwrap_or_default();
    let x_max_extent = glyph_data
        .glyphs
        .iter()
        .map(|glyph| {
            glyph
                .lsb
                .saturating_add(glyph.bbox.x_max - glyph.bbox.x_min)
        })
        .max()
        .unwrap_or_default();
    let mut table = Vec::new();

    push_u32(&mut table, 0x0001_0000);
    push_i16(&mut table, options.ascent);
    push_i16(&mut table, options.descent);
    push_i16(&mut table, 0);
    push_u16(&mut table, advance_width_max);
    push_i16(&mut table, min_left_side_bearing);
    push_i16(&mut table, min_right_side_bearing);
    push_i16(&mut table, x_max_extent);
    push_i16(&mut table, 1);
    push_i16(&mut table, 0);
    push_i16(&mut table, 0);
    push_i16(&mut table, 0);
    push_i16(&mut table, 0);
    push_i16(&mut table, 0);
    push_i16(&mut table, 0);
    push_i16(&mut table, 0);
    push_u16(&mut table, glyph_count);

    table
}

fn hmtx_table(glyph_data: &GlyphDataSet) -> Vec<u8> {
    let mut table = Vec::new();

    for glyph in &glyph_data.glyphs {
        push_u16(&mut table, glyph.advance_width);
        push_i16(&mut table, glyph.lsb);
    }

    table
}

fn loca_table(offsets: &[u32], index_to_loc_format: i16) -> Result<Vec<u8>> {
    let mut table = Vec::new();

    if index_to_loc_format == 0 {
        for offset in offsets {
            push_u16(
                &mut table,
                u16::try_from(offset / 2).map_err(|_| FontminError::ConvertFailed {
                    message: "short loca offset overflow".into(),
                })?,
            );
        }
    } else {
        for offset in offsets {
            push_u32(&mut table, *offset);
        }
    }

    Ok(table)
}

fn maxp_table(glyph_data: &GlyphDataSet, glyph_count: u16) -> Vec<u8> {
    let mut table = Vec::new();

    push_u32(&mut table, 0x0001_0000);
    push_u16(&mut table, glyph_count);
    push_u16(
        &mut table,
        glyph_data
            .glyphs
            .iter()
            .map(|glyph| glyph.points)
            .max()
            .unwrap_or_default(),
    );
    push_u16(
        &mut table,
        glyph_data
            .glyphs
            .iter()
            .map(|glyph| glyph.contours)
            .max()
            .unwrap_or_default(),
    );
    push_u16(&mut table, 0);
    push_u16(&mut table, 0);
    push_u16(&mut table, 2);
    push_u16(&mut table, 0);
    push_u16(&mut table, 0);
    push_u16(&mut table, 0);
    push_u16(&mut table, 0);
    push_u16(&mut table, 0);
    push_u16(&mut table, 0);
    push_u16(&mut table, 0);
    push_u16(&mut table, 0);

    table
}

fn name_table(font_name: &str) -> Result<Vec<u8>> {
    let full_name = format!("{font_name} Regular");
    let postscript_name = font_name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>();
    let names = [
        (1u16, font_name.to_string()),
        (2, "Regular".into()),
        (4, full_name),
        (
            6,
            if postscript_name.is_empty() {
                "IconFont".into()
            } else {
                postscript_name
            },
        ),
    ];
    let count = u16::try_from(names.len()).map_err(|_| FontminError::ConvertFailed {
        message: "too many name records for TrueType name table".into(),
    })?;
    let storage_offset =
        u16::try_from(6 + usize::from(count) * 12).map_err(|_| FontminError::ConvertFailed {
            message: "name table header is too large".into(),
        })?;
    let mut records = Vec::new();
    let mut storage = Vec::new();

    for (name_id, value) in names {
        let encoded = utf16be(&value);
        let encoded_len =
            u16::try_from(encoded.len()).map_err(|_| FontminError::ConvertFailed {
                message: "icon font name is too large for TrueType name table".into(),
            })?;
        let storage_len =
            u16::try_from(storage.len()).map_err(|_| FontminError::ConvertFailed {
                message: "name table storage is too large".into(),
            })?;

        push_u16(&mut records, 3);
        push_u16(&mut records, 1);
        push_u16(&mut records, 0x0409);
        push_u16(&mut records, name_id);
        push_u16(&mut records, encoded_len);
        push_u16(&mut records, storage_len);
        storage.extend(encoded);
    }

    let mut table = Vec::new();

    push_u16(&mut table, 0);
    push_u16(&mut table, count);
    push_u16(&mut table, storage_offset);
    table.extend(records);
    table.extend(storage);

    Ok(table)
}

fn post_table() -> Vec<u8> {
    let mut table = Vec::new();

    push_u32(&mut table, 0x0003_0000);
    push_u32(&mut table, 0);
    push_i16(&mut table, 0);
    push_i16(&mut table, 0);
    push_u32(&mut table, 0);
    push_u32(&mut table, 0);
    push_u32(&mut table, 0);
    push_u32(&mut table, 0);
    push_u32(&mut table, 0);

    table
}

fn font_bounds(glyph_data: &GlyphDataSet) -> Bounds {
    let mut bounds = Bounds {
        x_min: i16::MAX,
        y_min: i16::MAX,
        x_max: i16::MIN,
        y_max: i16::MIN,
    };

    for glyph in glyph_data.glyphs.iter().skip(1) {
        bounds.x_min = bounds.x_min.min(glyph.bbox.x_min);
        bounds.y_min = bounds.y_min.min(glyph.bbox.y_min);
        bounds.x_max = bounds.x_max.max(glyph.bbox.x_max);
        bounds.y_max = bounds.y_max.max(glyph.bbox.y_max);
    }

    if bounds.x_min == i16::MAX {
        Bounds::default()
    } else {
        bounds
    }
}

fn font_file(tables: &[Table]) -> Result<Vec<u8>> {
    let num_tables = u16::try_from(tables.len()).map_err(|_| FontminError::ConvertFailed {
        message: "too many tables for TrueType font".into(),
    })?;
    let entry_selector = floor_log2(num_tables);
    let search_range = 2u16.pow(u32::from(entry_selector)) * 16;
    let range_shift = num_tables * 16 - search_range;
    let mut records = Vec::new();
    let mut table_data = Vec::new();
    let mut offset = 12 + usize::from(num_tables) * 16;
    let mut head_offset = 0usize;

    for table in tables {
        let padded_offset = align4(offset);

        while offset < padded_offset {
            table_data.push(0);
            offset += 1;
        }
        if table.tag == *b"head" {
            head_offset = padded_offset;
        }

        records.extend(table.tag);
        push_u32(&mut records, checksum(&table.data));
        push_u32(
            &mut records,
            u32::try_from(padded_offset).map_err(|_| FontminError::ConvertFailed {
                message: "TrueType table offset is too large".into(),
            })?,
        );
        push_u32(
            &mut records,
            u32::try_from(table.data.len()).map_err(|_| FontminError::ConvertFailed {
                message: "TrueType table is too large".into(),
            })?,
        );

        table_data.extend(&table.data);
        offset = padded_offset + table.data.len();
        while table_data.len() % 4 != 0 {
            table_data.push(0);
            offset += 1;
        }
    }

    let mut font = Vec::new();

    push_u32(&mut font, 0x0001_0000);
    push_u16(&mut font, num_tables);
    push_u16(&mut font, search_range);
    push_u16(&mut font, entry_selector);
    push_u16(&mut font, range_shift);
    font.extend(records);
    font.extend(table_data);

    let adjustment = 0xB1B0_AFBAu32.wrapping_sub(checksum(&font));
    font[head_offset + 8..head_offset + 12].copy_from_slice(&adjustment.to_be_bytes());

    Ok(font)
}

fn checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;

    for chunk in data.chunks(4) {
        let mut bytes = [0u8; 4];

        bytes[..chunk.len()].copy_from_slice(chunk);
        sum = sum.wrapping_add(u32::from_be_bytes(bytes));
    }

    sum
}

fn utf16be(value: &str) -> Vec<u8> {
    value.encode_utf16().flat_map(u16::to_be_bytes).collect()
}

fn align4(value: usize) -> usize {
    (value + 3) & !3
}

fn pad_to_even(value: &mut Vec<u8>) {
    if !value.len().is_multiple_of(2) {
        value.push(0);
    }
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend(value.to_be_bytes());
}

fn push_i16(output: &mut Vec<u8>, value: i16) {
    output.extend(value.to_be_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend(value.to_be_bytes());
}

fn push_u64(output: &mut Vec<u8>, value: u64) {
    output.extend(value.to_be_bytes());
}
