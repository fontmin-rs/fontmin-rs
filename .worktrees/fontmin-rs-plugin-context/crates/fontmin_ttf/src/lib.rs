use std::collections::HashMap;

use fontmin_core::FontMetadata;
use fontmin_diagnostics::{FontminError, Result};

#[derive(Debug, Clone)]
struct TableRecord {
    tag: String,
    offset: usize,
    length: usize,
}

#[derive(Debug, Clone)]
struct NameRecord {
    platform_id: u16,
    language_id: u16,
    name_id: u16,
    value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SfntFlavor {
    TrueType,
    OpenTypeCff,
}

pub fn inspect_ttf(input: &[u8]) -> Result<FontMetadata> {
    inspect_sfnt(input, SfntFlavor::TrueType)
}

pub fn inspect_sfnt(input: &[u8], flavor: SfntFlavor) -> Result<FontMetadata> {
    if !matches_sfnt_flavor(input, flavor) {
        return Err(FontminError::invalid_font(format!(
            "expected {} sfnt data",
            sfnt_flavor_name(flavor),
        )));
    }

    let tables = read_table_directory(input)?;
    let table_map = tables
        .iter()
        .map(|record| (record.tag.as_str(), record))
        .collect::<HashMap<_, _>>();

    let head = required_table(&table_map, "head")?;
    let hhea = required_table(&table_map, "hhea")?;
    let maxp = required_table(&table_map, "maxp")?;
    let name = required_table(&table_map, "name")?;

    let names = read_names(input, name)?;
    let mut table_tags = tables
        .iter()
        .map(|record| record.tag.clone())
        .collect::<Vec<_>>();
    table_tags.sort_unstable();

    Ok(FontMetadata {
        family_name: pick_name(&names, 1),
        subfamily_name: pick_name(&names, 2),
        full_name: pick_name(&names, 4),
        post_script_name: pick_name(&names, 6),
        glyph_count: read_u16(input, maxp.offset + 4)?,
        units_per_em: read_u16(input, head.offset + 18)?,
        ascender: read_i16(input, hhea.offset + 4)?,
        descender: read_i16(input, hhea.offset + 6)?,
        tables: table_tags,
    })
}

fn matches_sfnt_flavor(input: &[u8], flavor: SfntFlavor) -> bool {
    match flavor {
        SfntFlavor::TrueType => {
            input.starts_with(&[0x00, 0x01, 0x00, 0x00]) || input.starts_with(b"true")
        }
        SfntFlavor::OpenTypeCff => input.starts_with(b"OTTO"),
    }
}

fn sfnt_flavor_name(flavor: SfntFlavor) -> &'static str {
    match flavor {
        SfntFlavor::TrueType => "TrueType",
        SfntFlavor::OpenTypeCff => "OpenType/CFF",
    }
}

fn read_table_directory(input: &[u8]) -> Result<Vec<TableRecord>> {
    if input.len() < 12 {
        return Err(FontminError::invalid_font("TTF header is truncated"));
    }

    let table_count = usize::from(read_u16(input, 4)?);
    let record_end = 12usize
        .checked_add(
            table_count
                .checked_mul(16)
                .ok_or_else(|| FontminError::invalid_font("TTF table directory is too large"))?,
        )
        .ok_or_else(|| FontminError::invalid_font("TTF table directory is too large"))?;

    if record_end > input.len() {
        return Err(FontminError::invalid_font(
            "TTF table directory is truncated",
        ));
    }

    let mut tables = Vec::with_capacity(table_count);

    for index in 0..table_count {
        let offset = 12 + index * 16;
        let tag = std::str::from_utf8(read_exact(input, offset, 4)?)
            .map_err(|_| FontminError::invalid_font("TTF table tag is not ASCII"))?
            .to_string();
        let table_offset = read_u32(input, offset + 8)? as usize;
        let table_length = read_u32(input, offset + 12)? as usize;

        let table_end = table_offset
            .checked_add(table_length)
            .ok_or_else(|| FontminError::invalid_font("TTF table range overflows"))?;
        if table_end > input.len() {
            return Err(FontminError::invalid_font(format!(
                "TTF table {tag} points outside the file",
            )));
        }

        tables.push(TableRecord {
            tag,
            offset: table_offset,
            length: table_length,
        });
    }

    Ok(tables)
}

fn required_table<'a>(
    tables: &'a HashMap<&str, &'a TableRecord>,
    tag: &str,
) -> Result<&'a TableRecord> {
    tables
        .get(tag)
        .copied()
        .ok_or_else(|| FontminError::invalid_font(format!("missing required TTF table {tag}")))
}

fn read_names(input: &[u8], table: &TableRecord) -> Result<Vec<NameRecord>> {
    if table.length < 6 {
        return Err(FontminError::invalid_font("name table is truncated"));
    }

    let count = usize::from(read_u16(input, table.offset + 2)?);
    let storage_offset = usize::from(read_u16(input, table.offset + 4)?);
    let record_end =
        6usize
            .checked_add(count.checked_mul(12).ok_or_else(|| {
                FontminError::invalid_font("name table record count is too large")
            })?)
            .ok_or_else(|| FontminError::invalid_font("name table record count is too large"))?;

    if record_end > table.length || storage_offset > table.length {
        return Err(FontminError::invalid_font(
            "name table records are truncated",
        ));
    }

    let storage_base = table
        .offset
        .checked_add(storage_offset)
        .ok_or_else(|| FontminError::invalid_font("name table storage overflows"))?;
    let table_end = table
        .offset
        .checked_add(table.length)
        .ok_or_else(|| FontminError::invalid_font("name table overflows"))?;
    let mut names = Vec::new();

    for index in 0..count {
        let record_offset = table.offset + 6 + index * 12;
        let platform_id = read_u16(input, record_offset)?;
        let language_id = read_u16(input, record_offset + 4)?;
        let name_id = read_u16(input, record_offset + 6)?;
        let value_length = usize::from(read_u16(input, record_offset + 8)?);
        let value_offset = usize::from(read_u16(input, record_offset + 10)?);
        let value_start = storage_base
            .checked_add(value_offset)
            .ok_or_else(|| FontminError::invalid_font("name record offset overflows"))?;
        let value_end = value_start
            .checked_add(value_length)
            .ok_or_else(|| FontminError::invalid_font("name record length overflows"))?;

        if value_end > table_end {
            return Err(FontminError::invalid_font(
                "name record points outside the name table",
            ));
        }

        let value = decode_name(platform_id, read_exact(input, value_start, value_length)?);
        if !value.is_empty() {
            names.push(NameRecord {
                platform_id,
                language_id,
                name_id,
                value,
            });
        }
    }

    Ok(names)
}

fn pick_name(names: &[NameRecord], name_id: u16) -> Option<String> {
    names
        .iter()
        .filter(|name| name.name_id == name_id)
        .min_by_key(|name| name_priority(name))
        .map(|name| name.value.clone())
}

fn name_priority(name: &NameRecord) -> u8 {
    match (name.platform_id, name.language_id) {
        (3, 0x0409) => 0,
        (0, _) => 1,
        (3, _) => 2,
        _ => 3,
    }
}

fn decode_name(platform_id: u16, bytes: &[u8]) -> String {
    if platform_id == 0 || platform_id == 3 {
        return decode_utf16be(bytes);
    }

    String::from_utf8_lossy(bytes).to_string()
}

fn decode_utf16be(bytes: &[u8]) -> String {
    let code_units = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]));

    char::decode_utf16(code_units)
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

fn read_u16(input: &[u8], offset: usize) -> Result<u16> {
    let bytes = read_exact(input, offset, 2)?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_i16(input: &[u8], offset: usize) -> Result<i16> {
    let bytes = read_exact(input, offset, 2)?;
    Ok(i16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u32(input: &[u8], offset: usize) -> Result<u32> {
    let bytes = read_exact(input, offset, 4)?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_exact(input: &[u8], offset: usize, length: usize) -> Result<&[u8]> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| FontminError::invalid_font("font read offset overflows"))?;

    input
        .get(offset..end)
        .ok_or_else(|| FontminError::invalid_font("font data is truncated"))
}
