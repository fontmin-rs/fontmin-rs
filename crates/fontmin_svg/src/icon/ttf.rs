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
            tag: "OS/2".into(),
            data: os2_table(glyphs, options),
        },
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

fn os2_table(glyphs: &[IconGlyph], options: &Svgs2TtfOptions) -> Vec<u8> {
    let mut table = Vec::with_capacity(96);
    let mut unicode_ranges = [0u32; 4];
    let mut bmp_codepoints = glyphs
        .iter()
        .map(|glyph| glyph.unicode)
        .filter(|unicode| u16::try_from(*unicode).is_ok());
    let first_bmp = bmp_codepoints.next();
    let (first_character, mut last_character) = first_bmp.map_or((0xFFFF, 0xFFFF), |first| {
        let mut min = first;
        let mut max = first;
        for unicode in bmp_codepoints {
            min = min.min(unicode);
            max = max.max(unicode);
        }
        (
            u16::try_from(min).expect("checked BMP codepoint fits u16"),
            u16::try_from(max).expect("checked BMP codepoint fits u16"),
        )
    });
    for unicode in glyphs.iter().map(|glyph| glyph.unicode) {
        if unicode <= 0x007F {
            unicode_ranges[0] |= 1;
        }
        if (0x0080..=0x00FF).contains(&unicode) {
            unicode_ranges[0] |= 1 << 1;
        }
        if (0xE000..=0xF8FF).contains(&unicode) {
            unicode_ranges[1] |= 1 << (60 - 32);
        }
        if u16::try_from(unicode).is_err() {
            unicode_ranges[1] |= 1 << (57 - 32);
            last_character = 0xFFFF;
        }
    }

    push_u16(&mut table, 4);
    push_i16(
        &mut table,
        i16::try_from(UNITS_PER_EM).expect("units per em fits i16"),
    );
    push_u16(&mut table, 400);
    push_u16(&mut table, 5);
    push_u16(&mut table, 0);
    push_i16(&mut table, 650);
    push_i16(&mut table, 600);
    push_i16(&mut table, 0);
    push_i16(&mut table, 75);
    push_i16(&mut table, 650);
    push_i16(&mut table, 600);
    push_i16(&mut table, 0);
    push_i16(&mut table, 350);
    push_i16(&mut table, 50);
    push_i16(&mut table, 250);
    push_i16(&mut table, 0);
    table.extend([0; 10]);
    for range in unicode_ranges {
        push_u32(&mut table, range);
    }
    table.extend(*b"FMIN");
    push_u16(&mut table, 0x0040);
    push_u16(&mut table, first_character);
    push_u16(&mut table, last_character);
    push_i16(&mut table, options.ascent);
    push_i16(&mut table, options.descent);
    push_i16(&mut table, 0);
    push_u16(&mut table, options.ascent.unsigned_abs());
    push_u16(&mut table, options.descent.unsigned_abs());
    push_u32(&mut table, u32::from(unicode_ranges[0] != 0));
    push_u32(&mut table, 0);
    push_i16(&mut table, 500);
    push_i16(&mut table, 700);
    push_u16(&mut table, 0);
    push_u16(&mut table, 0x0020);
    push_u16(&mut table, 2);

    debug_assert_eq!(table.len(), 96);

    table
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
            glyph.unicode,
            u16::try_from(index + 1).map_err(|_| FontminError::ConvertFailed {
                message: "too many SVG icons for cmap".into(),
            })?,
        );
    }

    let bmp_mappings = mappings
        .iter()
        .filter_map(|(codepoint, glyph_id)| {
            if *codepoint <= 0xFFFE {
                Some((
                    u16::try_from(*codepoint).expect("checked BMP codepoint fits u16"),
                    *glyph_id,
                ))
            } else {
                None
            }
        })
        .collect::<BTreeMap<_, _>>();
    let needs_format_12 = mappings.keys().any(|codepoint| *codepoint > 0xFFFE);
    let format_4 = (!bmp_mappings.is_empty())
        .then(|| cmap_format_4(&bmp_mappings))
        .transpose()?;
    let format_12 = needs_format_12
        .then(|| cmap_format_12(&mappings))
        .transpose()?;
    let record_count = usize::from(format_4.is_some()) + usize::from(format_12.is_some());
    let header_size = 4 + record_count * 8;
    let mut offset = header_size;
    let mut table = Vec::new();

    push_u16(&mut table, 0);
    push_u16(
        &mut table,
        u16::try_from(record_count).expect("cmap record count is at most two"),
    );
    if let Some(subtable) = &format_4 {
        push_u16(&mut table, 3);
        push_u16(&mut table, 1);
        push_u32(
            &mut table,
            u32::try_from(offset).expect("cmap format 4 offset fits u32"),
        );
        offset += subtable.len();
    }
    if format_12.is_some() {
        push_u16(&mut table, 3);
        push_u16(&mut table, 10);
        push_u32(
            &mut table,
            u32::try_from(offset).expect("cmap format 12 offset fits u32"),
        );
    }
    if let Some(subtable) = format_4 {
        table.extend(subtable);
    }
    if let Some(subtable) = format_12 {
        table.extend(subtable);
    }

    Ok(table)
}

fn cmap_format_4(mappings: &BTreeMap<u16, u16>) -> Result<Vec<u8>> {
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
    let mut table = Vec::new();

    push_u16(&mut table, 4);
    push_u16(&mut table, length);
    push_u16(&mut table, 0);
    push_u16(&mut table, seg_count_x2);
    push_u16(&mut table, search_range);
    push_u16(&mut table, entry_selector);
    push_u16(&mut table, range_shift);

    for codepoint in mappings.keys() {
        push_u16(&mut table, *codepoint);
    }
    push_u16(&mut table, 0xFFFF);
    push_u16(&mut table, 0);
    for codepoint in mappings.keys() {
        push_u16(&mut table, *codepoint);
    }
    push_u16(&mut table, 0xFFFF);
    for (codepoint, glyph_id) in mappings {
        push_u16(&mut table, glyph_id.wrapping_sub(*codepoint));
    }
    push_u16(&mut table, 1);
    for _ in 0..seg_count {
        push_u16(&mut table, 0);
    }

    Ok(table)
}

fn cmap_format_12(mappings: &BTreeMap<u32, u16>) -> Result<Vec<u8>> {
    let mut groups: Vec<(u32, u32, u16)> = Vec::new();
    for (codepoint, glyph_id) in mappings {
        if let Some((start, end, start_glyph_id)) = groups.last_mut() {
            let expected_glyph_id = u32::from(*start_glyph_id) + (*codepoint - *start);
            if *codepoint == *end + 1 && u32::from(*glyph_id) == expected_glyph_id {
                *end = *codepoint;
                continue;
            }
        }
        groups.push((*codepoint, *codepoint, *glyph_id));
    }
    let length =
        16usize
            .checked_add(groups.len().checked_mul(12).ok_or_else(|| {
                FontminError::convert_failed("too many groups for cmap format 12")
            })?)
            .ok_or_else(|| FontminError::convert_failed("cmap format 12 is too large"))?;
    let mut table = Vec::with_capacity(length);

    push_u16(&mut table, 12);
    push_u16(&mut table, 0);
    push_u32(
        &mut table,
        u32::try_from(length)
            .map_err(|_| FontminError::convert_failed("cmap format 12 is too large"))?,
    );
    push_u32(&mut table, 0);
    push_u32(
        &mut table,
        u32::try_from(groups.len())
            .map_err(|_| FontminError::convert_failed("too many groups for cmap format 12"))?,
    );
    for (start, end, glyph_id) in groups {
        push_u32(&mut table, start);
        push_u32(&mut table, end);
        push_u32(&mut table, u32::from(glyph_id));
    }

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
