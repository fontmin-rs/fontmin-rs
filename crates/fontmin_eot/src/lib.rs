use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};

const EOT_VERSION_1: u32 = 0x0002_0001;
const EOT_VERSION_2: u32 = 0x0002_0002;
const EOT_MAGIC: u16 = 0x504c;
const EOT_FIXED_HEADER_SIZE: usize = 80;
const DEFAULT_WEIGHT: u32 = 400;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EotOptions {
    pub version: Option<u32>,
}

pub fn encode_ttf_to_eot(input: &[u8], options: &EotOptions) -> Result<Vec<u8>> {
    if !is_ttf(input) {
        return Err(FontminError::invalid_font(
            "expected TrueType sfnt data for EOT encoding",
        ));
    }

    let version = options.version.unwrap_or(EOT_VERSION_1);

    if !is_supported_version(version) {
        return Err(FontminError::config(format!(
            "unsupported EOT version: 0x{version:08x}",
        )));
    }

    let total_size = minimal_header_size()
        .checked_add(input.len())
        .ok_or_else(|| FontminError::invalid_font("EOT output size overflows"))?;
    let total_size_u32 = checked_u32(total_size, "EOT output size")?;
    let font_size_u32 = checked_u32(input.len(), "EOT font data size")?;
    let mut output = Vec::with_capacity(total_size);

    write_u32(&mut output, total_size_u32);
    write_u32(&mut output, font_size_u32);
    write_u32(&mut output, version);
    write_u32(&mut output, 0);
    output.extend_from_slice(&[0; 10]);
    output.push(1);
    output.push(0);
    write_u32(&mut output, DEFAULT_WEIGHT);
    write_u16(&mut output, 0);
    write_u16(&mut output, EOT_MAGIC);

    for _ in 0..4 {
        write_u32(&mut output, 0);
    }
    for _ in 0..2 {
        write_u32(&mut output, 0);
    }

    write_u32(&mut output, 0);

    for _ in 0..4 {
        write_u32(&mut output, 0);
    }

    write_empty_string(&mut output);
    write_empty_string(&mut output);
    write_empty_string(&mut output);
    write_empty_string(&mut output);
    write_u16(&mut output, 0);
    output.extend_from_slice(input);

    Ok(output)
}

pub fn decode_eot_to_ttf(input: &[u8]) -> Result<Vec<u8>> {
    if input.len() < minimal_header_size() {
        return Err(FontminError::invalid_font("EOT header is truncated"));
    }

    let declared_size = read_u32(input, 0)? as usize;
    let font_data_size = read_u32(input, 4)? as usize;
    let version = read_u32(input, 8)?;

    if declared_size != input.len() {
        return Err(FontminError::invalid_font(
            "EOT declared length does not match file length",
        ));
    }
    if !is_supported_version(version) {
        return Err(FontminError::invalid_font("EOT version is not supported"));
    }
    if read_u16(input, 34)? != EOT_MAGIC {
        return Err(FontminError::invalid_font("EOT magic number is invalid"));
    }

    let font_offset = font_data_offset(input)?;
    let font_end = font_offset
        .checked_add(font_data_size)
        .ok_or_else(|| FontminError::invalid_font("EOT font data range overflows"))?;

    if font_end != input.len() {
        return Err(FontminError::invalid_font(
            "EOT font data size does not match file length",
        ));
    }

    let output = input[font_offset..font_end].to_vec();

    if !is_ttf(&output) {
        return Err(FontminError::invalid_font(
            "expected TrueType sfnt data inside EOT",
        ));
    }

    Ok(output)
}

fn minimal_header_size() -> usize {
    EOT_FIXED_HEADER_SIZE + 18
}

fn font_data_offset(input: &[u8]) -> Result<usize> {
    let mut offset = EOT_FIXED_HEADER_SIZE;

    for _ in 0..4 {
        offset = skip_u16(input, offset)?;
        let name_size = usize::from(read_u16(input, offset)?);
        offset = skip_u16(input, offset)?;
        offset = offset
            .checked_add(name_size)
            .ok_or_else(|| FontminError::invalid_font("EOT name size overflows"))?;

        if offset > input.len() {
            return Err(FontminError::invalid_font("EOT name data is truncated"));
        }
    }

    let root_string_size = usize::from(read_u16(input, offset)?);
    offset = skip_u16(input, offset)?;
    offset = offset
        .checked_add(root_string_size)
        .ok_or_else(|| FontminError::invalid_font("EOT root string size overflows"))?;

    if offset > input.len() {
        return Err(FontminError::invalid_font(
            "EOT root string data is truncated",
        ));
    }

    Ok(offset)
}

fn write_empty_string(output: &mut Vec<u8>) {
    write_u16(output, 0);
    write_u16(output, 0);
}

fn is_ttf(input: &[u8]) -> bool {
    input.starts_with(&[0x00, 0x01, 0x00, 0x00]) || input.starts_with(b"true")
}

fn is_supported_version(version: u32) -> bool {
    version == EOT_VERSION_1 || version == EOT_VERSION_2
}

fn checked_u32(value: usize, name: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| FontminError::invalid_font(format!("{name} exceeds u32")))
}

fn write_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn read_u16(input: &[u8], offset: usize) -> Result<u16> {
    let end = offset
        .checked_add(2)
        .ok_or_else(|| FontminError::invalid_font("EOT offset overflows"))?;
    let bytes = input
        .get(offset..end)
        .ok_or_else(|| FontminError::invalid_font("EOT data is truncated"))?;

    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(input: &[u8], offset: usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| FontminError::invalid_font("EOT offset overflows"))?;
    let bytes = input
        .get(offset..end)
        .ok_or_else(|| FontminError::invalid_font("EOT data is truncated"))?;

    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn skip_u16(input: &[u8], offset: usize) -> Result<usize> {
    let next = offset
        .checked_add(2)
        .ok_or_else(|| FontminError::invalid_font("EOT offset overflows"))?;

    if next > input.len() {
        return Err(FontminError::invalid_font("EOT data is truncated"));
    }

    Ok(next)
}

#[cfg(test)]
mod tests {
    use fontmin_testing::ROBOTO;

    use super::{EotOptions, decode_eot_to_ttf, encode_ttf_to_eot};

    #[test]
    fn encodes_ttf_as_eot_lite() {
        let output = encode_ttf_to_eot(ROBOTO, &EotOptions::default()).unwrap();

        assert_eq!(
            u32::from_le_bytes(output[0..4].try_into().unwrap()) as usize,
            output.len()
        );
        assert_eq!(
            u32::from_le_bytes(output[4..8].try_into().unwrap()) as usize,
            ROBOTO.len()
        );
        assert_eq!(&output[8..12], &[0x01, 0x00, 0x02, 0x00]);
        assert_eq!(&output[34..36], &[0x4c, 0x50]);
        assert!(output.ends_with(ROBOTO));
    }

    #[test]
    fn decodes_eot_lite_to_ttf() {
        let eot = encode_ttf_to_eot(ROBOTO, &EotOptions::default()).unwrap();
        let output = decode_eot_to_ttf(&eot).unwrap();

        assert_eq!(output, ROBOTO);
    }
}
