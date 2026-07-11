use fontmin_diagnostics::{FontminError, Result};

use crate::outline::{GlyphPath, Point, Segment};
use crate::sfnt::StaticCffSource;

pub(crate) struct TrueTypeOutlineTables {
    pub head: Vec<u8>,
    pub hhea: Vec<u8>,
    pub hmtx: Vec<u8>,
    pub maxp: Vec<u8>,
    pub glyf: Vec<u8>,
    pub loca: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EncodedGlyf {
    pub glyf: Vec<u8>,
    pub loca: Vec<u8>,
    pub max_points: u16,
    pub max_contours: u16,
    glyph_bounds: Vec<Bounds>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Bounds {
    x_min: i16,
    y_min: i16,
    x_max: i16,
    y_max: i16,
}

#[derive(Debug, Clone, Copy)]
struct GlyphPoint {
    x: i16,
    y: i16,
    on_curve: bool,
}

pub(crate) fn encode_glyf_and_loca(glyphs: &[GlyphPath]) -> Result<EncodedGlyf> {
    let mut glyf = Vec::new();
    let mut offsets = Vec::with_capacity(glyphs.len() + 1);
    let mut max_points = 0;
    let mut max_contours = 0;
    let mut glyph_bounds = Vec::with_capacity(glyphs.len());

    offsets.push(0);
    for glyph in glyphs {
        let (encoded, point_count, contour_count, bounds) = encode_glyph(glyph)?;
        glyf.extend_from_slice(&encoded);
        if !glyf.len().is_multiple_of(2) {
            glyf.push(0);
        }
        offsets.push(checked_u32(glyf.len(), "glyf table is too large")?);
        max_points = max_points.max(point_count);
        max_contours = max_contours.max(contour_count);
        glyph_bounds.push(bounds);
    }

    let mut loca = Vec::with_capacity(offsets.len() * 4);
    for offset in offsets {
        write_u32(&mut loca, offset);
    }

    Ok(EncodedGlyf {
        glyf,
        loca,
        max_points,
        max_contours,
        glyph_bounds,
    })
}

pub(crate) fn build_truetype_outline_tables(
    source: &StaticCffSource<'_>,
    glyphs: &[GlyphPath],
) -> Result<TrueTypeOutlineTables> {
    if glyphs.len() != usize::from(source.num_glyphs) {
        return Err(FontminError::convert_failed(
            "CFF glyph count does not match maxp",
        ));
    }

    let encoded = encode_glyf_and_loca(glyphs)
        .map_err(|error| FontminError::convert_failed(error.to_string()))?;
    let head = build_head(source.table("head"), &encoded.glyph_bounds)?;
    let (hhea, hmtx) = build_horizontal_metrics(source, &encoded.glyph_bounds)?;
    let maxp = build_maxp(source.num_glyphs, encoded.max_points, encoded.max_contours);

    Ok(TrueTypeOutlineTables {
        head,
        hhea,
        hmtx,
        maxp,
        glyf: encoded.glyf,
        loca: encoded.loca,
    })
}

fn encode_glyph(glyph: &GlyphPath) -> Result<(Vec<u8>, u16, u16, Bounds)> {
    let mut points = Vec::new();
    let mut contour_endpoints = Vec::new();

    for contour in &glyph.contours {
        if contour.segments.is_empty() {
            continue;
        }

        points.push(round_point(contour.start)?);
        for segment in &contour.segments {
            match segment {
                Segment::Line(to) => points.push(round_point(*to)?),
                Segment::Quadratic { control, to } => {
                    let mut control = round_point(*control)?;
                    control.on_curve = false;
                    points.push(control);
                    points.push(round_point(*to)?);
                }
            }
        }

        let endpoint = points
            .len()
            .checked_sub(1)
            .ok_or_else(|| FontminError::invalid_font("glyph contour has no points"))?;
        contour_endpoints.push(checked_u16(endpoint, "glyph has too many points")?);
    }

    if points.is_empty() {
        return Ok((
            Vec::new(),
            0,
            0,
            Bounds {
                x_min: 0,
                y_min: 0,
                x_max: 0,
                y_max: 0,
            },
        ));
    }

    let point_count = checked_u16(points.len(), "glyph has too many points")?;
    let contour_count = checked_u16(contour_endpoints.len(), "glyph has too many contours")?;
    let bounds = bounds(&points);
    let mut output = Vec::new();

    write_i16(
        &mut output,
        i16::try_from(contour_count).map_err(|_| {
            FontminError::invalid_font("glyph has too many contours for simple glyph encoding")
        })?,
    );
    write_i16(&mut output, bounds.x_min);
    write_i16(&mut output, bounds.y_min);
    write_i16(&mut output, bounds.x_max);
    write_i16(&mut output, bounds.y_max);
    for endpoint in contour_endpoints {
        write_u16(&mut output, endpoint);
    }
    write_u16(&mut output, 0);

    for point in &points {
        output.push(u8::from(point.on_curve));
    }
    write_coordinate_deltas(&mut output, &points, true)?;
    write_coordinate_deltas(&mut output, &points, false)?;

    Ok((output, point_count, contour_count, bounds))
}

fn build_head(source: &[u8], glyph_bounds: &[Bounds]) -> Result<Vec<u8>> {
    if source.len() < 54 {
        return Err(FontminError::convert_failed("head table is truncated"));
    }

    let bounds = global_bounds(glyph_bounds);
    let mut head = source.to_vec();
    head[8..12].fill(0);
    head[36..38].copy_from_slice(&bounds.x_min.to_be_bytes());
    head[38..40].copy_from_slice(&bounds.y_min.to_be_bytes());
    head[40..42].copy_from_slice(&bounds.x_max.to_be_bytes());
    head[42..44].copy_from_slice(&bounds.y_max.to_be_bytes());
    head[50..52].copy_from_slice(&1i16.to_be_bytes());

    Ok(head)
}

fn build_horizontal_metrics(
    source: &StaticCffSource<'_>,
    glyph_bounds: &[Bounds],
) -> Result<(Vec<u8>, Vec<u8>)> {
    if source.table("hhea").len() < 36 {
        return Err(FontminError::convert_failed("hhea table is truncated"));
    }

    let mut advance_width_max = 0;
    let mut min_left_side_bearing = i16::MAX;
    let mut min_right_side_bearing = i16::MAX;
    let mut x_max_extent = i16::MIN;
    let mut hmtx = Vec::with_capacity(source.metrics.len() * 4);

    for (metric, bounds) in source.metrics.iter().zip(glyph_bounds) {
        advance_width_max = advance_width_max.max(metric.advance_width);
        min_left_side_bearing = min_left_side_bearing.min(metric.left_side_bearing);
        let extent = i32::from(metric.left_side_bearing) + i32::from(bounds.x_max);
        let right_side_bearing = i32::from(metric.advance_width) - extent;
        x_max_extent = x_max_extent.max(checked_i16(extent, "xMaxExtent is outside i16 range")?);
        min_right_side_bearing = min_right_side_bearing.min(checked_i16(
            right_side_bearing,
            "minRightSideBearing is outside i16 range",
        )?);

        write_u16(&mut hmtx, metric.advance_width);
        write_i16(&mut hmtx, metric.left_side_bearing);
    }

    let mut hhea = source.table("hhea").to_vec();
    hhea[10..12].copy_from_slice(&advance_width_max.to_be_bytes());
    hhea[12..14].copy_from_slice(&min_left_side_bearing.to_be_bytes());
    hhea[14..16].copy_from_slice(&min_right_side_bearing.to_be_bytes());
    hhea[16..18].copy_from_slice(&x_max_extent.to_be_bytes());
    hhea[34..36].copy_from_slice(&source.num_glyphs.to_be_bytes());

    Ok((hhea, hmtx))
}

fn build_maxp(num_glyphs: u16, max_points: u16, max_contours: u16) -> Vec<u8> {
    let mut maxp = Vec::with_capacity(32);

    write_u32(&mut maxp, 0x0001_0000);
    write_u16(&mut maxp, num_glyphs);
    for value in [max_points, max_contours, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0] {
        write_u16(&mut maxp, value);
    }

    maxp
}

fn round_point(point: Point) -> Result<GlyphPoint> {
    if !point.x.is_finite() || !point.y.is_finite() {
        return Err(FontminError::invalid_font(
            "glyph contains a non-finite coordinate",
        ));
    }

    let x = rounded_i16(point.x, "glyph x coordinate is outside i16 range")?;
    let y = rounded_i16(point.y, "glyph y coordinate is outside i16 range")?;

    Ok(GlyphPoint {
        x,
        y,
        on_curve: true,
    })
}

fn rounded_i16(value: f64, error: &'static str) -> Result<i16> {
    let value = value.round();
    if !(f64::from(i16::MIN)..=f64::from(i16::MAX)).contains(&value) {
        return Err(FontminError::invalid_font(error));
    }

    Ok(value as i16)
}

fn bounds(points: &[GlyphPoint]) -> Bounds {
    let mut x_min = i16::MAX;
    let mut y_min = i16::MAX;
    let mut x_max = i16::MIN;
    let mut y_max = i16::MIN;

    for point in points {
        x_min = x_min.min(point.x);
        y_min = y_min.min(point.y);
        x_max = x_max.max(point.x);
        y_max = y_max.max(point.y);
    }

    Bounds {
        x_min,
        y_min,
        x_max,
        y_max,
    }
}

fn global_bounds(glyph_bounds: &[Bounds]) -> Bounds {
    let mut bounds = Bounds {
        x_min: 0,
        y_min: 0,
        x_max: 0,
        y_max: 0,
    };
    let mut first = true;

    for glyph in glyph_bounds {
        if first {
            bounds = *glyph;
            first = false;
        } else {
            bounds.x_min = bounds.x_min.min(glyph.x_min);
            bounds.y_min = bounds.y_min.min(glyph.y_min);
            bounds.x_max = bounds.x_max.max(glyph.x_max);
            bounds.y_max = bounds.y_max.max(glyph.y_max);
        }
    }

    bounds
}

fn write_coordinate_deltas(
    output: &mut Vec<u8>,
    points: &[GlyphPoint],
    horizontal: bool,
) -> Result<()> {
    let mut previous = 0i16;

    for point in points {
        let coordinate = if horizontal { point.x } else { point.y };
        let delta = i32::from(coordinate) - i32::from(previous);
        let delta = i16::try_from(delta).map_err(|_| {
            FontminError::invalid_font("glyph coordinate delta is outside i16 range")
        })?;

        write_i16(output, delta);
        previous = coordinate;
    }

    Ok(())
}

fn checked_u16(value: usize, error: &'static str) -> Result<u16> {
    u16::try_from(value).map_err(|_| FontminError::invalid_font(error))
}

fn checked_u32(value: usize, error: &'static str) -> Result<u32> {
    u32::try_from(value).map_err(|_| FontminError::invalid_font(error))
}

fn checked_i16(value: i32, error: &'static str) -> Result<i16> {
    i16::try_from(value).map_err(|_| FontminError::convert_failed(error))
}

fn write_i16(output: &mut Vec<u8>, value: i16) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn write_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_be_bytes());
}
