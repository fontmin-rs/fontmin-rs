use std::collections::{HashMap, HashSet};

use fontmin_core::FontMetadata;
use fontmin_diagnostics::{FontminError, Result};

const SFNT_HEADER_SIZE: usize = 12;
const SFNT_TABLE_RECORD_SIZE: usize = 16;
const TRUE_TYPE_FLAVOR: u32 = 0x0001_0000;
const CHECKSUM_ADJUSTMENT_MAGIC: u32 = 0xB1B0_AFBA;

#[derive(Debug, Clone)]
pub struct SfntTableRecord {
    pub tag: String,
    pub checksum: u32,
    pub offset: usize,
    pub length: usize,
}

#[derive(Debug, Clone)]
pub struct TtfFont<'a> {
    pub data: &'a [u8],
    pub tables: Vec<SfntTableRecord>,
}

#[derive(Debug, Clone)]
pub struct OwnedSfntTable {
    pub tag: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct OwnedTtfFont {
    pub tables: Vec<OwnedSfntTable>,
}

impl<'a> TtfFont<'a> {
    #[must_use]
    pub fn table(&self, tag: &str) -> Option<&'a [u8]> {
        let record = self.tables.iter().find(|record| record.tag == tag)?;
        let end = record.offset.checked_add(record.length)?;

        self.data.get(record.offset..end)
    }
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
    let font = read_ttf(input)?;

    inspect_sfnt_tables(font.data, &font.tables)
}

pub fn inspect_sfnt(input: &[u8], flavor: SfntFlavor) -> Result<FontMetadata> {
    if !matches_sfnt_flavor(input, flavor) {
        return Err(FontminError::invalid_font(format!(
            "expected {} sfnt data",
            sfnt_flavor_name(flavor),
        )));
    }

    let tables = read_sfnt_table_directory(input)?;

    inspect_sfnt_tables(input, &tables)
}

fn inspect_sfnt_tables(input: &[u8], tables: &[SfntTableRecord]) -> Result<FontMetadata> {
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

pub fn read_ttf(input: &[u8]) -> Result<TtfFont<'_>> {
    if !matches_sfnt_flavor(input, SfntFlavor::TrueType) {
        return Err(FontminError::invalid_font(format!(
            "expected {} sfnt data",
            sfnt_flavor_name(SfntFlavor::TrueType),
        )));
    }

    Ok(TtfFont {
        data: input,
        tables: read_sfnt_table_directory(input)?,
    })
}

pub fn write_ttf(font: &OwnedTtfFont) -> Result<Vec<u8>> {
    if font.tables.is_empty() {
        return Err(FontminError::invalid_font("sfnt contains no tables"));
    }

    let directory_size = SFNT_HEADER_SIZE
        .checked_add(
            font.tables
                .len()
                .checked_mul(SFNT_TABLE_RECORD_SIZE)
                .ok_or_else(|| FontminError::invalid_font("sfnt table directory is too large"))?,
        )
        .ok_or_else(|| FontminError::invalid_font("sfnt table directory is too large"))?;
    let (search_range, entry_selector, range_shift) = sfnt_search_params(font.tables.len())?;
    let table_count = checked_u16(font.tables.len(), "sfnt table count")?;
    let mut records: Vec<WritableSfntTable> = Vec::with_capacity(font.tables.len());
    let mut offset = directory_size;

    for table in &font.tables {
        let tag = sfnt_tag(&table.tag)?;
        if records.iter().any(|record| record.tag == tag) {
            return Err(FontminError::invalid_font(format!(
                "duplicate sfnt table tag `{}`",
                table.tag
            )));
        }
        let mut data = table.data.clone();
        let checksum = if tag == *b"head" {
            let adjustment = data.get_mut(8..12).ok_or_else(|| {
                FontminError::invalid_font("head table is missing checkSumAdjustment")
            })?;
            adjustment.fill(0);
            calculate_table_checksum(&data)
        } else {
            calculate_table_checksum(&data)
        };
        let padded_length = padded_len(data.len());

        records.push(WritableSfntTable {
            tag,
            checksum,
            offset,
            length: data.len(),
            data,
        });
        offset = offset
            .checked_add(padded_length)
            .ok_or_else(|| FontminError::invalid_font("sfnt table data is too large"))?;
    }

    records.sort_by_key(|record| record.tag);
    offset = directory_size;
    for record in &mut records {
        record.offset = offset;
        offset = offset
            .checked_add(padded_len(record.data.len()))
            .ok_or_else(|| FontminError::invalid_font("sfnt table data is too large"))?;
    }

    let mut output = Vec::with_capacity(offset);

    write_u32(&mut output, TRUE_TYPE_FLAVOR);
    write_u16(&mut output, table_count);
    write_u16(&mut output, search_range);
    write_u16(&mut output, entry_selector);
    write_u16(&mut output, range_shift);

    for record in &records {
        write_bytes(&mut output, &record.tag);
        write_u32(&mut output, record.checksum);
        write_u32(
            &mut output,
            checked_u32(record.offset, "sfnt table offset")?,
        );
        write_u32(
            &mut output,
            checked_u32(record.length, "sfnt table length")?,
        );
    }

    for record in &records {
        write_bytes(&mut output, &record.data);
        while !output.len().is_multiple_of(4) {
            output.push(0);
        }
    }

    apply_checksum_adjustment(&mut output, &records)?;

    Ok(output)
}

struct WritableSfntTable {
    tag: [u8; 4],
    checksum: u32,
    offset: usize,
    length: usize,
    data: Vec<u8>,
}

fn apply_checksum_adjustment(output: &mut [u8], records: &[WritableSfntTable]) -> Result<()> {
    let head = records
        .iter()
        .find(|record| record.tag == *b"head")
        .ok_or_else(|| FontminError::invalid_font("missing required TTF table head"))?;
    let adjustment_offset = head
        .offset
        .checked_add(8)
        .ok_or_else(|| FontminError::invalid_font("head checkSumAdjustment offset overflows"))?;
    let adjustment_end = adjustment_offset
        .checked_add(4)
        .ok_or_else(|| FontminError::invalid_font("head checkSumAdjustment offset overflows"))?;

    if adjustment_end > output.len() {
        return Err(FontminError::invalid_font(
            "head table is missing checkSumAdjustment",
        ));
    }

    let adjustment = CHECKSUM_ADJUSTMENT_MAGIC.wrapping_sub(calculate_table_checksum(output));
    output[adjustment_offset..adjustment_end].copy_from_slice(&adjustment.to_be_bytes());

    Ok(())
}

fn sfnt_search_params(table_count: usize) -> Result<(u16, u16, u16)> {
    if table_count == 0 {
        return Err(FontminError::invalid_font("sfnt contains no tables"));
    }

    let max_power = 1usize << table_count.ilog2();
    let search_range = checked_u16(max_power * 16, "sfnt search range")?;
    let entry_selector = checked_u16(max_power.ilog2() as usize, "sfnt entry selector")?;
    let range_shift = checked_u16(
        table_count
            .checked_mul(16)
            .and_then(|range| range.checked_sub(usize::from(search_range)))
            .ok_or_else(|| FontminError::invalid_font("sfnt range shift overflows"))?,
        "sfnt range shift",
    )?;

    Ok((search_range, entry_selector, range_shift))
}

fn sfnt_tag(tag: &str) -> Result<[u8; 4]> {
    let bytes = tag.as_bytes();

    if bytes.len() != 4 || !bytes.is_ascii() {
        return Err(FontminError::invalid_font(format!(
            "sfnt table tag `{tag}` must be 4 ASCII bytes",
        )));
    }

    Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
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

pub fn read_sfnt_table_directory(input: &[u8]) -> Result<Vec<SfntTableRecord>> {
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
    let mut seen_tags = HashSet::with_capacity(table_count);

    for index in 0..table_count {
        let offset = 12 + index * 16;
        let tag = std::str::from_utf8(read_exact(input, offset, 4)?)
            .map_err(|_| FontminError::invalid_font("TTF table tag is not ASCII"))?
            .to_string();

        if !seen_tags.insert(tag.clone()) {
            return Err(FontminError::invalid_font(format!(
                "duplicate sfnt table tag `{tag}`",
            )));
        }

        let checksum = read_u32(input, offset + 4)?;
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

        tables.push(SfntTableRecord {
            tag,
            checksum,
            offset: table_offset,
            length: table_length,
        });
    }

    Ok(tables)
}

#[must_use]
pub fn calculate_table_checksum(input: &[u8]) -> u32 {
    let mut checksum = 0u32;
    let chunks = input.chunks_exact(4);
    let remainder = chunks.remainder();

    for chunk in chunks {
        checksum =
            checksum.wrapping_add(u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }

    if !remainder.is_empty() {
        let mut padded = [0u8; 4];
        padded[..remainder.len()].copy_from_slice(remainder);
        checksum = checksum.wrapping_add(u32::from_be_bytes(padded));
    }

    checksum
}

fn required_table<'a>(
    tables: &'a HashMap<&str, &'a SfntTableRecord>,
    tag: &str,
) -> Result<&'a SfntTableRecord> {
    tables
        .get(tag)
        .copied()
        .ok_or_else(|| FontminError::invalid_font(format!("missing required TTF table {tag}")))
}

fn read_names(input: &[u8], table: &SfntTableRecord) -> Result<Vec<NameRecord>> {
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

fn write_bytes(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(bytes);
}

fn write_u16(output: &mut Vec<u8>, value: u16) {
    write_bytes(output, &value.to_be_bytes());
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
    write_bytes(output, &value.to_be_bytes());
}

fn padded_len(length: usize) -> usize {
    (length + 3) & !3
}

fn checked_u16(value: usize, label: &str) -> Result<u16> {
    u16::try_from(value).map_err(|_| FontminError::invalid_font(format!("{label} exceeds u16")))
}

fn checked_u32(value: usize, label: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| FontminError::invalid_font(format!("{label} exceeds u32")))
}

fn read_exact(input: &[u8], offset: usize, length: usize) -> Result<&[u8]> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| FontminError::invalid_font("font read offset overflows"))?;

    input
        .get(offset..end)
        .ok_or_else(|| FontminError::invalid_font("font data is truncated"))
}

#[cfg(test)]
mod tests {
    use fontmin_testing::ROBOTO;

    use super::{
        OwnedSfntTable, OwnedTtfFont, SfntFlavor, calculate_table_checksum, inspect_sfnt,
        read_sfnt_table_directory, read_ttf, write_ttf,
    };

    #[test]
    fn calculates_padded_table_checksums() {
        assert_eq!(calculate_table_checksum(b"\x00\x00\x00\x01"), 1);
        assert_eq!(calculate_table_checksum(b"\x00\x00\x00\x01\x00"), 1);
        assert_eq!(
            calculate_table_checksum(b"\xff\xff\xff\xff\x00\x00\x00\x02"),
            1
        );
        assert_eq!(calculate_table_checksum(b"abc"), 0x6162_6300);
    }

    #[test]
    fn reads_ttf_font_with_table_lookup() {
        let font = read_ttf(ROBOTO).unwrap();
        let head = font.table("head").unwrap();
        let name = font.table("name").unwrap();

        assert_eq!(font.data, ROBOTO);
        assert_eq!(
            font.tables.len(),
            read_sfnt_table_directory(ROBOTO).unwrap().len()
        );
        assert_eq!(u16::from_be_bytes([head[18], head[19]]), 2048);
        assert!(name.len() > 6);
        assert!(font.table("nope").is_none());
    }

    #[test]
    fn writes_owned_ttf_font_roundtrip() {
        let font = read_ttf(ROBOTO).unwrap();
        let owned = OwnedTtfFont {
            tables: font
                .tables
                .iter()
                .map(|record| OwnedSfntTable {
                    tag: record.tag.clone(),
                    data: font.table(&record.tag).unwrap().to_vec(),
                })
                .collect(),
        };

        let output = write_ttf(&owned).unwrap();
        let original_info = super::inspect_ttf(ROBOTO).unwrap();
        let output_info = super::inspect_ttf(&output).unwrap();
        let output_font = read_ttf(&output).unwrap();

        assert_eq!(output_info, original_info);
        assert!(output.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        for record in &output_font.tables {
            let table = output_font.table(&record.tag).unwrap();

            assert_eq!(
                record.checksum,
                if record.tag == "head" {
                    let mut head = table.to_vec();
                    head[8..12].fill(0);
                    calculate_table_checksum(&head)
                } else {
                    calculate_table_checksum(table)
                }
            );
        }
    }

    #[test]
    fn writes_ttf_with_checksum_adjustment() {
        let font = read_ttf(ROBOTO).unwrap();
        let owned = OwnedTtfFont {
            tables: font
                .tables
                .iter()
                .map(|record| OwnedSfntTable {
                    tag: record.tag.clone(),
                    data: font.table(&record.tag).unwrap().to_vec(),
                })
                .collect(),
        };

        let output = write_ttf(&owned).unwrap();
        let output_font = read_ttf(&output).unwrap();
        let head = output_font.table("head").unwrap();

        assert_eq!(calculate_table_checksum(&output), 0xB1B0_AFBA);
        assert_ne!(&head[8..12], &[0, 0, 0, 0]);
    }

    #[test]
    fn writes_ttf_table_directory_sorted_by_tag() {
        let font = read_ttf(ROBOTO).unwrap();
        let owned = OwnedTtfFont {
            tables: font
                .tables
                .iter()
                .rev()
                .map(|record| OwnedSfntTable {
                    tag: record.tag.clone(),
                    data: font.table(&record.tag).unwrap().to_vec(),
                })
                .collect(),
        };

        let output = write_ttf(&owned).unwrap();
        let output_font = read_ttf(&output).unwrap();
        let tags = output_font
            .tables
            .iter()
            .map(|record| record.tag.as_str())
            .collect::<Vec<_>>();
        let mut sorted_tags = tags.clone();

        sorted_tags.sort_unstable();

        assert_eq!(tags, sorted_tags);
        assert_eq!(
            super::inspect_ttf(&output).unwrap().family_name.as_deref(),
            Some("Roboto")
        );
    }

    #[test]
    fn rejects_owned_tables_with_invalid_tags() {
        let error = write_ttf(&OwnedTtfFont {
            tables: vec![OwnedSfntTable {
                tag: "abc".into(),
                data: Vec::new(),
            }],
        })
        .unwrap_err();

        assert!(error.to_string().contains("sfnt table tag"));
    }

    #[test]
    fn rejects_owned_tables_with_duplicate_tags() {
        let font = read_ttf(ROBOTO).unwrap();
        let head = font.table("head").unwrap().to_vec();
        let error = write_ttf(&OwnedTtfFont {
            tables: vec![
                OwnedSfntTable {
                    tag: "head".into(),
                    data: head.clone(),
                },
                OwnedSfntTable {
                    tag: "head".into(),
                    data: head,
                },
            ],
        })
        .unwrap_err();

        assert!(error.to_string().contains("duplicate sfnt table tag"));
    }

    #[test]
    fn rejects_duplicate_sfnt_table_records() {
        let mut font = ROBOTO.to_vec();
        let first_tag = font[12..16].to_vec();

        font[28..32].copy_from_slice(&first_tag);

        let error = read_ttf(&font).unwrap_err();

        assert!(error.to_string().contains("duplicate sfnt table tag"));
    }

    #[test]
    fn rejects_non_ttf_sfnt_flavor() {
        let mut otf = ROBOTO.to_vec();
        otf[0..4].copy_from_slice(b"OTTO");

        let error = read_ttf(&otf).unwrap_err();

        assert!(error.to_string().contains("expected TrueType sfnt data"));
        assert!(inspect_sfnt(&otf, SfntFlavor::OpenTypeCff).is_ok());
    }

    #[test]
    fn reads_sfnt_table_directory_records() {
        let tables = read_sfnt_table_directory(ROBOTO).unwrap();

        assert!(tables.iter().any(|record| record.tag == "head"));
        assert!(tables.iter().any(|record| record.tag == "name"));
        assert!(tables.iter().any(|record| record.tag == "glyf"));
        assert!(tables.iter().all(|record| record.checksum != 0));
        assert!(tables.iter().all(|record| record.offset < ROBOTO.len()));
        assert!(
            tables
                .iter()
                .all(|record| record.offset + record.length <= ROBOTO.len())
        );
    }

    #[test]
    fn rejects_truncated_sfnt_table_directory() {
        let error = read_sfnt_table_directory(&ROBOTO[..20]).unwrap_err();

        assert!(error.to_string().contains("table directory is truncated"));
    }

    #[test]
    fn rejects_sfnt_tables_outside_file() {
        let mut font = ROBOTO.to_vec();
        let font_len = u32::try_from(font.len()).unwrap();

        font[24..28].copy_from_slice(&font_len.to_be_bytes());
        font[28..32].copy_from_slice(&1u32.to_be_bytes());

        let error = read_sfnt_table_directory(&font).unwrap_err();

        assert!(error.to_string().contains("points outside the file"));
    }
}
