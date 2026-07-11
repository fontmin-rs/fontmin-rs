use std::io::{Read, Write};

use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};

const WOFF_HEADER_SIZE: usize = 44;
const WOFF_TABLE_RECORD_SIZE: usize = 20;
const SFNT_HEADER_SIZE: usize = 12;
const SFNT_TABLE_RECORD_SIZE: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WoffOptions {
    pub deflate: bool,
    pub compression_level: Option<u32>,
    pub metadata: Option<String>,
    pub private_data: Option<Vec<u8>>,
}

impl Default for WoffOptions {
    fn default() -> Self {
        Self {
            deflate: true,
            compression_level: None,
            metadata: None,
            private_data: None,
        }
    }
}

#[derive(Debug, Clone)]
struct SfntTable {
    tag: [u8; 4],
    checksum: u32,
    offset: usize,
    length: usize,
}

#[derive(Debug, Clone)]
struct WoffTable {
    tag: [u8; 4],
    checksum: u32,
    data: Vec<u8>,
    original_length: usize,
    offset: usize,
}

pub fn encode_ttf_to_woff(input: &[u8], options: &WoffOptions) -> Result<Vec<u8>> {
    if !is_ttf(input) {
        return Err(FontminError::invalid_font(
            "expected TrueType sfnt data for WOFF encoding",
        ));
    }

    if options.metadata.is_some() || options.private_data.is_some() {
        return Err(FontminError::config(
            "WOFF metadata and private data are not supported yet",
        ));
    }

    let flavor = read_u32(input, 0)?;
    let tables = read_sfnt_tables(input)?;
    let mut woff_tables = encode_tables(input, tables, options)?;
    let total_sfnt_size = sfnt_size(&woff_tables)?;
    let directory_size = WOFF_HEADER_SIZE
        .checked_add(
            woff_tables
                .len()
                .checked_mul(WOFF_TABLE_RECORD_SIZE)
                .ok_or_else(|| FontminError::invalid_font("WOFF table directory is too large"))?,
        )
        .ok_or_else(|| FontminError::invalid_font("WOFF table directory is too large"))?;
    let total_length = assign_offsets(directory_size, &mut woff_tables)?;

    let mut output = Vec::with_capacity(total_length);
    write_header(
        &mut output,
        flavor,
        total_length,
        checked_u16(woff_tables.len(), "WOFF table count")?,
        total_sfnt_size,
    )?;
    write_table_directory(&mut output, &woff_tables)?;
    write_table_data(&mut output, &woff_tables);

    Ok(output)
}

pub fn decode_woff_to_ttf(input: &[u8]) -> Result<Vec<u8>> {
    if !input.starts_with(b"wOFF") {
        return Err(FontminError::invalid_font(
            "expected WOFF data for TTF decoding",
        ));
    }
    if input.len() < WOFF_HEADER_SIZE {
        return Err(FontminError::invalid_font("WOFF header is truncated"));
    }

    let flavor = read_u32(input, 4)?;
    let declared_length = read_u32(input, 8)? as usize;
    let table_count = usize::from(read_u16(input, 12)?);
    let total_sfnt_size = read_u32(input, 16)?;

    if declared_length != input.len() {
        return Err(FontminError::invalid_font(
            "WOFF declared length does not match file length",
        ));
    }
    if table_count == 0 {
        return Err(FontminError::invalid_font("WOFF contains no tables"));
    }

    let mut tables = read_woff_tables(input, table_count)?;
    let expected_sfnt_size = sfnt_size(&tables)?;

    if expected_sfnt_size != total_sfnt_size {
        return Err(FontminError::invalid_font(
            "WOFF total sfnt size does not match table data",
        ));
    }

    write_sfnt(flavor, &mut tables)
}

fn is_ttf(input: &[u8]) -> bool {
    input.starts_with(&[0x00, 0x01, 0x00, 0x00]) || input.starts_with(b"true")
}

fn read_sfnt_tables(input: &[u8]) -> Result<Vec<SfntTable>> {
    if input.len() < SFNT_HEADER_SIZE {
        return Err(FontminError::invalid_font("sfnt header is truncated"));
    }

    let table_count = usize::from(read_u16(input, 4)?);
    let directory_end = SFNT_HEADER_SIZE
        .checked_add(
            table_count
                .checked_mul(SFNT_TABLE_RECORD_SIZE)
                .ok_or_else(|| FontminError::invalid_font("sfnt table directory is too large"))?,
        )
        .ok_or_else(|| FontminError::invalid_font("sfnt table directory is too large"))?;

    if directory_end > input.len() {
        return Err(FontminError::invalid_font(
            "sfnt table directory is truncated",
        ));
    }

    let mut tables = Vec::with_capacity(table_count);

    for index in 0..table_count {
        let record_offset = SFNT_HEADER_SIZE + index * SFNT_TABLE_RECORD_SIZE;
        let tag = read_tag(input, record_offset)?;
        let checksum = read_u32(input, record_offset + 4)?;
        let offset = read_u32(input, record_offset + 8)? as usize;
        let length = read_u32(input, record_offset + 12)? as usize;
        let table_end = offset
            .checked_add(length)
            .ok_or_else(|| FontminError::invalid_font("sfnt table range overflows"))?;

        if table_end > input.len() {
            return Err(FontminError::invalid_font(format!(
                "sfnt table {} points outside the file",
                tag_to_string(tag),
            )));
        }

        tables.push(SfntTable {
            tag,
            checksum,
            offset,
            length,
        });
    }

    tables.sort_by_key(|table| table.tag);

    Ok(tables)
}

fn encode_tables(
    input: &[u8],
    tables: Vec<SfntTable>,
    options: &WoffOptions,
) -> Result<Vec<WoffTable>> {
    let mut output = Vec::with_capacity(tables.len());

    for table in tables {
        let original = read_exact(input, table.offset, table.length)?;
        let data = if options.deflate {
            compressed_or_original(original, options.compression_level)?
        } else {
            original.to_vec()
        };

        output.push(WoffTable {
            tag: table.tag,
            checksum: table.checksum,
            data,
            original_length: table.length,
            offset: 0,
        });
    }

    Ok(output)
}

fn read_woff_tables(input: &[u8], table_count: usize) -> Result<Vec<WoffTable>> {
    let directory_end = WOFF_HEADER_SIZE
        .checked_add(
            table_count
                .checked_mul(WOFF_TABLE_RECORD_SIZE)
                .ok_or_else(|| FontminError::invalid_font("WOFF table directory is too large"))?,
        )
        .ok_or_else(|| FontminError::invalid_font("WOFF table directory is too large"))?;

    if directory_end > input.len() {
        return Err(FontminError::invalid_font(
            "WOFF table directory is truncated",
        ));
    }

    let mut tables = Vec::with_capacity(table_count);

    for index in 0..table_count {
        let record_offset = WOFF_HEADER_SIZE + index * WOFF_TABLE_RECORD_SIZE;
        let tag = read_tag(input, record_offset)?;
        let offset = read_u32(input, record_offset + 4)? as usize;
        let compressed_length = read_u32(input, record_offset + 8)? as usize;
        let original_length = read_u32(input, record_offset + 12)? as usize;
        let checksum = read_u32(input, record_offset + 16)?;
        let table_data = read_exact(input, offset, compressed_length)?;
        let data = decode_table_data(tag, table_data, compressed_length, original_length)?;

        tables.push(WoffTable {
            tag,
            checksum,
            data,
            original_length,
            offset: 0,
        });
    }

    tables.sort_by_key(|table| table.tag);

    Ok(tables)
}

fn decode_table_data(
    tag: [u8; 4],
    data: &[u8],
    compressed_length: usize,
    original_length: usize,
) -> Result<Vec<u8>> {
    if compressed_length > original_length {
        return Err(FontminError::invalid_font(format!(
            "WOFF table {} compressed length exceeds original length",
            tag_to_string(tag),
        )));
    }

    if compressed_length == original_length {
        return Ok(data.to_vec());
    }

    let mut decoder = ZlibDecoder::new(data);
    let mut output = Vec::with_capacity(original_length);

    decoder.read_to_end(&mut output).map_err(|error| {
        FontminError::invalid_font(format!(
            "failed to decompress WOFF table {}: {error}",
            tag_to_string(tag),
        ))
    })?;

    if output.len() != original_length {
        return Err(FontminError::invalid_font(format!(
            "WOFF table {} decompressed length does not match original length",
            tag_to_string(tag),
        )));
    }

    Ok(output)
}

fn compressed_or_original(input: &[u8], level: Option<u32>) -> Result<Vec<u8>> {
    let level = level.unwrap_or(6).min(9);
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(level));
    encoder.write_all(input).map_err(|error| {
        FontminError::invalid_font(format!("failed to compress table: {error}"))
    })?;
    let compressed = encoder.finish().map_err(|error| {
        FontminError::invalid_font(format!("failed to finish compression: {error}"))
    })?;

    if compressed.len() < input.len() {
        Ok(compressed)
    } else {
        Ok(input.to_vec())
    }
}

fn sfnt_size(tables: &[WoffTable]) -> Result<u32> {
    let directory_size = SFNT_HEADER_SIZE
        .checked_add(
            tables
                .len()
                .checked_mul(SFNT_TABLE_RECORD_SIZE)
                .ok_or_else(|| FontminError::invalid_font("sfnt table directory is too large"))?,
        )
        .ok_or_else(|| FontminError::invalid_font("sfnt table directory is too large"))?;
    let table_data_size = tables.iter().try_fold(0usize, |total, table| {
        total
            .checked_add(padded_len(table.original_length))
            .ok_or_else(|| FontminError::invalid_font("sfnt table data is too large"))
    })?;

    checked_u32(
        directory_size
            .checked_add(table_data_size)
            .ok_or_else(|| FontminError::invalid_font("sfnt size is too large"))?,
        "sfnt size",
    )
}

fn assign_offsets(start_offset: usize, tables: &mut [WoffTable]) -> Result<usize> {
    let mut offset = start_offset;

    for table in tables {
        table.offset = offset;
        offset = offset
            .checked_add(padded_len(table.data.len()))
            .ok_or_else(|| FontminError::invalid_font("WOFF file is too large"))?;
    }

    Ok(offset)
}

fn write_sfnt(flavor: u32, tables: &mut [WoffTable]) -> Result<Vec<u8>> {
    let directory_size = SFNT_HEADER_SIZE
        .checked_add(
            tables
                .len()
                .checked_mul(SFNT_TABLE_RECORD_SIZE)
                .ok_or_else(|| FontminError::invalid_font("sfnt table directory is too large"))?,
        )
        .ok_or_else(|| FontminError::invalid_font("sfnt table directory is too large"))?;
    let total_length = assign_offsets(directory_size, tables)?;
    let table_count = checked_u16(tables.len(), "sfnt table count")?;
    let (search_range, entry_selector, range_shift) = sfnt_search_params(tables.len())?;
    let mut output = Vec::with_capacity(total_length);

    write_u32(&mut output, flavor);
    write_u16(&mut output, table_count);
    write_u16(&mut output, search_range);
    write_u16(&mut output, entry_selector);
    write_u16(&mut output, range_shift);

    for table in tables.iter() {
        write_bytes(&mut output, &table.tag);
        write_u32(&mut output, table.checksum);
        write_u32(&mut output, checked_u32(table.offset, "sfnt table offset")?);
        write_u32(
            &mut output,
            checked_u32(table.original_length, "sfnt table length")?,
        );
    }

    write_table_data(&mut output, tables);

    Ok(output)
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

fn write_header(
    output: &mut Vec<u8>,
    flavor: u32,
    length: usize,
    table_count: u16,
    total_sfnt_size: u32,
) -> Result<()> {
    write_bytes(output, b"wOFF");
    write_u32(output, flavor);
    write_u32(output, checked_u32(length, "WOFF length")?);
    write_u16(output, table_count);
    write_u16(output, 0);
    write_u32(output, total_sfnt_size);
    write_u16(output, 1);
    write_u16(output, 0);
    write_u32(output, 0);
    write_u32(output, 0);
    write_u32(output, 0);
    write_u32(output, 0);
    write_u32(output, 0);

    Ok(())
}

fn write_table_directory(output: &mut Vec<u8>, tables: &[WoffTable]) -> Result<()> {
    for table in tables {
        write_bytes(output, &table.tag);
        write_u32(output, checked_u32(table.offset, "WOFF table offset")?);
        write_u32(
            output,
            checked_u32(table.data.len(), "WOFF table compressed length")?,
        );
        write_u32(
            output,
            checked_u32(table.original_length, "WOFF table original length")?,
        );
        write_u32(output, table.checksum);
    }

    Ok(())
}

fn write_table_data(output: &mut Vec<u8>, tables: &[WoffTable]) {
    for table in tables {
        write_bytes(output, &table.data);
        while !output.len().is_multiple_of(4) {
            output.push(0);
        }
    }
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

fn read_tag(input: &[u8], offset: usize) -> Result<[u8; 4]> {
    let bytes = read_exact(input, offset, 4)?;

    Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_u16(input: &[u8], offset: usize) -> Result<u16> {
    let bytes = read_exact(input, offset, 2)?;

    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
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

fn write_bytes(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(bytes);
}

fn write_u16(output: &mut Vec<u8>, value: u16) {
    write_bytes(output, &value.to_be_bytes());
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
    write_bytes(output, &value.to_be_bytes());
}

fn tag_to_string(tag: [u8; 4]) -> String {
    String::from_utf8_lossy(&tag).to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        WoffOptions, decode_woff_to_ttf, encode_ttf_to_woff, read_tag, read_u16, read_u32,
    };

    const ROBOTO: &[u8] = include_bytes!("../../../fixtures/fonts/ttf/roboto-regular.ttf");

    #[test]
    fn decodes_encoded_woff_to_valid_ttf() {
        let woff = encode_ttf_to_woff(ROBOTO, &WoffOptions::default()).unwrap();
        let output = decode_woff_to_ttf(&woff).unwrap();

        assert!(output.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert_eq!(output.len(), read_u32(&woff, 16).unwrap() as usize);
        assert_eq!(read_u16(&output, 4).unwrap(), read_u16(ROBOTO, 4).unwrap());
        assert_eq!(table_tags(&output), table_tags(ROBOTO));
    }

    fn table_tags(input: &[u8]) -> Vec<[u8; 4]> {
        let table_count = usize::from(read_u16(input, 4).unwrap());

        (0..table_count)
            .map(|index| read_tag(input, 12 + index * 16).unwrap())
            .collect()
    }
}
