use std::{
    collections::{HashMap, HashSet},
    io::Read,
};

use fontmin_core::FontMetadata;
use fontmin_diagnostics::{FontminError, Result};
use fontmin_ttf::SfntFlavor;
use serde::{Deserialize, Serialize};
use ttf2woff2::BrotliQuality;

const DEFAULT_BROTLI_QUALITY: u8 = 6;
const WOFF2_HEADER_SIZE: usize = 48;
const WOFF2_SIGNATURE: &[u8; 4] = b"wOF2";
const WOFF2_CUSTOM_TAG_FLAG: u8 = 63;
const SFNT_HEADER_SIZE: u32 = 12;
const SFNT_TABLE_RECORD_SIZE: u32 = 16;
const BROTLI_BUFFER_SIZE: usize = 4096;
const METADATA_TABLES: [[u8; 4]; 4] = [*b"head", *b"hhea", *b"maxp", *b"name"];

const KNOWN_TAGS: [[u8; 4]; 63] = [
    *b"cmap", *b"head", *b"hhea", *b"hmtx", *b"maxp", *b"name", *b"OS/2", *b"post", *b"cvt ",
    *b"fpgm", *b"glyf", *b"loca", *b"prep", *b"CFF ", *b"VORG", *b"EBDT", *b"EBLC", *b"gasp",
    *b"hdmx", *b"kern", *b"LTSH", *b"PCLT", *b"VDMX", *b"vhea", *b"vmtx", *b"BASE", *b"GDEF",
    *b"GPOS", *b"GSUB", *b"EBSC", *b"JSTF", *b"MATH", *b"CBDT", *b"CBLC", *b"COLR", *b"CPAL",
    *b"SVG ", *b"sbix", *b"acnt", *b"avar", *b"bdat", *b"bloc", *b"bsln", *b"cvar", *b"fdsc",
    *b"feat", *b"fmtx", *b"fvar", *b"gvar", *b"hsty", *b"just", *b"lcar", *b"mort", *b"morx",
    *b"opbd", *b"prop", *b"trak", *b"Zapf", *b"Silf", *b"Glat", *b"Gloc", *b"Feat", *b"Sill",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Woff2Options {
    pub quality: Option<u8>,
}

pub fn encode_ttf_to_woff2(input: &[u8], options: &Woff2Options) -> Result<Vec<u8>> {
    if !is_ttf(input) {
        return Err(FontminError::invalid_font(
            "expected TrueType sfnt data for WOFF2 encoding",
        ));
    }

    let quality = BrotliQuality::from(options.quality.unwrap_or(DEFAULT_BROTLI_QUALITY));

    ttf2woff2::encode(input, quality).map_err(woff2_error)
}

pub fn decode_woff2_to_ttf(input: &[u8]) -> Result<Vec<u8>> {
    let mut input = input;

    woff2_patched::convert_woff2_to_ttf(&mut input).map_err(woff2_decode_error)
}

pub fn validate_woff2(input: &[u8]) -> Result<()> {
    parse_woff2(input).map(|_| ())
}

pub fn inspect_woff2(input: &[u8]) -> Result<FontMetadata> {
    let info = parse_woff2(input)?;
    let tables = table_tags(&info.tables)?;
    let decompressed = decompress_table_data(input, &info)?;
    let table_data = table_data_slices(&decompressed, &info.tables)?;

    if let Ok(mut metadata) = inspect_metadata_tables(&info, &table_data) {
        metadata.tables = tables;

        return Ok(metadata);
    }

    Ok(basic_metadata(tables))
}

fn basic_metadata(tables: Vec<String>) -> FontMetadata {
    FontMetadata {
        family_name: None,
        subfamily_name: None,
        full_name: None,
        post_script_name: None,
        glyph_count: 0,
        units_per_em: 0,
        ascender: 0,
        descender: 0,
        tables,
    }
}

fn is_ttf(input: &[u8]) -> bool {
    input.starts_with(&[0x00, 0x01, 0x00, 0x00]) || input.starts_with(b"true")
}

fn woff2_error(error: ttf2woff2::Error) -> FontminError {
    match error {
        ttf2woff2::Error::UnsupportedFormat => {
            FontminError::invalid_font("expected TrueType sfnt data for WOFF2 encoding")
        }
        other => FontminError::ConvertFailed {
            message: format!("failed to encode WOFF2: {other}"),
        },
    }
}

fn woff2_decode_error(error: woff2_patched::decode::DecodeError) -> FontminError {
    match error {
        woff2_patched::decode::DecodeError::Invalid(message) => {
            FontminError::invalid_font(format!("failed to decode WOFF2: {message}"))
        }
        woff2_patched::decode::DecodeError::Unsupported(feature) => {
            FontminError::unsupported(format!("woff2 decode: {feature}"))
        }
    }
}

#[derive(Debug, Clone)]
struct Woff2Info {
    header: Woff2Header,
    tables: Vec<Woff2Table>,
    compressed_offset: usize,
}

#[derive(Debug, Clone, Copy)]
struct Woff2Header {
    flavor: [u8; 4],
    length: usize,
    num_tables: usize,
    reserved: u16,
    total_sfnt_size: u32,
    total_compressed_size: usize,
    meta_offset: usize,
    meta_length: usize,
    meta_orig_length: usize,
    priv_offset: usize,
    priv_length: usize,
}

#[derive(Debug, Clone)]
struct Woff2Directory {
    tables: Vec<Woff2Table>,
    end: usize,
}

#[derive(Debug, Clone, Copy)]
struct Woff2Table {
    tag: [u8; 4],
    original_length: u32,
    transform_version: u8,
    transformed_length: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
struct Woff2DataRange {
    start: usize,
    end: usize,
}

impl Woff2DataRange {
    fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

fn parse_woff2(input: &[u8]) -> Result<Woff2Info> {
    let header = read_header(input)?;

    validate_header(input, header)?;
    let directory = read_table_directory(input, header.num_tables)?;
    validate_table_set(&directory.tables)?;
    validate_total_sfnt_size(header.total_sfnt_size, header.num_tables, &directory.tables)?;
    let compressed_end = validate_compressed_block(input, header, directory.end)?;
    let metadata_range = validate_optional_block(
        input,
        "WOFF2 metadata",
        header.meta_offset,
        header.meta_length,
        compressed_end,
    )?;
    let private_data_range = validate_optional_block(
        input,
        "WOFF2 private data",
        header.priv_offset,
        header.priv_length,
        compressed_end,
    )?;

    if header.meta_offset == 0 && header.meta_orig_length != 0 {
        return Err(FontminError::invalid_font(
            "WOFF2 metadata original length is non-zero without metadata",
        ));
    }
    if metadata_range.is_some_and(|metadata| {
        private_data_range.is_some_and(|private_data| metadata.overlaps(private_data))
    }) {
        return Err(FontminError::invalid_font(
            "WOFF2 metadata block overlaps private data",
        ));
    }

    Ok(Woff2Info {
        header,
        tables: directory.tables,
        compressed_offset: directory.end,
    })
}

fn read_header(input: &[u8]) -> Result<Woff2Header> {
    if input.len() < WOFF2_SIGNATURE.len() {
        return Err(FontminError::invalid_font("WOFF2 header is truncated"));
    }
    if !input.starts_with(WOFF2_SIGNATURE) {
        return Err(FontminError::invalid_font(
            "expected WOFF2 data for inspection",
        ));
    }
    if input.len() < WOFF2_HEADER_SIZE {
        return Err(FontminError::invalid_font("WOFF2 header is truncated"));
    }

    Ok(Woff2Header {
        flavor: read_array::<4>(input, 4, "WOFF2 flavor")?,
        length: read_u32_as_usize(input, 8, "WOFF2 declared length")?,
        num_tables: usize::from(read_u16(input, 12)?),
        reserved: read_u16(input, 14)?,
        total_sfnt_size: read_u32(input, 16)?,
        total_compressed_size: read_u32_as_usize(input, 20, "WOFF2 compressed size")?,
        meta_offset: read_u32_as_usize(input, 28, "WOFF2 metadata offset")?,
        meta_length: read_u32_as_usize(input, 32, "WOFF2 metadata length")?,
        meta_orig_length: read_u32_as_usize(input, 36, "WOFF2 metadata original length")?,
        priv_offset: read_u32_as_usize(input, 40, "WOFF2 private offset")?,
        priv_length: read_u32_as_usize(input, 44, "WOFF2 private length")?,
    })
}

fn validate_header(input: &[u8], header: Woff2Header) -> Result<()> {
    if header.length != input.len() {
        return Err(FontminError::invalid_font(
            "WOFF2 declared length does not match file length",
        ));
    }
    if !header.length.is_multiple_of(4) {
        return Err(FontminError::invalid_font(
            "WOFF2 declared length is not 4-byte aligned",
        ));
    }
    if header.num_tables == 0 {
        return Err(FontminError::invalid_font("WOFF2 contains no tables"));
    }
    if header.reserved != 0 {
        return Err(FontminError::invalid_font(
            "WOFF2 reserved field is non-zero",
        ));
    }
    if header.total_compressed_size == 0 {
        return Err(FontminError::invalid_font(
            "WOFF2 compressed data block is empty",
        ));
    }

    Ok(())
}

fn read_table_directory(input: &[u8], table_count: usize) -> Result<Woff2Directory> {
    let mut offset = WOFF2_HEADER_SIZE;
    let mut tables = Vec::with_capacity(table_count);
    let mut seen = HashSet::with_capacity(table_count);

    for _ in 0..table_count {
        let flags = read_byte(input, &mut offset)?;
        let tag_flag = flags & 0x3f;
        let transform_version = flags >> 6;
        let tag = if tag_flag == WOFF2_CUSTOM_TAG_FLAG {
            read_tag(input, &mut offset)?
        } else {
            KNOWN_TAGS[usize::from(tag_flag)]
        };

        if !seen.insert(tag) {
            return Err(FontminError::invalid_font(format!(
                "WOFF2 table {} is duplicated",
                tag_to_string(tag)?
            )));
        }

        validate_transform_version(tag, transform_version)?;

        let original_length = read_uint_base128(input, &mut offset, "WOFF2 table length")?;
        let transformed_length = has_transform_length(tag, transform_version)
            .then(|| read_uint_base128(input, &mut offset, "WOFF2 transformed table length"))
            .transpose()?;

        tables.push(Woff2Table {
            tag,
            original_length,
            transform_version,
            transformed_length,
        });
    }

    Ok(Woff2Directory {
        tables,
        end: offset,
    })
}

fn validate_table_set(tables: &[Woff2Table]) -> Result<()> {
    let glyf_index = tables.iter().position(|table| table.tag == *b"glyf");
    let loca_index = tables.iter().position(|table| table.tag == *b"loca");

    match (glyf_index, loca_index) {
        (Some(glyf), Some(loca)) if loca != glyf + 1 => Err(FontminError::invalid_font(
            "WOFF2 loca table must immediately follow glyf table",
        )),
        (Some(_), None) => Err(FontminError::invalid_font(
            "WOFF2 glyf table requires a loca table",
        )),
        (None, Some(_)) => Err(FontminError::invalid_font(
            "WOFF2 loca table requires a glyf table",
        )),
        _ => Ok(()),
    }
}

fn validate_total_sfnt_size(
    declared_size: u32,
    table_count: usize,
    tables: &[Woff2Table],
) -> Result<()> {
    let expected_size = sfnt_size(table_count, tables)?;

    if expected_size != declared_size {
        return Err(FontminError::invalid_font(
            "WOFF2 total sfnt size does not match table directory",
        ));
    }

    Ok(())
}

fn validate_compressed_block(
    input: &[u8],
    header: Woff2Header,
    directory_end: usize,
) -> Result<usize> {
    let compressed_end = directory_end
        .checked_add(header.total_compressed_size)
        .ok_or_else(|| FontminError::invalid_font("WOFF2 compressed data range overflows"))?;

    if compressed_end > input.len() {
        return Err(FontminError::invalid_font(
            "WOFF2 compressed data block points outside the file",
        ));
    }

    Ok(compressed_end)
}

fn decompress_table_data(input: &[u8], info: &Woff2Info) -> Result<Vec<u8>> {
    let compressed_end = info
        .compressed_offset
        .checked_add(info.header.total_compressed_size)
        .ok_or_else(|| FontminError::invalid_font("WOFF2 compressed data range overflows"))?;
    let compressed = input
        .get(info.compressed_offset..compressed_end)
        .ok_or_else(|| FontminError::invalid_font("WOFF2 compressed data block is truncated"))?;
    let expected_length = stored_tables_length(&info.tables)?;
    let mut decoder = brotli::Decompressor::new(compressed, BROTLI_BUFFER_SIZE);
    let mut output = Vec::with_capacity(expected_length);

    decoder.read_to_end(&mut output).map_err(|error| {
        FontminError::invalid_font(format!("failed to decompress WOFF2 table data: {error}"))
    })?;

    if output.len() != expected_length {
        return Err(FontminError::invalid_font(
            "WOFF2 decompressed table data length does not match table directory",
        ));
    }

    Ok(output)
}

fn table_data_slices<'a>(
    data: &'a [u8],
    tables: &[Woff2Table],
) -> Result<HashMap<[u8; 4], &'a [u8]>> {
    let mut offset = 0usize;
    let mut output = HashMap::with_capacity(tables.len());

    for table in tables {
        let length = stored_table_length(table)?;
        let end = offset
            .checked_add(length)
            .ok_or_else(|| FontminError::invalid_font("WOFF2 table data offset overflows"))?;
        let table_data = data.get(offset..end).ok_or_else(|| {
            FontminError::invalid_font(format!(
                "WOFF2 table {} points outside decompressed data",
                tag_to_string(table.tag).unwrap_or_else(|_| "<invalid>".into()),
            ))
        })?;

        output.insert(table.tag, table_data);
        offset = end;
    }

    if offset != data.len() {
        return Err(FontminError::invalid_font(
            "WOFF2 decompressed table data has trailing bytes",
        ));
    }

    Ok(output)
}

fn inspect_metadata_tables(
    info: &Woff2Info,
    table_data: &HashMap<[u8; 4], &[u8]>,
) -> Result<FontMetadata> {
    let flavor = sfnt_flavor(info.header.flavor)?;
    let sfnt = build_metadata_sfnt(info.header.flavor, &info.tables, table_data)?;

    fontmin_ttf::inspect_sfnt(&sfnt, flavor)
}

fn build_metadata_sfnt(
    flavor: [u8; 4],
    directory: &[Woff2Table],
    table_data: &HashMap<[u8; 4], &[u8]>,
) -> Result<Vec<u8>> {
    let mut tables = Vec::with_capacity(METADATA_TABLES.len());

    for tag in METADATA_TABLES {
        let table = directory
            .iter()
            .find(|table| table.tag == tag)
            .ok_or_else(|| {
                FontminError::invalid_font(format!(
                    "missing required WOFF2 metadata table {}",
                    tag_to_string(tag).unwrap_or_else(|_| "<invalid>".into()),
                ))
            })?;
        if !table_is_stored_raw(table) {
            return Err(FontminError::invalid_font(format!(
                "WOFF2 metadata table {} is transformed",
                tag_to_string(tag).unwrap_or_else(|_| "<invalid>".into()),
            )));
        }

        let data = table_data.get(&tag).copied().ok_or_else(|| {
            FontminError::invalid_font(format!(
                "missing decompressed WOFF2 metadata table {}",
                tag_to_string(tag).unwrap_or_else(|_| "<invalid>".into()),
            ))
        })?;
        if data.len() != table.original_length as usize {
            return Err(FontminError::invalid_font(format!(
                "WOFF2 metadata table {} length does not match original length",
                tag_to_string(tag).unwrap_or_else(|_| "<invalid>".into()),
            )));
        }

        tables.push((tag, data));
    }

    tables.sort_by_key(|(tag, _)| *tag);
    write_metadata_sfnt(flavor, &tables)
}

fn write_metadata_sfnt(flavor: [u8; 4], tables: &[([u8; 4], &[u8])]) -> Result<Vec<u8>> {
    let table_count = u16::try_from(tables.len())
        .map_err(|_| FontminError::invalid_font("WOFF2 metadata table count is too large"))?;
    let directory_size = 12usize
        .checked_add(
            tables
                .len()
                .checked_mul(16)
                .ok_or_else(|| FontminError::invalid_font("WOFF2 metadata sfnt is too large"))?,
        )
        .ok_or_else(|| FontminError::invalid_font("WOFF2 metadata sfnt is too large"))?;
    let mut offsets = Vec::with_capacity(tables.len());
    let mut offset = directory_size;

    for (_, data) in tables {
        offsets.push(offset);
        offset = padded_len(
            offset
                .checked_add(data.len())
                .ok_or_else(|| FontminError::invalid_font("WOFF2 metadata sfnt is too large"))?,
        );
    }

    let mut output = vec![0; directory_size];
    output[0..4].copy_from_slice(&flavor);
    write_u16(&mut output, 4, table_count)?;

    for (index, ((tag, data), table_offset)) in tables.iter().zip(offsets.iter()).enumerate() {
        let record_offset = 12 + index * 16;
        output[record_offset..record_offset + 4].copy_from_slice(tag);
        write_u32(
            &mut output,
            record_offset + 8,
            checked_u32(*table_offset, "table offset")?,
        )?;
        write_u32(
            &mut output,
            record_offset + 12,
            checked_u32(data.len(), "table length")?,
        )?;
    }

    for ((_, data), table_offset) in tables.iter().zip(offsets) {
        output.resize(table_offset, 0);
        output.extend_from_slice(data);
        output.resize(padded_len(output.len()), 0);
    }

    Ok(output)
}

fn sfnt_flavor(flavor: [u8; 4]) -> Result<SfntFlavor> {
    match &flavor {
        [0x00, 0x01, 0x00, 0x00] | b"true" => Ok(SfntFlavor::TrueType),
        b"OTTO" => Ok(SfntFlavor::OpenTypeCff),
        _ => Err(FontminError::invalid_font("unsupported WOFF2 sfnt flavor")),
    }
}

fn table_tags(tables: &[Woff2Table]) -> Result<Vec<String>> {
    let mut tags = tables
        .iter()
        .map(|table| tag_to_string(table.tag))
        .collect::<Result<Vec<_>>>()?;
    tags.sort_unstable();

    Ok(tags)
}

fn stored_tables_length(tables: &[Woff2Table]) -> Result<usize> {
    tables.iter().try_fold(0usize, |length, table| {
        length
            .checked_add(stored_table_length(table)?)
            .ok_or_else(|| FontminError::invalid_font("WOFF2 stored table data is too large"))
    })
}

fn stored_table_length(table: &Woff2Table) -> Result<usize> {
    let length = table.transformed_length.unwrap_or(table.original_length);

    usize::try_from(length)
        .map_err(|_| FontminError::invalid_font("WOFF2 table length is too large"))
}

fn table_is_stored_raw(table: &Woff2Table) -> bool {
    !is_glyf_or_loca(table.tag)
        && table.transform_version == 0
        && table.transformed_length.is_none()
}

fn validate_optional_block(
    input: &[u8],
    label: &'static str,
    offset: usize,
    length: usize,
    minimum_offset: usize,
) -> Result<Option<Woff2DataRange>> {
    if offset == 0 {
        if length != 0 {
            return Err(FontminError::invalid_font(format!(
                "{label} length is non-zero without an offset",
            )));
        }

        return Ok(None);
    }

    if length == 0 {
        return Err(FontminError::invalid_font(format!(
            "{label} offset is non-zero without a length",
        )));
    }
    if !offset.is_multiple_of(4) {
        return Err(FontminError::invalid_font(format!(
            "{label} offset is not 4-byte aligned",
        )));
    }
    if offset < minimum_offset {
        return Err(FontminError::invalid_font(format!(
            "{label} block begins before compressed data ends",
        )));
    }

    let end = offset
        .checked_add(length)
        .ok_or_else(|| FontminError::invalid_font(format!("{label} range overflows")))?;
    if end > input.len() {
        return Err(FontminError::invalid_font(format!(
            "{label} points outside the file",
        )));
    }

    Ok(Some(Woff2DataRange { start: offset, end }))
}

fn validate_transform_version(tag: [u8; 4], transform_version: u8) -> Result<()> {
    if is_glyf_or_loca(tag) && !matches!(transform_version, 0 | 3) {
        return Err(FontminError::invalid_font(
            "WOFF2 glyf/loca table has an invalid transform version",
        ));
    }

    Ok(())
}

fn has_transform_length(tag: [u8; 4], transform_version: u8) -> bool {
    if is_glyf_or_loca(tag) {
        return transform_version == 0;
    }

    transform_version != 0
}

fn is_glyf_or_loca(tag: [u8; 4]) -> bool {
    tag == *b"glyf" || tag == *b"loca"
}

fn sfnt_size(table_count: usize, tables: &[Woff2Table]) -> Result<u32> {
    let table_count = u32::try_from(table_count)
        .map_err(|_| FontminError::invalid_font("WOFF2 table count is too large"))?;
    let table_records_size = table_count
        .checked_mul(SFNT_TABLE_RECORD_SIZE)
        .ok_or_else(|| FontminError::invalid_font("WOFF2 sfnt size overflows"))?;
    let table_data_size = tables.iter().try_fold(0u32, |size, table| {
        size.checked_add(align4(table.original_length)?)
            .ok_or_else(|| FontminError::invalid_font("WOFF2 sfnt size overflows"))
    })?;

    SFNT_HEADER_SIZE
        .checked_add(table_records_size)
        .and_then(|size| size.checked_add(table_data_size))
        .ok_or_else(|| FontminError::invalid_font("WOFF2 sfnt size overflows"))
}

fn align4(value: u32) -> Result<u32> {
    value
        .checked_add(3)
        .map(|value| value & !3)
        .ok_or_else(|| FontminError::invalid_font("WOFF2 table length overflows"))
}

fn padded_len(value: usize) -> usize {
    (value + 3) & !3
}

fn read_byte(input: &[u8], offset: &mut usize) -> Result<u8> {
    let byte = *input
        .get(*offset)
        .ok_or_else(|| FontminError::invalid_font("WOFF2 table directory is truncated"))?;
    *offset += 1;

    Ok(byte)
}

fn read_tag(input: &[u8], offset: &mut usize) -> Result<[u8; 4]> {
    let tag = read_array::<4>(input, *offset, "WOFF2 custom table tag")?;
    *offset += 4;

    Ok(tag)
}

fn read_uint_base128(input: &[u8], offset: &mut usize, field: &'static str) -> Result<u32> {
    let mut value = 0u32;

    for index in 0..5 {
        let byte = read_byte(input, offset)?;
        if index == 0 && byte == 0x80 {
            return Err(FontminError::invalid_font(format!(
                "{field} has leading zeroes",
            )));
        }
        if value > (u32::MAX >> 7) {
            return Err(FontminError::invalid_font(format!("{field} overflows")));
        }

        value = (value << 7) | u32::from(byte & 0x7f);

        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }

    Err(FontminError::invalid_font(format!(
        "{field} exceeds five bytes",
    )))
}

fn read_u16(input: &[u8], offset: usize) -> Result<u16> {
    let bytes = read_array::<2>(input, offset, "WOFF2 u16 field")?;

    Ok(u16::from_be_bytes(bytes))
}

fn read_u32(input: &[u8], offset: usize) -> Result<u32> {
    let bytes = read_array::<4>(input, offset, "WOFF2 u32 field")?;

    Ok(u32::from_be_bytes(bytes))
}

fn write_u16(output: &mut [u8], offset: usize, value: u16) -> Result<()> {
    let end = offset
        .checked_add(2)
        .ok_or_else(|| FontminError::invalid_font("sfnt write offset overflows"))?;
    let bytes = output
        .get_mut(offset..end)
        .ok_or_else(|| FontminError::invalid_font("sfnt write offset is out of bounds"))?;
    bytes.copy_from_slice(&value.to_be_bytes());

    Ok(())
}

fn write_u32(output: &mut [u8], offset: usize, value: u32) -> Result<()> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| FontminError::invalid_font("sfnt write offset overflows"))?;
    let bytes = output
        .get_mut(offset..end)
        .ok_or_else(|| FontminError::invalid_font("sfnt write offset is out of bounds"))?;
    bytes.copy_from_slice(&value.to_be_bytes());

    Ok(())
}

fn checked_u32(value: usize, label: &'static str) -> Result<u32> {
    u32::try_from(value).map_err(|_| FontminError::invalid_font(format!("{label} is too large")))
}

fn read_u32_as_usize(input: &[u8], offset: usize, field: &'static str) -> Result<usize> {
    usize::try_from(read_u32(input, offset)?)
        .map_err(|_| FontminError::invalid_font(format!("{field} is too large")))
}

fn read_array<const N: usize>(input: &[u8], offset: usize, label: &'static str) -> Result<[u8; N]> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| FontminError::invalid_font(format!("{label} range overflows")))?;
    let bytes = input
        .get(offset..end)
        .ok_or_else(|| FontminError::invalid_font(format!("{label} is truncated")))?;

    bytes
        .try_into()
        .map_err(|_| FontminError::invalid_font(format!("{label} is truncated")))
}

fn tag_to_string(tag: [u8; 4]) -> Result<String> {
    std::str::from_utf8(&tag)
        .map(str::to_string)
        .map_err(|_| FontminError::invalid_font("WOFF2 table tag is not ASCII"))
}

#[cfg(test)]
mod tests {
    use fontmin_diagnostics::FontminErrorKind;
    use fontmin_testing::ROBOTO;

    use super::{
        Woff2Options, decode_woff2_to_ttf, encode_ttf_to_woff2, inspect_woff2, validate_woff2,
    };

    #[test]
    fn inspect_woff2_reports_table_directory() {
        let woff2 = encode_ttf_to_woff2(ROBOTO, &Woff2Options::default()).unwrap();
        let metadata = inspect_woff2(&woff2).unwrap();

        assert_eq!(metadata.family_name.as_deref(), Some("Roboto"));
        assert_eq!(metadata.subfamily_name.as_deref(), Some("Regular"));
        assert_eq!(metadata.full_name.as_deref(), Some("Roboto Regular"));
        assert_eq!(metadata.post_script_name.as_deref(), Some("Roboto-Regular"));
        assert_eq!(metadata.glyph_count, 3387);
        assert_eq!(metadata.units_per_em, 2048);
        assert_eq!(metadata.ascender, 2146);
        assert_eq!(metadata.descender, -555);
        assert!(metadata.tables.contains(&"cmap".into()));
        assert!(metadata.tables.contains(&"glyf".into()));
        assert!(metadata.tables.contains(&"loca".into()));
        assert!(metadata.tables.contains(&"name".into()));
    }

    #[test]
    fn validate_woff2_accepts_encoded_output() {
        let woff2 = encode_ttf_to_woff2(ROBOTO, &Woff2Options::default()).unwrap();

        validate_woff2(&woff2).unwrap();
    }

    #[test]
    fn decode_woff2_returns_valid_ttf() {
        let woff2 = encode_ttf_to_woff2(ROBOTO, &Woff2Options::default()).unwrap();
        let ttf = decode_woff2_to_ttf(&woff2).unwrap();
        let metadata = fontmin_ttf::inspect_ttf(&ttf).unwrap();

        assert!(ttf.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert_eq!(metadata.family_name.as_deref(), Some("Roboto"));
        assert_eq!(metadata.glyph_count, 3387);
    }

    #[test]
    fn validate_woff2_rejects_truncated_header() {
        let error = validate_woff2(b"wOF2").unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::InvalidFont);
        assert!(error.to_string().contains("header is truncated"));
    }

    #[test]
    fn validate_woff2_rejects_bad_declared_length() {
        let mut woff2 = encode_ttf_to_woff2(ROBOTO, &Woff2Options::default()).unwrap();
        woff2[8..12].copy_from_slice(&1u32.to_be_bytes());

        let error = validate_woff2(&woff2).unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::InvalidFont);
        assert!(error.to_string().contains("declared length"));
    }

    #[test]
    fn validate_woff2_rejects_metadata_before_compressed_data_ends() {
        let mut woff2 = encode_ttf_to_woff2(ROBOTO, &Woff2Options::default()).unwrap();

        woff2[28..32].copy_from_slice(&48u32.to_be_bytes());
        woff2[32..36].copy_from_slice(&4u32.to_be_bytes());
        woff2[36..40].copy_from_slice(&4u32.to_be_bytes());

        let error = validate_woff2(&woff2).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("WOFF2 metadata block begins before compressed data ends")
        );
    }

    #[test]
    fn validate_woff2_rejects_private_data_before_compressed_data_ends() {
        let mut woff2 = encode_ttf_to_woff2(ROBOTO, &Woff2Options::default()).unwrap();

        woff2[40..44].copy_from_slice(&48u32.to_be_bytes());
        woff2[44..48].copy_from_slice(&4u32.to_be_bytes());

        let error = validate_woff2(&woff2).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("WOFF2 private data block begins before compressed data ends")
        );
    }

    #[test]
    fn validate_woff2_rejects_metadata_overlapping_private_data() {
        let mut woff2 = encode_ttf_to_woff2(ROBOTO, &Woff2Options::default()).unwrap();
        let auxiliary_offset = u32::try_from(woff2.len()).unwrap();

        woff2.extend_from_slice(&[0; 4]);
        let declared_length = u32::try_from(woff2.len()).unwrap();

        woff2[8..12].copy_from_slice(&declared_length.to_be_bytes());
        woff2[28..32].copy_from_slice(&auxiliary_offset.to_be_bytes());
        woff2[32..36].copy_from_slice(&4u32.to_be_bytes());
        woff2[36..40].copy_from_slice(&4u32.to_be_bytes());
        woff2[40..44].copy_from_slice(&auxiliary_offset.to_be_bytes());
        woff2[44..48].copy_from_slice(&4u32.to_be_bytes());

        let error = validate_woff2(&woff2).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("WOFF2 metadata block overlaps private data")
        );
    }
}
