use std::{
    collections::HashSet,
    io::{Read, Write},
};

use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};

const WOFF_HEADER_SIZE: usize = 44;
const WOFF_TABLE_RECORD_SIZE: usize = 20;
const SFNT_HEADER_SIZE: usize = 12;
const SFNT_TABLE_RECORD_SIZE: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
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

#[derive(Debug, Clone)]
struct WoffTableDirectory {
    tables: Vec<WoffTable>,
    data_ranges: Vec<WoffDataRange>,
}

#[derive(Debug, Clone, Copy)]
struct WoffDataRange {
    start: usize,
    end: usize,
}

impl WoffDataRange {
    fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

#[derive(Debug, Clone, Default)]
struct WoffAuxiliaryBlocks {
    metadata: Option<WoffMetadataBlock>,
    private_data: Option<WoffPrivateBlock>,
}

#[derive(Debug, Clone)]
struct WoffMetadataBlock {
    compressed: Vec<u8>,
    offset: usize,
    original_length: usize,
}

#[derive(Debug, Clone)]
struct WoffPrivateBlock {
    data: Vec<u8>,
    offset: usize,
}

pub fn encode_ttf_to_woff(input: &[u8], options: &WoffOptions) -> Result<Vec<u8>> {
    if !is_ttf(input) {
        return Err(FontminError::invalid_font(
            "expected TrueType sfnt data for WOFF encoding",
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
    let table_data_end = assign_offsets(directory_size, &mut woff_tables)?;
    let auxiliary_blocks = build_auxiliary_blocks(table_data_end, options)?;
    let total_length = auxiliary_total_length(table_data_end, &auxiliary_blocks)?;

    let mut output = Vec::with_capacity(total_length);
    write_header(
        &mut output,
        flavor,
        total_length,
        checked_u16(woff_tables.len(), "WOFF table count")?,
        total_sfnt_size,
        &auxiliary_blocks,
    )?;
    write_table_directory(&mut output, &woff_tables)?;
    write_table_data(&mut output, &woff_tables);
    write_auxiliary_blocks(&mut output, &auxiliary_blocks);

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
    let reserved = read_u16(input, 14)?;
    let total_sfnt_size = read_u32(input, 16)?;
    let major_version = read_u16(input, 20)?;
    let minor_version = read_u16(input, 22)?;
    let metadata_offset = read_u32(input, 24)? as usize;
    let metadata_length = read_u32(input, 28)? as usize;
    let metadata_original_length = read_u32(input, 32)? as usize;
    let private_offset = read_u32(input, 36)? as usize;
    let private_length = read_u32(input, 40)? as usize;

    if declared_length != input.len() {
        return Err(FontminError::invalid_font(
            "WOFF declared length does not match file length",
        ));
    }
    if reserved != 0 {
        return Err(FontminError::invalid_font(
            "WOFF reserved field is non-zero",
        ));
    }
    if major_version != 1 || minor_version != 0 {
        return Err(FontminError::invalid_font("WOFF version is not supported"));
    }
    if table_count == 0 {
        return Err(FontminError::invalid_font("WOFF contains no tables"));
    }
    let metadata_range =
        validate_optional_block(input, "WOFF metadata", metadata_offset, metadata_length)?;
    let private_data_range =
        validate_optional_block(input, "WOFF private data", private_offset, private_length)?;
    if metadata_offset == 0 && metadata_original_length != 0 {
        return Err(FontminError::invalid_font(
            "WOFF metadata original length is non-zero without metadata",
        ));
    }

    let WoffTableDirectory {
        mut tables,
        data_ranges,
    } = read_woff_tables(input, table_count)?;
    validate_auxiliary_block_layout(metadata_range, private_data_range, &data_ranges)?;
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
    let mut seen_tags = HashSet::with_capacity(table_count);

    for index in 0..table_count {
        let record_offset = SFNT_HEADER_SIZE + index * SFNT_TABLE_RECORD_SIZE;
        let tag = read_tag(input, record_offset)?;

        if !seen_tags.insert(tag) {
            return Err(FontminError::invalid_font(format!(
                "duplicate sfnt table tag {}",
                tag_to_string(tag),
            )));
        }

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

fn read_woff_tables(input: &[u8], table_count: usize) -> Result<WoffTableDirectory> {
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
    let mut data_ranges = Vec::with_capacity(table_count);
    let mut seen_tags = HashSet::with_capacity(table_count);

    for index in 0..table_count {
        let record_offset = WOFF_HEADER_SIZE + index * WOFF_TABLE_RECORD_SIZE;
        let tag = read_tag(input, record_offset)?;
        let offset = read_u32(input, record_offset + 4)? as usize;
        let compressed_length = read_u32(input, record_offset + 8)? as usize;
        let original_length = read_u32(input, record_offset + 12)? as usize;
        let checksum = read_u32(input, record_offset + 16)?;

        if offset < directory_end {
            return Err(FontminError::invalid_font(
                "WOFF table data begins inside the table directory",
            ));
        }
        if !offset.is_multiple_of(4) {
            return Err(FontminError::invalid_font(
                "WOFF table data offset is not 4-byte aligned",
            ));
        }

        let table_end = offset
            .checked_add(compressed_length)
            .ok_or_else(|| FontminError::invalid_font("WOFF table data range overflows"))?;

        let data_range = WoffDataRange {
            start: offset,
            end: table_end,
        };

        if data_ranges.iter().any(|range| data_range.overlaps(*range)) {
            return Err(FontminError::invalid_font("WOFF table data ranges overlap"));
        }

        data_ranges.push(data_range);

        let table_data = read_exact(input, offset, compressed_length)?;
        let data = decode_table_data(tag, table_data, compressed_length, original_length)?;

        if !seen_tags.insert(tag) {
            return Err(FontminError::invalid_font(format!(
                "duplicate WOFF table tag {}",
                tag_to_string(tag),
            )));
        }

        tables.push(WoffTable {
            tag,
            checksum,
            data,
            original_length,
            offset: 0,
        });
    }

    tables.sort_by_key(|table| table.tag);

    Ok(WoffTableDirectory {
        tables,
        data_ranges,
    })
}

fn validate_optional_block(
    input: &[u8],
    label: &str,
    offset: usize,
    length: usize,
) -> Result<Option<WoffDataRange>> {
    if offset == 0 && length == 0 {
        return Ok(None);
    }
    if offset == 0 || length == 0 {
        return Err(FontminError::invalid_font(format!(
            "{label} block has an incomplete range",
        )));
    }
    if !offset.is_multiple_of(4) {
        return Err(FontminError::invalid_font(format!(
            "{label} block offset is not 4-byte aligned",
        )));
    }

    let end = offset
        .checked_add(length)
        .ok_or_else(|| FontminError::invalid_font(format!("{label} block range overflows")))?;

    if end > input.len() {
        return Err(FontminError::invalid_font(format!(
            "{label} block points outside the file",
        )));
    }

    Ok(Some(WoffDataRange { start: offset, end }))
}

fn validate_auxiliary_block_layout(
    metadata: Option<WoffDataRange>,
    private_data: Option<WoffDataRange>,
    table_data: &[WoffDataRange],
) -> Result<()> {
    if let Some(metadata) = metadata {
        if table_data.iter().any(|range| metadata.overlaps(*range)) {
            return Err(FontminError::invalid_font(
                "WOFF metadata block overlaps table data",
            ));
        }

        if private_data.is_some_and(|private_data| metadata.overlaps(private_data)) {
            return Err(FontminError::invalid_font(
                "WOFF metadata block overlaps private data",
            ));
        }
    }

    if let Some(private_data) = private_data
        && table_data.iter().any(|range| private_data.overlaps(*range))
    {
        return Err(FontminError::invalid_font(
            "WOFF private data block overlaps table data",
        ));
    }

    Ok(())
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

fn build_auxiliary_blocks(
    table_data_end: usize,
    options: &WoffOptions,
) -> Result<WoffAuxiliaryBlocks> {
    let metadata = match options
        .metadata
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        Some(metadata) => Some(WoffMetadataBlock {
            compressed: compress_metadata(metadata.as_bytes())?,
            offset: table_data_end,
            original_length: metadata.len(),
        }),
        None => None,
    };
    let private_offset = match &metadata {
        Some(metadata) => padded_len(
            metadata
                .offset
                .checked_add(metadata.compressed.len())
                .ok_or_else(|| FontminError::invalid_font("WOFF metadata block is too large"))?,
        ),
        None => table_data_end,
    };
    let private_data = options
        .private_data
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(|data| WoffPrivateBlock {
            data: data.to_vec(),
            offset: private_offset,
        });

    Ok(WoffAuxiliaryBlocks {
        metadata,
        private_data,
    })
}

fn auxiliary_total_length(
    table_data_end: usize,
    auxiliary_blocks: &WoffAuxiliaryBlocks,
) -> Result<usize> {
    if let Some(private_data) = &auxiliary_blocks.private_data {
        return private_data
            .offset
            .checked_add(private_data.data.len())
            .ok_or_else(|| FontminError::invalid_font("WOFF private block is too large"));
    }

    if let Some(metadata) = &auxiliary_blocks.metadata {
        return metadata
            .offset
            .checked_add(metadata.compressed.len())
            .ok_or_else(|| FontminError::invalid_font("WOFF metadata block is too large"));
    }

    Ok(table_data_end)
}

fn compress_metadata(input: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(6));

    encoder.write_all(input).map_err(|error| {
        FontminError::invalid_font(format!("failed to compress WOFF metadata: {error}"))
    })?;
    encoder.finish().map_err(|error| {
        FontminError::invalid_font(format!(
            "failed to finish WOFF metadata compression: {error}"
        ))
    })
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
    auxiliary_blocks: &WoffAuxiliaryBlocks,
) -> Result<()> {
    write_bytes(output, b"wOFF");
    write_u32(output, flavor);
    write_u32(output, checked_u32(length, "WOFF length")?);
    write_u16(output, table_count);
    write_u16(output, 0);
    write_u32(output, total_sfnt_size);
    write_u16(output, 1);
    write_u16(output, 0);
    write_u32(
        output,
        optional_u32(
            auxiliary_blocks
                .metadata
                .as_ref()
                .map(|metadata| metadata.offset),
            "WOFF metadata offset",
        )?,
    );
    write_u32(
        output,
        optional_u32(
            auxiliary_blocks
                .metadata
                .as_ref()
                .map(|metadata| metadata.compressed.len()),
            "WOFF metadata length",
        )?,
    );
    write_u32(
        output,
        optional_u32(
            auxiliary_blocks
                .metadata
                .as_ref()
                .map(|metadata| metadata.original_length),
            "WOFF metadata original length",
        )?,
    );
    write_u32(
        output,
        optional_u32(
            auxiliary_blocks
                .private_data
                .as_ref()
                .map(|private_data| private_data.offset),
            "WOFF private data offset",
        )?,
    );
    write_u32(
        output,
        optional_u32(
            auxiliary_blocks
                .private_data
                .as_ref()
                .map(|private_data| private_data.data.len()),
            "WOFF private data length",
        )?,
    );

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

fn write_auxiliary_blocks(output: &mut Vec<u8>, auxiliary_blocks: &WoffAuxiliaryBlocks) {
    if let Some(metadata) = &auxiliary_blocks.metadata {
        pad_to_offset(output, metadata.offset);
        write_bytes(output, &metadata.compressed);
    }

    if let Some(private_data) = &auxiliary_blocks.private_data {
        pad_to_offset(output, private_data.offset);
        write_bytes(output, &private_data.data);
    }
}

fn write_table_data(output: &mut Vec<u8>, tables: &[WoffTable]) {
    for table in tables {
        write_bytes(output, &table.data);
        while !output.len().is_multiple_of(4) {
            output.push(0);
        }
    }
}

fn pad_to_offset(output: &mut Vec<u8>, offset: usize) {
    while output.len() < offset {
        output.push(0);
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

fn optional_u32(value: Option<usize>, label: &str) -> Result<u32> {
    match value {
        Some(value) => checked_u32(value, label),
        None => Ok(0),
    }
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
    use std::io::Read;

    use flate2::read::ZlibDecoder;
    use fontmin_testing::ROBOTO;

    use super::{
        WOFF_HEADER_SIZE, WoffOptions, decode_woff_to_ttf, encode_ttf_to_woff, read_tag, read_u16,
        read_u32,
    };

    #[test]
    fn decodes_encoded_woff_to_valid_ttf() {
        let woff = encode_ttf_to_woff(ROBOTO, &WoffOptions::default()).unwrap();
        let output = decode_woff_to_ttf(&woff).unwrap();

        assert!(output.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert_eq!(output.len(), read_u32(&woff, 16).unwrap() as usize);
        assert_eq!(read_u16(&output, 4).unwrap(), read_u16(ROBOTO, 4).unwrap());
        assert_eq!(table_tags(&output), table_tags(ROBOTO));
    }

    #[test]
    fn encodes_metadata_and_private_data_blocks() {
        let metadata = r#"<?xml version="1.0" encoding="UTF-8"?><metadata version="1.0" />"#;
        let private_data = b"fontmin-rs private data".to_vec();
        let woff = encode_ttf_to_woff(
            ROBOTO,
            &WoffOptions {
                metadata: Some(metadata.to_string()),
                private_data: Some(private_data.clone()),
                ..WoffOptions::default()
            },
        )
        .unwrap();
        let declared_length = read_u32(&woff, 8).unwrap() as usize;
        let meta_offset = read_u32(&woff, 24).unwrap() as usize;
        let meta_length = read_u32(&woff, 28).unwrap() as usize;
        let meta_original_length = read_u32(&woff, 32).unwrap() as usize;
        let private_offset = read_u32(&woff, 36).unwrap() as usize;
        let private_length = read_u32(&woff, 40).unwrap() as usize;
        let mut decoded_metadata = String::new();
        let mut decoder = ZlibDecoder::new(&woff[meta_offset..meta_offset + meta_length]);

        decoder.read_to_string(&mut decoded_metadata).unwrap();

        assert_eq!(declared_length, woff.len());
        assert_eq!(meta_offset % 4, 0);
        assert_eq!(private_offset % 4, 0);
        assert_eq!(meta_original_length, metadata.len());
        assert_eq!(decoded_metadata, metadata);
        assert_eq!(private_length, private_data.len());
        assert_eq!(
            &woff[private_offset..private_offset + private_length],
            private_data.as_slice()
        );
        assert!(
            decode_woff_to_ttf(&woff)
                .unwrap()
                .starts_with(&[0, 1, 0, 0])
        );
    }

    #[test]
    fn rejects_woff_with_duplicate_table_tags() {
        let mut woff = encode_ttf_to_woff(ROBOTO, &WoffOptions::default()).unwrap();
        let first_tag = read_tag(&woff, 44).unwrap();

        woff[64..68].copy_from_slice(&first_tag);

        let error = decode_woff_to_ttf(&woff).unwrap_err();

        assert!(error.to_string().contains("duplicate WOFF table tag"));
    }

    #[test]
    fn rejects_woff_with_non_zero_reserved_field() {
        let mut woff = encode_ttf_to_woff(ROBOTO, &WoffOptions::default()).unwrap();

        woff[14..16].copy_from_slice(&1u16.to_be_bytes());

        let error = decode_woff_to_ttf(&woff).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("WOFF reserved field is non-zero")
        );
    }

    #[test]
    fn rejects_woff_with_unsupported_version() {
        let mut woff = encode_ttf_to_woff(ROBOTO, &WoffOptions::default()).unwrap();

        woff[20..22].copy_from_slice(&2u16.to_be_bytes());

        let error = decode_woff_to_ttf(&woff).unwrap_err();

        assert!(error.to_string().contains("WOFF version is not supported"));
    }

    #[test]
    fn rejects_woff_with_private_data_outside_file() {
        let mut woff = encode_ttf_to_woff(ROBOTO, &WoffOptions::default()).unwrap();
        let outside_offset = u32::try_from(woff.len()).unwrap();

        woff[36..40].copy_from_slice(&outside_offset.to_be_bytes());
        woff[40..44].copy_from_slice(&4u32.to_be_bytes());

        let error = decode_woff_to_ttf(&woff).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("WOFF private data block points outside the file")
        );
    }

    #[test]
    fn rejects_woff_with_table_data_inside_directory() {
        let mut woff = encode_ttf_to_woff(ROBOTO, &WoffOptions::default()).unwrap();
        let header_size = u32::try_from(WOFF_HEADER_SIZE).unwrap();

        woff[48..52].copy_from_slice(&header_size.to_be_bytes());

        let error = decode_woff_to_ttf(&woff).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("WOFF table data begins inside the table directory")
        );
    }

    #[test]
    fn rejects_woff_with_overlapping_table_data() {
        let mut woff = encode_ttf_to_woff(ROBOTO, &WoffOptions::default()).unwrap();
        let first_table_offset = woff[48..52].to_owned();

        woff[68..72].copy_from_slice(&first_table_offset);

        let error = decode_woff_to_ttf(&woff).unwrap_err();

        assert!(error.to_string().contains("WOFF table data ranges overlap"));
    }

    #[test]
    fn rejects_woff_with_metadata_overlapping_table_data() {
        let mut woff = encode_ttf_to_woff(ROBOTO, &WoffOptions::default()).unwrap();
        let first_table_offset = woff[48..52].to_owned();

        woff[24..28].copy_from_slice(&first_table_offset);
        woff[28..32].copy_from_slice(&4u32.to_be_bytes());
        woff[32..36].copy_from_slice(&4u32.to_be_bytes());

        let error = decode_woff_to_ttf(&woff).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("WOFF metadata block overlaps table data")
        );
    }

    #[test]
    fn rejects_woff_with_private_data_overlapping_table_data() {
        let mut woff = encode_ttf_to_woff(ROBOTO, &WoffOptions::default()).unwrap();
        let first_table_offset = woff[48..52].to_owned();

        woff[36..40].copy_from_slice(&first_table_offset);
        woff[40..44].copy_from_slice(&4u32.to_be_bytes());

        let error = decode_woff_to_ttf(&woff).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("WOFF private data block overlaps table data")
        );
    }

    #[test]
    fn rejects_woff_with_metadata_overlapping_private_data() {
        let mut woff = encode_ttf_to_woff(
            ROBOTO,
            &WoffOptions {
                metadata: Some("metadata".into()),
                private_data: Some(b"private data".to_vec()),
                ..WoffOptions::default()
            },
        )
        .unwrap();
        let metadata_offset = woff[24..28].to_owned();

        woff[36..40].copy_from_slice(&metadata_offset);

        let error = decode_woff_to_ttf(&woff).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("WOFF metadata block overlaps private data")
        );
    }

    #[test]
    fn rejects_ttf_with_duplicate_table_tags() {
        let mut font = ROBOTO.to_vec();
        let first_tag = read_tag(&font, 12).unwrap();

        font[28..32].copy_from_slice(&first_tag);

        let error = encode_ttf_to_woff(&font, &WoffOptions::default()).unwrap_err();

        assert!(error.to_string().contains("duplicate sfnt table tag"));
    }

    fn table_tags(input: &[u8]) -> Vec<[u8; 4]> {
        let table_count = usize::from(read_u16(input, 4).unwrap());

        (0..table_count)
            .map(|index| read_tag(input, 12 + index * 16).unwrap())
            .collect()
    }
}
