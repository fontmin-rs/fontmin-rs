//! Canonical reader and writer for sfnt table directories.
//!
//! Format crates provide table payloads; this module owns tag validation,
//! alignment, directory search parameters, checksums, and the `head`
//! `checkSumAdjustment`.

use std::collections::HashSet;

use fontmin_diagnostics::{FontminError, Result};

const SFNT_HEADER_SIZE: usize = 12;
const SFNT_TABLE_RECORD_SIZE: usize = 16;
const TRUE_TYPE_SIGNATURE: [u8; 4] = [0x00, 0x01, 0x00, 0x00];
const CHECKSUM_ADJUSTMENT_MAGIC: u32 = 0xB1B0_AFBA;

/// A validated table entry from an sfnt directory.
#[derive(Debug, Clone)]
pub struct SfntTableRecord {
    pub tag: String,
    pub checksum: u32,
    pub offset: usize,
    pub length: usize,
}

/// A borrowed sfnt font and its validated table directory.
#[derive(Debug, Clone)]
pub struct SfntFont<'a> {
    pub data: &'a [u8],
    pub flavor: SfntFlavor,
    pub tables: Vec<SfntTableRecord>,
}

/// A borrowed TrueType font.
pub type TtfFont<'a> = SfntFont<'a>;

/// An owned sfnt table payload ready for canonical serialization.
#[derive(Debug, Clone)]
pub struct OwnedSfntTable {
    pub tag: String,
    pub data: Vec<u8>,
}

/// An owned sfnt font with an explicit outline flavor.
#[derive(Debug, Clone)]
pub struct OwnedSfntFont {
    pub flavor: SfntFlavor,
    pub tables: Vec<OwnedSfntTable>,
}

/// An owned TrueType font.
#[derive(Debug, Clone)]
pub struct OwnedTtfFont {
    pub tables: Vec<OwnedSfntTable>,
}

/// The supported sfnt header signatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SfntFlavor {
    TrueType,
    OpenTypeCff,
}

impl SfntFlavor {
    /// Parses a supported sfnt signature.
    pub fn from_signature(signature: [u8; 4]) -> Result<Self> {
        match &signature {
            [0x00, 0x01, 0x00, 0x00] | b"true" => Ok(Self::TrueType),
            b"OTTO" => Ok(Self::OpenTypeCff),
            _ => Err(FontminError::invalid_font("unsupported sfnt flavor")),
        }
    }

    /// Returns the canonical four-byte signature for this flavor.
    #[must_use]
    pub const fn signature(self) -> [u8; 4] {
        match self {
            Self::TrueType => TRUE_TYPE_SIGNATURE,
            Self::OpenTypeCff => *b"OTTO",
        }
    }

    pub(crate) fn matches(self, input: &[u8]) -> bool {
        match self {
            Self::TrueType => input.starts_with(&TRUE_TYPE_SIGNATURE) || input.starts_with(b"true"),
            Self::OpenTypeCff => input.starts_with(b"OTTO"),
        }
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::TrueType => "TrueType",
            Self::OpenTypeCff => "OpenType/CFF",
        }
    }
}

impl<'a> SfntFont<'a> {
    /// Returns a table payload by its four-character tag.
    #[must_use]
    pub fn table(&self, tag: &str) -> Option<&'a [u8]> {
        let record = self.tables.iter().find(|record| record.tag == tag)?;
        let end = record.offset.checked_add(record.length)?;

        self.data.get(record.offset..end)
    }
}

/// Reads and validates an sfnt font with the expected outline flavor.
pub fn read_sfnt(input: &[u8], flavor: SfntFlavor) -> Result<SfntFont<'_>> {
    if !flavor.matches(input) {
        return Err(FontminError::invalid_font(format!(
            "expected {} sfnt data",
            flavor.name(),
        )));
    }
    if input.len() < SFNT_HEADER_SIZE {
        return Err(FontminError::invalid_font("TTF header is truncated"));
    }

    let table_count = usize::from(read_u16(input, 4)?);
    if flavor == SfntFlavor::TrueType {
        validate_sfnt_search_params(input, table_count)?;
    } else {
        sfnt_search_params(table_count)?;
    }
    let tables = parse_sfnt_table_directory(input, table_count)?;

    if flavor == SfntFlavor::TrueType
        && tables
            .iter()
            .any(|record| record.tag == "head" && record.length < 12)
    {
        return Err(FontminError::invalid_font(
            "head table is missing checkSumAdjustment",
        ));
    }

    Ok(SfntFont {
        data: input,
        flavor,
        tables,
    })
}

/// Reads and validates a TrueType font.
pub fn read_ttf(input: &[u8]) -> Result<TtfFont<'_>> {
    read_sfnt(input, SfntFlavor::TrueType)
}

/// Serializes a TrueType font through the canonical sfnt writer.
pub fn write_ttf(font: &OwnedTtfFont) -> Result<Vec<u8>> {
    write_sfnt_tables(SfntFlavor::TrueType, &font.tables)
}

/// Serializes an sfnt font with canonical ordering, checksums, and alignment.
pub fn write_sfnt(font: &OwnedSfntFont) -> Result<Vec<u8>> {
    write_sfnt_tables(font.flavor, &font.tables)
}

fn write_sfnt_tables(flavor: SfntFlavor, tables: &[OwnedSfntTable]) -> Result<Vec<u8>> {
    if tables.is_empty() {
        return Err(FontminError::invalid_font("sfnt contains no tables"));
    }

    let directory_size = SFNT_HEADER_SIZE
        .checked_add(
            tables
                .len()
                .checked_mul(SFNT_TABLE_RECORD_SIZE)
                .ok_or_else(|| FontminError::invalid_font("sfnt table directory is too large"))?,
        )
        .ok_or_else(|| FontminError::invalid_font("sfnt table directory is too large"))?;
    let (search_range, entry_selector, range_shift) = sfnt_search_params(tables.len())?;
    let table_count = checked_u16(tables.len(), "sfnt table count")?;
    let mut records: Vec<WritableSfntTable> = Vec::with_capacity(tables.len());
    let mut offset = directory_size;

    for table in tables {
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

    write_bytes(&mut output, &flavor.signature());
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
        .ok_or_else(|| FontminError::invalid_font("missing required sfnt table head"))?;
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

/// Reads and validates an sfnt table directory.
pub fn read_sfnt_table_directory(input: &[u8]) -> Result<Vec<SfntTableRecord>> {
    if input.len() < SFNT_HEADER_SIZE {
        return Err(FontminError::invalid_font("TTF header is truncated"));
    }

    let signature: [u8; 4] = input[0..4]
        .try_into()
        .map_err(|_| FontminError::invalid_font("TTF header is truncated"))?;
    let flavor = SfntFlavor::from_signature(signature)?;

    read_sfnt(input, flavor).map(|font| font.tables)
}

fn parse_sfnt_table_directory(input: &[u8], table_count: usize) -> Result<Vec<SfntTableRecord>> {
    let record_end = SFNT_HEADER_SIZE
        .checked_add(
            table_count
                .checked_mul(SFNT_TABLE_RECORD_SIZE)
                .ok_or_else(|| FontminError::invalid_font("TTF table directory is too large"))?,
        )
        .ok_or_else(|| FontminError::invalid_font("TTF table directory is too large"))?;

    if record_end > input.len() {
        return Err(FontminError::invalid_font(
            "TTF table directory is truncated",
        ));
    }

    let mut tables = Vec::with_capacity(table_count);
    let mut ranges = Vec::with_capacity(table_count);
    let mut seen_tags = HashSet::with_capacity(table_count);

    for index in 0..table_count {
        let offset = SFNT_HEADER_SIZE + index * SFNT_TABLE_RECORD_SIZE;
        let tag_bytes = read_exact(input, offset, 4)?;
        if !tag_bytes.is_ascii() {
            return Err(FontminError::invalid_font("sfnt table tag is not ASCII"));
        }
        let tag = std::str::from_utf8(tag_bytes)
            .map_err(|_| FontminError::invalid_font("sfnt table tag is not ASCII"))?
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
        if table_length > 0 {
            if table_offset < record_end {
                return Err(FontminError::invalid_font(format!(
                    "sfnt table {tag} starts inside the table directory",
                )));
            }
            if !table_offset.is_multiple_of(4) {
                return Err(FontminError::invalid_font(format!(
                    "sfnt table {tag} is not four-byte aligned",
                )));
            }

            ranges.push((table_offset, table_end, tag.clone()));
        }

        tables.push(SfntTableRecord {
            tag,
            checksum,
            offset: table_offset,
            length: table_length,
        });
    }

    ranges.sort_unstable_by_key(|(start, _, _)| *start);
    for pair in ranges.windows(2) {
        let (_, previous_end, previous_tag) = &pair[0];
        let (next_start, _, next_tag) = &pair[1];

        if previous_end > next_start {
            return Err(FontminError::invalid_font(format!(
                "sfnt tables {previous_tag} and {next_tag} overlap",
            )));
        }
    }

    Ok(tables)
}

fn validate_sfnt_search_params(input: &[u8], table_count: usize) -> Result<()> {
    let (expected_search_range, expected_entry_selector, expected_range_shift) =
        sfnt_search_params(table_count)?;
    let search_range = read_u16(input, 6)?;
    let entry_selector = read_u16(input, 8)?;
    let range_shift = read_u16(input, 10)?;

    if search_range != expected_search_range {
        return Err(FontminError::invalid_font("sfnt searchRange is invalid"));
    }
    if entry_selector != expected_entry_selector {
        return Err(FontminError::invalid_font("sfnt entrySelector is invalid"));
    }
    if range_shift != expected_range_shift {
        return Err(FontminError::invalid_font("sfnt rangeShift is invalid"));
    }

    Ok(())
}

fn sfnt_search_params(table_count: usize) -> Result<(u16, u16, u16)> {
    if table_count == 0 {
        return Err(FontminError::invalid_font("sfnt contains no tables"));
    }

    let max_power = 1usize << table_count.ilog2();
    let search_range = checked_u16(max_power * SFNT_TABLE_RECORD_SIZE, "sfnt search range")?;
    let entry_selector = checked_u16(max_power.ilog2() as usize, "sfnt entry selector")?;
    let range_shift = checked_u16(
        table_count
            .checked_mul(SFNT_TABLE_RECORD_SIZE)
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

pub(crate) fn read_u16(input: &[u8], offset: usize) -> Result<u16> {
    let bytes = read_exact(input, offset, 2)?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

pub(crate) fn read_i16(input: &[u8], offset: usize) -> Result<i16> {
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

pub(crate) fn read_exact(input: &[u8], offset: usize, length: usize) -> Result<&[u8]> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| FontminError::invalid_font("font read offset overflows"))?;

    input
        .get(offset..end)
        .ok_or_else(|| FontminError::invalid_font("font data is truncated"))
}

#[cfg(test)]
mod tests {
    use fontmin_testing::{ROBOTO, SOURCE_SANS_3_REGULAR_CFF};

    use super::{
        CHECKSUM_ADJUSTMENT_MAGIC, OwnedSfntFont, OwnedSfntTable, OwnedTtfFont, SfntFlavor,
        calculate_table_checksum, read_sfnt, read_sfnt_table_directory, read_ttf, write_sfnt,
        write_ttf,
    };

    fn owned_tables(input: &[u8]) -> Vec<OwnedSfntTable> {
        read_sfnt_table_directory(input)
            .unwrap()
            .into_iter()
            .map(|record| OwnedSfntTable {
                tag: record.tag,
                data: input[record.offset..record.offset + record.length].to_vec(),
            })
            .collect()
    }

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
    fn reads_opentype_cff_with_the_canonical_sfnt_reader() {
        let font = read_sfnt(SOURCE_SANS_3_REGULAR_CFF, SfntFlavor::OpenTypeCff).unwrap();

        assert_eq!(font.flavor, SfntFlavor::OpenTypeCff);
        assert!(font.table("CFF ").is_some());
        assert!(font.table("head").is_some());
        assert!(font.table("glyf").is_none());
    }

    #[test]
    fn writes_owned_ttf_font_roundtrip() {
        let output = write_ttf(&OwnedTtfFont {
            tables: owned_tables(ROBOTO),
        })
        .unwrap();
        let original_info = crate::inspect_ttf(ROBOTO).unwrap();
        let output_info = crate::inspect_ttf(&output).unwrap();
        let output_font = read_ttf(&output).unwrap();

        assert_eq!(output_info, original_info);
        assert!(output.starts_with(&SfntFlavor::TrueType.signature()));
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
    fn writes_canonical_opentype_cff_sfnt() {
        let output = write_sfnt(&OwnedSfntFont {
            flavor: SfntFlavor::OpenTypeCff,
            tables: owned_tables(SOURCE_SANS_3_REGULAR_CFF),
        })
        .unwrap();

        assert!(output.starts_with(b"OTTO"));
        assert_eq!(calculate_table_checksum(&output), CHECKSUM_ADJUSTMENT_MAGIC);
        assert_eq!(
            crate::inspect_sfnt(&output, SfntFlavor::OpenTypeCff).unwrap(),
            crate::inspect_sfnt(SOURCE_SANS_3_REGULAR_CFF, SfntFlavor::OpenTypeCff).unwrap(),
        );
    }

    #[test]
    fn writes_ttf_with_checksum_adjustment() {
        let output = write_ttf(&OwnedTtfFont {
            tables: owned_tables(ROBOTO),
        })
        .unwrap();
        let output_font = read_ttf(&output).unwrap();
        let head = output_font.table("head").unwrap();

        assert_eq!(calculate_table_checksum(&output), CHECKSUM_ADJUSTMENT_MAGIC);
        assert_ne!(&head[8..12], &[0, 0, 0, 0]);
    }

    #[test]
    fn writes_ttf_table_directory_sorted_by_tag() {
        let mut tables = owned_tables(ROBOTO);
        tables.reverse();
        let output = write_ttf(&OwnedTtfFont { tables }).unwrap();
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
            crate::inspect_ttf(&output).unwrap().family_name.as_deref(),
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
        assert!(crate::inspect_sfnt(&otf, SfntFlavor::OpenTypeCff).is_ok());
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
    fn rejects_zero_sfnt_table_count() {
        let error = read_ttf(&[0x00, 0x01, 0x00, 0x00, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap_err();

        assert!(error.to_string().contains("sfnt contains no tables"));
    }

    #[test]
    fn rejects_unencodable_sfnt_search_range() {
        let error = read_ttf(&[0x00, 0x01, 0x00, 0x00, 0x10, 0, 0, 0, 0, 0, 0, 0]).unwrap_err();

        assert!(error.to_string().contains("sfnt search range exceeds u16"));
    }

    #[test]
    fn rejects_noncanonical_truetype_search_parameters() {
        let mut font = ROBOTO.to_vec();

        font[6..12].fill(0);

        let error = read_ttf(&font).unwrap_err();

        assert!(error.to_string().contains("sfnt searchRange is invalid"));
    }

    #[test]
    fn accepts_noncanonical_opentype_search_parameters() {
        let mut font = SOURCE_SANS_3_REGULAR_CFF.to_vec();

        font[6..12].fill(0);

        assert!(read_sfnt(&font, SfntFlavor::OpenTypeCff).is_ok());
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

    #[test]
    fn rejects_misaligned_sfnt_table_data() {
        let mut font = ROBOTO.to_vec();
        let first_table_offset = u32::from_be_bytes(font[20..24].try_into().unwrap());

        font[20..24].copy_from_slice(&(first_table_offset + 1).to_be_bytes());

        let error = read_ttf(&font).unwrap_err();

        assert!(error.to_string().contains("not four-byte aligned"));
    }

    #[test]
    fn rejects_sfnt_table_data_inside_directory() {
        let mut font = ROBOTO.to_vec();
        let table_count = usize::from(u16::from_be_bytes(font[4..6].try_into().unwrap()));
        let directory_end = u32::try_from(12 + table_count * 16).unwrap();

        font[20..24].copy_from_slice(&(directory_end - 4).to_be_bytes());

        let error = read_ttf(&font).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("starts inside the table directory")
        );
    }

    #[test]
    fn rejects_overlapping_sfnt_tables() {
        let mut font = ROBOTO.to_vec();
        let first_table_offset = font[20..24].to_vec();

        font[36..40].copy_from_slice(&first_table_offset);

        let error = read_ttf(&font).unwrap_err();

        assert!(error.to_string().contains("overlap"));
    }

    #[test]
    fn rejects_short_head_table_before_subset_reader() {
        let font = [
            0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x20, 0x00, 0x01, 0x00, 0x00, b'h', b'e',
            b'a', b'd', 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x00,
            b'h', b'e', 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00,
            0x00, 0x00,
        ];

        let error = read_ttf(&font).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("head table is missing checkSumAdjustment")
        );
    }
}
