use std::collections::BTreeMap;

use fontmin_diagnostics::{FontminError, Result};
use fontmin_ttf::{OwnedSfntTable, OwnedTtfFont};

use super::{Bounds, IconGlyph, Point, Svgs2TtfOptions, UNITS_PER_EM};

#[derive(Debug, Clone)]
struct GlyphData {
    advance_width: u16,
    bbox: Bounds,
    contours: u16,
    data: Vec<u8>,
    lsb: i16,
    points: u16,
}

struct GlyphDataSet {
    glyf: Vec<u8>,
    glyphs: Vec<GlyphData>,
    offsets: Vec<u32>,
}

pub(super) fn encode(glyphs: &[IconGlyph], options: &Svgs2TtfOptions) -> Result<Vec<u8>> {
    let glyph_data = glyph_data(glyphs)?;
    let font_bounds = font_bounds(&glyph_data);
    let glyph_count =
        u16::try_from(glyph_data.glyphs.len()).map_err(|_| FontminError::ConvertFailed {
            message: "too many glyphs for TrueType font".into(),
        })?;
    let last_offset = glyph_data.offsets.last().copied().unwrap_or_default();
    let index_to_loc_format = i16::from(u16::try_from(last_offset / 2).is_err());
    let tables = vec![
        OwnedSfntTable {
            tag: "cmap".into(),
            data: cmap_table(glyphs)?,
        },
        OwnedSfntTable {
            tag: "glyf".into(),
            data: glyph_data.glyf.clone(),
        },
        OwnedSfntTable {
            tag: "head".into(),
            data: head_table(options, font_bounds, index_to_loc_format),
        },
        OwnedSfntTable {
            tag: "hhea".into(),
            data: hhea_table(options, &glyph_data, glyph_count),
        },
        OwnedSfntTable {
            tag: "hmtx".into(),
            data: hmtx_table(&glyph_data),
        },
        OwnedSfntTable {
            tag: "loca".into(),
            data: loca_table(&glyph_data.offsets, index_to_loc_format)?,
        },
        OwnedSfntTable {
            tag: "maxp".into(),
            data: maxp_table(&glyph_data, glyph_count),
        },
        OwnedSfntTable {
            tag: "name".into(),
            data: name_table(&options.font_name)?,
        },
        OwnedSfntTable {
            tag: "post".into(),
            data: post_table(),
        },
    ];

    fontmin_ttf::write_ttf(&OwnedTtfFont { tables })
        .map_err(|error| FontminError::convert_failed(error.to_string()))
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

fn utf16be(value: &str) -> Vec<u8> {
    value.encode_utf16().flat_map(u16::to_be_bytes).collect()
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
