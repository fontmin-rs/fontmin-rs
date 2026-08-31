use std::collections::HashMap;

use crate::SubsetError;

// Error type

#[derive(Debug, Clone, Copy)]
enum CffError {
    TooShort,
    InvalidIndex,
    InvalidDict,
    UnsupportedVersion,
    Unsupported(&'static str),
}

impl CffError {
    fn into_subset_error(self, flavor: &'static str) -> SubsetError {
        match self {
            Self::TooShort => SubsetError::InvalidFont(format!("{flavor} table is truncated")),
            Self::InvalidIndex => {
                SubsetError::InvalidFont(format!("{flavor} INDEX or offset is invalid"))
            }
            Self::InvalidDict => SubsetError::InvalidFont(format!("{flavor} DICT data is invalid")),
            Self::UnsupportedVersion => {
                SubsetError::InvalidFont(format!("{flavor} table version is unsupported"))
            }
            Self::Unsupported(what) => SubsetError::Unsupported(what),
        }
    }
}

// Public entry point

/// Rewrite a CFF table for a font subset.
///
/// `gid_remap` maps old GID → new GID (only entries for retained glyphs).
/// Returns a new CFF table with only the charstrings for retained glyphs.
/// Malformed or unsupported structures return a [`SubsetError`] instead of
/// silently copying charstrings that no longer match the subset GID space.
pub fn rewrite_cff(table: &[u8], gid_remap: &HashMap<u16, u16>) -> Result<Vec<u8>, SubsetError> {
    rewrite_cff_inner(table, gid_remap).map_err(|error| error.into_subset_error("CFF"))
}

// Internal structures

/// Parsed information from the CFF Top DICT.
struct TopDictInfo {
    /// Offset from start of CFF table to CharStrings INDEX.
    charstrings_offset: u32,
    /// Predefined charset selector or absolute custom charset offset.
    charset: Cff1Charset,
    /// Custom Encoding absolute offset; predefined encodings are `None`.
    encoding_offset: Option<u32>,
    /// (Private DICT length, Private DICT absolute offset in CFF).
    private: Option<(u32, u32)>,
    /// Absolute offset to a CID-keyed Font DICT INDEX.
    fdarray_offset: Option<u32>,
    /// Absolute offset to a CID-keyed FDSelect.
    fdselect_offset: Option<u32>,
    /// Whether the Top DICT declares a CID-keyed font through ROS.
    is_cid: bool,
}

#[derive(Debug, Clone, Copy)]
enum Cff1Charset {
    Predefined(u8),
    Custom(u32),
}

// INDEX parsing and building

/// Parse a CFF INDEX, returning (entries, bytes_consumed).
/// Empty INDEX (count=0) consumes 2 bytes.
fn parse_index(data: &[u8]) -> Result<(Vec<Vec<u8>>, usize), CffError> {
    if data.len() < 2 {
        return Err(CffError::TooShort);
    }
    let count = u16::from_be_bytes([data[0], data[1]]) as usize;
    if count == 0 {
        return Ok((vec![], 2));
    }
    if data.len() < 3 {
        return Err(CffError::TooShort);
    }
    let off_size = data[2] as usize;
    if off_size == 0 || off_size > 4 {
        return Err(CffError::InvalidIndex);
    }
    // Offset array: (count+1) entries, each off_size bytes.
    let offset_array_len = (count + 1) * off_size;
    let header_len = 3 + offset_array_len;
    if data.len() < header_len {
        return Err(CffError::TooShort);
    }

    let read_offset = |idx: usize| -> Result<usize, CffError> {
        let base = 3 + idx * off_size;
        let mut val = 0usize;
        for k in 0..off_size {
            val = (val << 8) | (data[base + k] as usize);
        }
        Ok(val)
    };

    let data_start = header_len;
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let start = read_offset(i)?
            .checked_sub(1)
            .ok_or(CffError::InvalidIndex)?; // 1-based → 0-based
        let end = read_offset(i + 1)?
            .checked_sub(1)
            .ok_or(CffError::InvalidIndex)?;
        if end < start {
            return Err(CffError::InvalidIndex);
        }
        let abs_start = data_start
            .checked_add(start)
            .ok_or(CffError::InvalidIndex)?;
        let abs_end = data_start.checked_add(end).ok_or(CffError::InvalidIndex)?;
        if abs_end > data.len() {
            return Err(CffError::TooShort);
        }
        entries.push(data[abs_start..abs_end].to_vec());
    }

    // Total bytes consumed = header + data (last offset - 1 = total data bytes).
    let total_data = read_offset(count)?
        .checked_sub(1)
        .ok_or(CffError::InvalidIndex)?;
    let consumed = header_len
        .checked_add(total_data)
        .ok_or(CffError::InvalidIndex)?;
    if consumed > data.len() {
        return Err(CffError::TooShort);
    }

    Ok((entries, consumed))
}

/// Build a CFF INDEX from entries.
fn build_index(entries: &[Vec<u8>]) -> Result<Vec<u8>, CffError> {
    if entries.is_empty() {
        return Ok(vec![0, 0]); // count = 0u16
    }
    let count = entries.len();
    let count_u16 = u16::try_from(count).map_err(|_| CffError::InvalidIndex)?;
    let total_data = entries.iter().try_fold(0usize, |total, entry| {
        total.checked_add(entry.len()).ok_or(CffError::InvalidIndex)
    })?;

    // Choose the minimum offSize that can represent total_data + 1.
    let max_offset = total_data.checked_add(1).ok_or(CffError::InvalidIndex)?;
    let off_size: u8 = if max_offset <= 0xFF {
        1
    } else if max_offset <= 0xFFFF {
        2
    } else if max_offset <= 0xFF_FFFF {
        3
    } else {
        4
    };

    let mut out = Vec::with_capacity(3 + (count + 1) * off_size as usize + total_data);
    out.extend_from_slice(&count_u16.to_be_bytes());
    out.push(off_size);

    let write_offset = |out: &mut Vec<u8>, off: usize| match off_size {
        1 => out.push(off as u8),
        2 => out.extend_from_slice(&(off as u16).to_be_bytes()),
        3 => {
            out.push((off >> 16) as u8);
            out.push((off >> 8) as u8);
            out.push(off as u8);
        }
        _ => out.extend_from_slice(&(off as u32).to_be_bytes()),
    };

    // Write offset array (1-based).
    let mut offset: usize = 1;
    write_offset(&mut out, offset);
    for entry in entries {
        offset = offset
            .checked_add(entry.len())
            .ok_or(CffError::InvalidIndex)?;
        write_offset(&mut out, offset);
    }

    // Write data.
    for entry in entries {
        out.extend_from_slice(entry);
    }

    Ok(out)
}

/// Parse a CFF2 INDEX. Unlike CFF1, the entry count is a four-byte Card32.
fn parse_cff2_index(data: &[u8]) -> Result<(Vec<Vec<u8>>, usize), CffError> {
    if data.len() < 4 {
        return Err(CffError::TooShort);
    }
    let count = usize::try_from(u32::from_be_bytes([data[0], data[1], data[2], data[3]]))
        .map_err(|_| CffError::InvalidIndex)?;
    if count == 0 {
        return Ok((vec![], 4));
    }
    let off_size = usize::from(*data.get(4).ok_or(CffError::TooShort)?);
    if !(1..=4).contains(&off_size) {
        return Err(CffError::InvalidIndex);
    }
    let offset_array_len = count
        .checked_add(1)
        .and_then(|value| value.checked_mul(off_size))
        .ok_or(CffError::InvalidIndex)?;
    let header_len = 5usize
        .checked_add(offset_array_len)
        .ok_or(CffError::InvalidIndex)?;
    if data.len() < header_len {
        return Err(CffError::TooShort);
    }

    let read_offset = |index: usize| -> Result<usize, CffError> {
        let base = 5 + index * off_size;
        let mut value = 0usize;
        for byte in &data[base..base + off_size] {
            value = value
                .checked_mul(256)
                .and_then(|value| value.checked_add(usize::from(*byte)))
                .ok_or(CffError::InvalidIndex)?;
        }
        Ok(value)
    };

    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let start = read_offset(index)?
            .checked_sub(1)
            .ok_or(CffError::InvalidIndex)?;
        let end = read_offset(index + 1)?
            .checked_sub(1)
            .ok_or(CffError::InvalidIndex)?;
        if end < start {
            return Err(CffError::InvalidIndex);
        }
        let absolute_start = header_len
            .checked_add(start)
            .ok_or(CffError::InvalidIndex)?;
        let absolute_end = header_len.checked_add(end).ok_or(CffError::InvalidIndex)?;
        entries.push(
            data.get(absolute_start..absolute_end)
                .ok_or(CffError::TooShort)?
                .to_vec(),
        );
    }

    let total_data = read_offset(count)?
        .checked_sub(1)
        .ok_or(CffError::InvalidIndex)?;
    let consumed = header_len
        .checked_add(total_data)
        .ok_or(CffError::InvalidIndex)?;
    if consumed > data.len() {
        return Err(CffError::TooShort);
    }

    Ok((entries, consumed))
}

/// Build a CFF2 INDEX with a four-byte Card32 count.
fn build_cff2_index(entries: &[Vec<u8>]) -> Result<Vec<u8>, CffError> {
    let count = u32::try_from(entries.len()).map_err(|_| CffError::InvalidIndex)?;
    if entries.is_empty() {
        return Ok(count.to_be_bytes().to_vec());
    }
    let total_data = entries.iter().try_fold(0usize, |total, entry| {
        total.checked_add(entry.len()).ok_or(CffError::InvalidIndex)
    })?;
    let max_offset = total_data.checked_add(1).ok_or(CffError::InvalidIndex)?;
    let off_size: u8 = if max_offset <= 0xFF {
        1
    } else if max_offset <= 0xFFFF {
        2
    } else if max_offset <= 0xFF_FFFF {
        3
    } else if u32::try_from(max_offset).is_ok() {
        4
    } else {
        return Err(CffError::InvalidIndex);
    };

    let mut output =
        Vec::with_capacity(5 + (entries.len() + 1) * usize::from(off_size) + total_data);
    output.extend_from_slice(&count.to_be_bytes());
    output.push(off_size);
    let write_offset = |output: &mut Vec<u8>, offset: usize| match off_size {
        1 => output.push(offset as u8),
        2 => output.extend_from_slice(&(offset as u16).to_be_bytes()),
        3 => {
            output.push((offset >> 16) as u8);
            output.push((offset >> 8) as u8);
            output.push(offset as u8);
        }
        _ => output.extend_from_slice(&(offset as u32).to_be_bytes()),
    };

    let mut offset = 1usize;
    write_offset(&mut output, offset);
    for entry in entries {
        offset += entry.len();
        write_offset(&mut output, offset);
    }
    for entry in entries {
        output.extend_from_slice(entry);
    }

    Ok(output)
}

// DICT parsing

/// Read one CFF DICT integer operand from `data[pos..]`.
/// Returns (value, bytes_consumed).
fn read_dict_integer(data: &[u8], pos: usize) -> Result<(i32, usize), CffError> {
    if pos >= data.len() {
        return Err(CffError::TooShort);
    }
    let b0 = data[pos];
    match b0 {
        32..=246 => Ok((b0 as i32 - 139, 1)),
        247..=250 => {
            // Positive 2-byte: value = (b0-247)*256 + b1 + 108
            if pos + 2 > data.len() {
                return Err(CffError::TooShort);
            }
            let b1 = data[pos + 1] as i32;
            Ok(((b0 as i32 - 247) * 256 + b1 + 108, 2))
        }
        251..=254 => {
            // Negative 2-byte: value = -(b0-251)*256 - b1 - 108
            if pos + 2 > data.len() {
                return Err(CffError::TooShort);
            }
            let b1 = data[pos + 1] as i32;
            Ok((-(b0 as i32 - 251) * 256 - b1 - 108, 2))
        }
        28 => {
            // 3-byte int16
            if pos + 3 > data.len() {
                return Err(CffError::TooShort);
            }
            let val = i16::from_be_bytes([data[pos + 1], data[pos + 2]]) as i32;
            Ok((val, 3))
        }
        29 => {
            // 5-byte int32
            if pos + 5 > data.len() {
                return Err(CffError::TooShort);
            }
            let val =
                i32::from_be_bytes([data[pos + 1], data[pos + 2], data[pos + 3], data[pos + 4]]);
            Ok((val, 5))
        }
        30 => {
            // Real number: skip packed BCD until 0xF nibble.
            let mut i = pos + 1;
            loop {
                if i >= data.len() {
                    return Err(CffError::TooShort);
                }
                let byte = data[i];
                i += 1;
                if (byte & 0xF0) == 0xF0 || (byte & 0x0F) == 0x0F {
                    break;
                }
            }
            // Return 0 as placeholder for reals (we don't use real values).
            Ok((0, i - pos))
        }
        _ => Err(CffError::InvalidDict),
    }
}

/// Encode an i32 as a 5-byte CFF DICT integer (prefix 29 + 4 bytes big-endian).
/// Using fixed 5-byte encoding avoids the chicken-and-egg offset-width problem.
fn encode_int32_fixed(val: i32) -> [u8; 5] {
    let bytes = val.to_be_bytes();
    [29, bytes[0], bytes[1], bytes[2], bytes[3]]
}

// Top DICT parsing

/// Parse the Top DICT bytes to extract key offsets.
fn parse_top_dict(data: &[u8]) -> Result<TopDictInfo, CffError> {
    let mut charstrings_offset: Option<u32> = None;
    let mut charset = Cff1Charset::Predefined(0);
    let mut encoding_offset: Option<u32> = None;
    let mut private_length: Option<u32> = None;
    let mut private_offset: Option<u32> = None;
    let mut fdarray_offset: Option<u32> = None;
    let mut fdselect_offset: Option<u32> = None;
    let mut has_ros = false;

    let mut pos = 0;
    // Stack of operands accumulated before each operator.
    let mut stack: Vec<i32> = Vec::with_capacity(48);

    while pos < data.len() {
        let b = data[pos];

        // Check for operator.
        match b {
            // 2-byte escape operator.
            12 => {
                if pos + 1 >= data.len() {
                    return Err(CffError::TooShort);
                }
                let op2 = data[pos + 1];
                match op2 {
                    30 => has_ros = true,
                    36 => {
                        fdarray_offset = Some(dict_offset(&stack)?);
                    }
                    37 => {
                        fdselect_offset = Some(dict_offset(&stack)?);
                    }
                    _ => {}
                }
                stack.clear();
                pos += 2;
            }
            // 1-byte operators (≤21, excluding 12 which is 2-byte escape).
            0..=21 => {
                match b {
                    15 => {
                        // charset: single integer operand (offset or predefined 0/1/2).
                        if let Some(&v) = stack.last() {
                            match v {
                                0..=2 => charset = Cff1Charset::Predefined(v as u8),
                                _ => {
                                    charset = Cff1Charset::Custom(
                                        u32::try_from(v).map_err(|_| CffError::InvalidDict)?,
                                    )
                                }
                            }
                        }
                    }
                    16 => {
                        let value = dict_offset(&stack)?;
                        if value > 1 {
                            encoding_offset = Some(value);
                        }
                    }
                    17 => {
                        // CharStrings: single integer operand (offset).
                        charstrings_offset = Some(dict_offset(&stack)?);
                    }
                    18 => {
                        // Private: [length, offset].
                        let (length, offset) = dict_private(&stack)?;
                        private_length = Some(length);
                        private_offset = Some(offset);
                    }
                    _ => {}
                }
                stack.clear();
                pos += 1;
            }
            // Operands: encoded integers or reals.
            _ => {
                let (val, consumed) = read_dict_integer(data, pos)?;
                stack.push(val);
                pos += consumed;
            }
        }
    }

    let cs_off = charstrings_offset.ok_or(CffError::InvalidDict)?;

    let private = match (private_length, private_offset) {
        (Some(len), Some(off)) => Some((len, off)),
        _ => None,
    };
    let has_cid_fields = fdarray_offset.is_some() || fdselect_offset.is_some();
    if has_ros != has_cid_fields
        || (has_cid_fields && (fdarray_offset.is_none() || fdselect_offset.is_none()))
        || (has_ros && private.is_some())
    {
        return Err(CffError::InvalidDict);
    }

    Ok(TopDictInfo {
        charstrings_offset: cs_off,
        charset,
        encoding_offset,
        private,
        fdarray_offset,
        fdselect_offset,
        is_cid: has_ros,
    })
}

fn dict_offset(stack: &[i32]) -> Result<u32, CffError> {
    stack
        .last()
        .copied()
        .ok_or(CffError::InvalidDict)
        .and_then(|value| u32::try_from(value).map_err(|_| CffError::InvalidDict))
}

fn dict_private(stack: &[i32]) -> Result<(u32, u32), CffError> {
    let length = stack
        .get(stack.len().checked_sub(2).ok_or(CffError::InvalidDict)?)
        .copied()
        .ok_or(CffError::InvalidDict)?;
    let offset = stack.last().copied().ok_or(CffError::InvalidDict)?;

    Ok((
        u32::try_from(length).map_err(|_| CffError::InvalidDict)?,
        u32::try_from(offset).map_err(|_| CffError::InvalidDict)?,
    ))
}

// Top DICT rebuilder

/// Rebuild the Top DICT bytes with fixed-width placeholders for every absolute offset.
///
/// Strategy: use fixed 5-byte int32 encoding for all rewritten operands so that
/// the Top DICT size is determined before computing downstream offsets.
///
/// The original Top DICT is scanned; operands for charset, Encoding,
/// CharStrings, Private, FDArray, and FDSelect are replaced when applicable.
/// All other bytes are copied verbatim.
///
/// Returns the rebuilt bytes and the byte positions of each placeholder so the
/// caller can patch them after computing final offsets.
fn rebuild_top_dict_with_placeholders(
    orig: &[u8],
    rewrite_charset: bool,
    patch_encoding: bool,
) -> Result<(Vec<u8>, TopDictPlaceholders), CffError> {
    let mut out: Vec<u8> = Vec::with_capacity(orig.len() + 24);
    let mut placeholders = TopDictPlaceholders::default();

    let mut pos = 0;
    let mut operand_start = 0; // start of current operand sequence

    while pos < orig.len() {
        let b = orig[pos];

        match b {
            12 => {
                let op2 = *orig.get(pos + 1).ok_or(CffError::TooShort)?;
                match op2 {
                    36 => {
                        placeholders.fdarray_patch_pos = Some(out.len());
                        out.extend_from_slice(&encode_int32_fixed(0));
                        out.extend_from_slice(&[12, 36]);
                    }
                    37 => {
                        placeholders.fdselect_patch_pos = Some(out.len());
                        out.extend_from_slice(&encode_int32_fixed(0));
                        out.extend_from_slice(&[12, 37]);
                    }
                    _ => out.extend_from_slice(&orig[operand_start..pos + 2]),
                }
                pos += 2;
                operand_start = pos;
            }
            0..=21 => {
                match b {
                    15 if rewrite_charset => {
                        // charset: replace operand(s) + operator with 5-byte int32 placeholder + op.
                        placeholders.charset_patch_pos = Some(out.len());
                        out.extend_from_slice(&encode_int32_fixed(0));
                        out.push(15);
                        pos += 1;
                        operand_start = pos;
                    }
                    16 if patch_encoding => {
                        placeholders.encoding_patch_pos = Some(out.len());
                        out.extend_from_slice(&encode_int32_fixed(0));
                        out.push(16);
                        pos += 1;
                        operand_start = pos;
                    }
                    17 => {
                        // CharStrings: replace.
                        placeholders.charstrings_patch_pos = Some(out.len());
                        out.extend_from_slice(&encode_int32_fixed(0));
                        out.push(17);
                        pos += 1;
                        operand_start = pos;
                    }
                    18 => {
                        // Private: [length, offset] operator.
                        // We must preserve Private length; only offset changes.
                        // Strategy: copy everything verbatim — Private length field stays the same
                        // since we copy Private DICT verbatim. Only Private offset needs update.
                        // Emit: length as 5-byte fixed, offset as 5-byte fixed, operator.
                        // First read the original operands.
                        let orig_slice = &orig[operand_start..pos];
                        let mut p2 = 0;
                        let mut vals: Vec<i32> = Vec::new();
                        while p2 < orig_slice.len() {
                            let (v, c) = read_dict_integer(orig_slice, p2)?;
                            vals.push(v);
                            p2 += c;
                        }
                        // Private = [length, offset].
                        let (priv_len, _) = dict_private(&vals)?;
                        let priv_len =
                            i32::try_from(priv_len).map_err(|_| CffError::InvalidDict)?;
                        out.extend_from_slice(&encode_int32_fixed(priv_len));
                        placeholders.private_patch_pos = Some(out.len());
                        out.extend_from_slice(&encode_int32_fixed(0)); // offset placeholder
                        out.push(18);
                        pos += 1;
                        operand_start = pos;
                    }
                    _ => {
                        // Other operator: copy operands + operator verbatim.
                        out.extend_from_slice(&orig[operand_start..pos + 1]);
                        pos += 1;
                        operand_start = pos;
                    }
                }
            }
            _ => {
                // Operand byte: skip (we copy in bulk when we hit the operator).
                let (_, consumed) = read_dict_integer(orig, pos)?;
                pos += consumed;
            }
        }
    }

    if rewrite_charset && placeholders.charset_patch_pos.is_none() {
        placeholders.charset_patch_pos = Some(out.len());
        out.extend_from_slice(&encode_int32_fixed(0));
        out.push(15);
    }
    if placeholders.charstrings_patch_pos.is_none() {
        return Err(CffError::InvalidDict);
    }

    Ok((out, placeholders))
}

#[derive(Default)]
struct TopDictPlaceholders {
    charstrings_patch_pos: Option<usize>,
    charset_patch_pos: Option<usize>,
    encoding_patch_pos: Option<usize>,
    private_patch_pos: Option<usize>,
    fdarray_patch_pos: Option<usize>,
    fdselect_patch_pos: Option<usize>,
}

/// Patch a 5-byte fixed int32 at `pos` in `data` with `value`.
fn patch_int32_at(data: &mut [u8], pos: usize, value: u32) {
    // 5-byte encoding: byte 29 + 4 big-endian bytes.
    let vb = (value as i32).to_be_bytes();
    data[pos] = 29;
    data[pos + 1] = vb[0];
    data[pos + 2] = vb[1];
    data[pos + 3] = vb[2];
    data[pos + 4] = vb[3];
}

// Charset parsing

/// Parse a CFF charset, returning SIDs/CIDs indexed by GID and bytes consumed.
fn parse_charset(data: &[u8], num_glyphs: usize) -> Result<(Vec<u16>, usize), CffError> {
    if num_glyphs == 0 {
        return Ok((vec![], 0));
    }
    if data.is_empty() {
        return Err(CffError::TooShort);
    }

    let mut sids = vec![0u16; num_glyphs];
    // GID 0 is always .notdef (SID 0).

    let format = data[0];
    let mut pos = 1;

    match format {
        0 => {
            // Format 0: array of SIDs (one per glyph excluding .notdef).
            for sid_slot in sids.iter_mut().skip(1) {
                if pos + 2 > data.len() {
                    return Err(CffError::TooShort);
                }
                *sid_slot = u16::from_be_bytes([data[pos], data[pos + 1]]);
                pos += 2;
            }
        }
        1 => {
            // Format 1: ranges of SIDs (u16 first, u8 nLeft).
            let mut gid = 1usize;
            while gid < num_glyphs {
                if pos + 3 > data.len() {
                    return Err(CffError::TooShort);
                }
                let first_sid = u16::from_be_bytes([data[pos], data[pos + 1]]);
                let n_left = data[pos + 2] as usize;
                pos += 3;
                let range_len = n_left.checked_add(1).ok_or(CffError::InvalidIndex)?;
                if range_len > num_glyphs - gid {
                    return Err(CffError::InvalidIndex);
                }
                for j in 0..=n_left {
                    sids[gid] = first_sid
                        .checked_add(u16::try_from(j).map_err(|_| CffError::InvalidIndex)?)
                        .ok_or(CffError::InvalidIndex)?;
                    gid += 1;
                }
            }
        }
        2 => {
            // Format 2: ranges of SIDs (u16 first, u16 nLeft).
            let mut gid = 1usize;
            while gid < num_glyphs {
                if pos + 4 > data.len() {
                    return Err(CffError::TooShort);
                }
                let first_sid = u16::from_be_bytes([data[pos], data[pos + 1]]);
                let n_left = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
                pos += 4;
                let range_len = n_left.checked_add(1).ok_or(CffError::InvalidIndex)?;
                if range_len > num_glyphs - gid {
                    return Err(CffError::InvalidIndex);
                }
                for j in 0..=n_left {
                    sids[gid] = first_sid
                        .checked_add(u16::try_from(j).map_err(|_| CffError::InvalidIndex)?)
                        .ok_or(CffError::InvalidIndex)?;
                    gid += 1;
                }
            }
        }
        _ => {
            return Err(CffError::InvalidDict);
        }
    }

    Ok((sids, pos))
}

/// Build a charset in format 0 from the given SID list (indexed by new GID, GID 0 excluded).
/// Returns the raw bytes (format byte + SIDs).
fn build_charset_format0(sids_for_non_notdef: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + sids_for_non_notdef.len() * 2);
    out.push(0u8); // format 0
    for &sid in sids_for_non_notdef {
        out.extend_from_slice(&sid.to_be_bytes());
    }
    out
}

fn predefined_charset(selector: u8, num_glyphs: usize) -> Result<Vec<u16>, CffError> {
    match selector {
        0 if num_glyphs <= 229 => (0..num_glyphs)
            .map(|gid| u16::try_from(gid).map_err(|_| CffError::InvalidIndex))
            .collect(),
        0 => Err(CffError::InvalidIndex),
        1 | 2 => Err(CffError::Unsupported(
            "dense subsetting of predefined Expert CFF charsets",
        )),
        _ => Err(CffError::InvalidDict),
    }
}

fn parse_cff1_fdselect(data: &[u8], glyph_count: usize) -> Result<(Vec<u16>, usize), CffError> {
    if data.first() == Some(&4) {
        return Err(CffError::InvalidIndex);
    }

    parse_cff2_fdselect(data, glyph_count)
}

fn build_cff1_fdselect(values: &[u16]) -> Result<Vec<u8>, CffError> {
    if values.len() > usize::from(u16::MAX)
        || values.iter().any(|value| *value > u16::from(u8::MAX))
    {
        return Err(CffError::InvalidIndex);
    }

    let output = build_cff2_fdselect(values)?;
    if output.first() != Some(&3) {
        return Err(CffError::InvalidIndex);
    }

    Ok(output)
}

struct Cff1Replacement {
    start: usize,
    old_len: usize,
    data: Vec<u8>,
}

type Cff1FdArraySource = (usize, usize, Vec<Vec<u8>>, Vec<usize>, Vec<u8>);

fn translate_cff1_offset(
    old_offset: u32,
    replacements: &[Cff1Replacement],
) -> Result<u32, CffError> {
    let old_offset_usize = usize::try_from(old_offset).map_err(|_| CffError::InvalidIndex)?;
    let mut translated = i64::from(old_offset);

    for replacement in replacements {
        if replacement.start < old_offset_usize {
            translated += replacement.data.len() as i64 - replacement.old_len as i64;
        }
    }

    u32::try_from(translated).map_err(|_| CffError::InvalidIndex)
}

fn validate_cff1_replacements(
    replacements: &[Cff1Replacement],
    table_len: usize,
) -> Result<(), CffError> {
    let mut previous_end = 0;
    for replacement in replacements {
        let end = replacement
            .start
            .checked_add(replacement.old_len)
            .ok_or(CffError::InvalidIndex)?;
        if replacement.start < previous_end || end > table_len {
            return Err(CffError::InvalidIndex);
        }
        previous_end = end;
    }

    Ok(())
}

fn rebuild_cff1_fdarray(
    font_dicts: &[Vec<u8>],
    selected_old_fds: &[usize],
    translate_offset: &impl Fn(u32) -> Result<u32, CffError>,
) -> Result<Vec<u8>, CffError> {
    let rewritten = selected_old_fds
        .iter()
        .map(|old_fd| {
            font_dicts
                .get(*old_fd)
                .ok_or(CffError::InvalidIndex)
                .and_then(|font_dict| patch_font_dict_private_offset(font_dict, translate_offset))
        })
        .collect::<Result<Vec<_>, _>>()?;

    build_index(&rewritten)
}

fn validate_private_location(table: &[u8], length: u32, offset: u32) -> Result<(), CffError> {
    let start = usize::try_from(offset).map_err(|_| CffError::InvalidIndex)?;
    let end = start
        .checked_add(usize::try_from(length).map_err(|_| CffError::InvalidIndex)?)
        .ok_or(CffError::InvalidIndex)?;
    if end > table.len() {
        return Err(CffError::TooShort);
    }

    Ok(())
}

fn validate_font_dict_private(table: &[u8], font_dict: &[u8]) -> Result<(), CffError> {
    let mut pos = 0;
    let mut stack = Vec::with_capacity(8);
    while pos < font_dict.len() {
        match font_dict[pos] {
            12 => {
                pos = pos.checked_add(2).ok_or(CffError::InvalidDict)?;
                if pos > font_dict.len() {
                    return Err(CffError::TooShort);
                }
                stack.clear();
            }
            operator @ 0..=21 => {
                if operator == 18 {
                    let (length, offset) = dict_private(&stack)?;
                    validate_private_location(table, length, offset)?;
                }
                pos += 1;
                stack.clear();
            }
            _ => {
                let (value, consumed) = read_dict_integer(font_dict, pos)?;
                stack.push(value);
                pos += consumed;
            }
        }
    }

    Ok(())
}

// Main inner function

fn rewrite_cff_inner(table: &[u8], gid_remap: &HashMap<u16, u16>) -> Result<Vec<u8>, CffError> {
    if table.len() < 4 {
        return Err(CffError::TooShort);
    }
    let major = table[0];
    let hdr_size = table[2] as usize;

    if major != 1 {
        return Err(CffError::UnsupportedVersion);
    }
    if hdr_size < 4 || hdr_size > table.len() {
        return Err(CffError::TooShort);
    }

    let mut pos = hdr_size;

    let (_, name_consumed) = parse_index(&table[pos..])?;
    pos = pos
        .checked_add(name_consumed)
        .ok_or(CffError::InvalidIndex)?;

    let top_dict_index_start = pos;
    let (top_dict_entries, top_dict_consumed) = parse_index(&table[pos..])?;
    pos = pos
        .checked_add(top_dict_consumed)
        .ok_or(CffError::InvalidIndex)?;
    if top_dict_entries.len() != 1 {
        return Err(CffError::InvalidDict);
    }
    let top_dict_raw = &top_dict_entries[0];
    let top_dict_info = parse_top_dict(top_dict_raw)?;

    let (_, string_consumed) = parse_index(&table[pos..])?;
    pos = pos
        .checked_add(string_consumed)
        .ok_or(CffError::InvalidIndex)?;

    let (_, global_subr_consumed) = parse_index(&table[pos..])?;
    let body_start = pos
        .checked_add(global_subr_consumed)
        .ok_or(CffError::InvalidIndex)?;

    let cs_off =
        usize::try_from(top_dict_info.charstrings_offset).map_err(|_| CffError::InvalidIndex)?;
    if cs_off < body_start || cs_off.checked_add(2).is_none_or(|end| end > table.len()) {
        return Err(CffError::TooShort);
    }
    let (charstrings_entries, old_charstrings_len) = parse_index(&table[cs_off..])?;
    let num_glyphs = charstrings_entries.len();

    let mut rev_remap: Vec<Option<usize>> = Vec::new();
    for (&old_gid, &new_gid) in gid_remap {
        let new_idx = new_gid as usize;
        if new_idx >= rev_remap.len() {
            rev_remap.resize(new_idx + 1, None);
        }
        rev_remap[new_idx] = Some(old_gid as usize);
    }
    if rev_remap.is_empty() || rev_remap.len() > usize::from(u16::MAX) {
        return Err(CffError::InvalidIndex);
    }
    let new_glyph_count = rev_remap.len();
    let identity_gid_space = rev_remap
        .iter()
        .enumerate()
        .all(|(gid, old_gid)| *old_gid == Some(gid));

    let mut new_charstrings: Vec<Vec<u8>> = Vec::with_capacity(new_glyph_count);
    for slot in &rev_remap {
        match slot {
            Some(old_gid) if *old_gid < num_glyphs => {
                new_charstrings.push(charstrings_entries[*old_gid].clone());
            }
            Some(_) => return Err(CffError::InvalidIndex),
            None => new_charstrings.push(vec![0x0E]),
        }
    }
    let new_charstrings_index = build_index(&new_charstrings)?;

    if top_dict_info.encoding_offset.is_some()
        && (!identity_gid_space || new_glyph_count != num_glyphs)
    {
        return Err(CffError::Unsupported(
            "subsetting fonts with a custom CFF Encoding",
        ));
    }

    if top_dict_info.is_cid && matches!(top_dict_info.charset, Cff1Charset::Predefined(_)) {
        return Err(CffError::InvalidDict);
    }

    let (new_charset, old_charset_range) = match top_dict_info.charset {
        Cff1Charset::Custom(offset) => {
            let start = usize::try_from(offset).map_err(|_| CffError::InvalidIndex)?;
            if start < body_start {
                return Err(CffError::InvalidIndex);
            }
            let (old_values, old_len) =
                parse_charset(table.get(start..).ok_or(CffError::TooShort)?, num_glyphs)?;
            let new_values = rev_remap
                .iter()
                .skip(1)
                .map(|old_gid| {
                    old_gid
                        .and_then(|old_gid| old_values.get(old_gid).copied())
                        .ok_or(CffError::InvalidIndex)
                })
                .collect::<Result<Vec<_>, _>>()?;

            (
                Some(build_charset_format0(&new_values)),
                Some((start, old_len)),
            )
        }
        Cff1Charset::Predefined(_) if identity_gid_space => (None, None),
        Cff1Charset::Predefined(selector) => {
            let old_values = predefined_charset(selector, num_glyphs)?;
            let new_values = rev_remap
                .iter()
                .skip(1)
                .map(|old_gid| {
                    old_gid
                        .and_then(|old_gid| old_values.get(old_gid).copied())
                        .ok_or(CffError::InvalidIndex)
                })
                .collect::<Result<Vec<_>, _>>()?;

            (Some(build_charset_format0(&new_values)), None)
        }
    };
    let rewrite_charset = new_charset.is_some();

    if let Some((length, offset)) = top_dict_info.private {
        validate_private_location(table, length, offset)?;
    }

    let mut fdarray_source: Option<Cff1FdArraySource> = None;
    let mut fdselect_replacement = None;
    if top_dict_info.is_cid {
        let fdarray_start =
            usize::try_from(top_dict_info.fdarray_offset.ok_or(CffError::InvalidDict)?)
                .map_err(|_| CffError::InvalidIndex)?;
        let (font_dicts, fdarray_old_len) =
            parse_index(table.get(fdarray_start..).ok_or(CffError::TooShort)?)?;
        if fdarray_start < body_start || font_dicts.is_empty() {
            return Err(CffError::InvalidIndex);
        }
        for font_dict in &font_dicts {
            validate_font_dict_private(table, font_dict)?;
        }

        let fdselect_start =
            usize::try_from(top_dict_info.fdselect_offset.ok_or(CffError::InvalidDict)?)
                .map_err(|_| CffError::InvalidIndex)?;
        let (old_fd_values, fdselect_old_len) = parse_cff1_fdselect(
            table.get(fdselect_start..).ok_or(CffError::TooShort)?,
            num_glyphs,
        )?;
        if fdselect_start < body_start {
            return Err(CffError::InvalidIndex);
        }

        let default_fd = old_fd_values
            .first()
            .copied()
            .ok_or(CffError::InvalidIndex)?;
        let mut old_to_new_fd = HashMap::<u16, u16>::new();
        let mut selected_old_fds = Vec::new();
        let new_fd_values = rev_remap
            .iter()
            .map(|old_gid| {
                let old_fd = old_gid
                    .and_then(|old_gid| old_fd_values.get(old_gid).copied())
                    .unwrap_or(default_fd);
                if usize::from(old_fd) >= font_dicts.len() {
                    return Err(CffError::InvalidIndex);
                }
                if let Some(new_fd) = old_to_new_fd.get(&old_fd) {
                    return Ok(*new_fd);
                }
                let new_fd =
                    u16::try_from(selected_old_fds.len()).map_err(|_| CffError::InvalidIndex)?;
                old_to_new_fd.insert(old_fd, new_fd);
                selected_old_fds.push(usize::from(old_fd));

                Ok(new_fd)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let provisional_fdarray =
            rebuild_cff1_fdarray(&font_dicts, &selected_old_fds, &|offset| Ok(offset))?;
        fdarray_source = Some((
            fdarray_start,
            fdarray_old_len,
            font_dicts,
            selected_old_fds,
            provisional_fdarray,
        ));
        fdselect_replacement = Some(Cff1Replacement {
            start: fdselect_start,
            old_len: fdselect_old_len,
            data: build_cff1_fdselect(&new_fd_values)?,
        });
    }

    let (mut rebuilt_top_dict, placeholders) = rebuild_top_dict_with_placeholders(
        top_dict_raw,
        rewrite_charset,
        top_dict_info.encoding_offset.is_some(),
    )?;
    let provisional_top_dict_index = build_index(&[rebuilt_top_dict.clone()])?;
    let mut replacements = vec![
        Cff1Replacement {
            start: top_dict_index_start,
            old_len: top_dict_consumed,
            data: provisional_top_dict_index,
        },
        Cff1Replacement {
            start: cs_off,
            old_len: old_charstrings_len,
            data: new_charstrings_index,
        },
    ];
    let mut appended_charset = false;
    if let Some(new_charset) = new_charset {
        if let Some((start, old_len)) = old_charset_range {
            replacements.push(Cff1Replacement {
                start,
                old_len,
                data: new_charset,
            });
        } else {
            appended_charset = true;
            replacements.push(Cff1Replacement {
                start: table.len(),
                old_len: 0,
                data: new_charset,
            });
        }
    }
    if let Some(replacement) = fdselect_replacement {
        replacements.push(replacement);
    }
    if let Some((start, old_len, _, _, provisional)) = &fdarray_source {
        replacements.push(Cff1Replacement {
            start: *start,
            old_len: *old_len,
            data: provisional.clone(),
        });
    }
    replacements.sort_by_key(|replacement| replacement.start);
    validate_cff1_replacements(&replacements, table.len())?;

    if let Some((start, _, font_dicts, selected_old_fds, provisional)) = fdarray_source {
        let relocated = rebuild_cff1_fdarray(&font_dicts, &selected_old_fds, &|offset| {
            translate_cff1_offset(offset, &replacements)
        })?;
        if relocated.len() != provisional.len() {
            return Err(CffError::InvalidIndex);
        }
        replacements
            .iter_mut()
            .find(|replacement| replacement.start == start)
            .ok_or(CffError::InvalidIndex)?
            .data = relocated;
    }

    let charstrings_patch_pos = placeholders
        .charstrings_patch_pos
        .ok_or(CffError::InvalidDict)?;
    patch_int32_at(
        &mut rebuilt_top_dict,
        charstrings_patch_pos,
        translate_cff1_offset(top_dict_info.charstrings_offset, &replacements)?,
    );
    if let Some(patch_pos) = placeholders.charset_patch_pos {
        let old_offset = match (top_dict_info.charset, appended_charset) {
            (_, true) => u32::try_from(table.len()).map_err(|_| CffError::InvalidIndex)?,
            (Cff1Charset::Custom(offset), false) => offset,
            (Cff1Charset::Predefined(_), false) => return Err(CffError::InvalidDict),
        };
        patch_int32_at(
            &mut rebuilt_top_dict,
            patch_pos,
            translate_cff1_offset(old_offset, &replacements)?,
        );
    }
    if let (Some(patch_pos), Some(old_offset)) = (
        placeholders.encoding_patch_pos,
        top_dict_info.encoding_offset,
    ) {
        patch_int32_at(
            &mut rebuilt_top_dict,
            patch_pos,
            translate_cff1_offset(old_offset, &replacements)?,
        );
    }
    if let (Some(patch_pos), Some((_, old_offset))) =
        (placeholders.private_patch_pos, top_dict_info.private)
    {
        patch_int32_at(
            &mut rebuilt_top_dict,
            patch_pos,
            translate_cff1_offset(old_offset, &replacements)?,
        );
    }
    if let (Some(patch_pos), Some(old_offset)) =
        (placeholders.fdarray_patch_pos, top_dict_info.fdarray_offset)
    {
        patch_int32_at(
            &mut rebuilt_top_dict,
            patch_pos,
            translate_cff1_offset(old_offset, &replacements)?,
        );
    }
    if let (Some(patch_pos), Some(old_offset)) = (
        placeholders.fdselect_patch_pos,
        top_dict_info.fdselect_offset,
    ) {
        patch_int32_at(
            &mut rebuilt_top_dict,
            patch_pos,
            translate_cff1_offset(old_offset, &replacements)?,
        );
    }
    let patched_top_dict_index = build_index(&[rebuilt_top_dict])?;
    let top_replacement = replacements
        .iter_mut()
        .find(|replacement| replacement.start == top_dict_index_start)
        .ok_or(CffError::InvalidIndex)?;
    if patched_top_dict_index.len() != top_replacement.data.len() {
        return Err(CffError::InvalidIndex);
    }
    top_replacement.data = patched_top_dict_index;

    let replacement_delta = replacements
        .iter()
        .map(|replacement| replacement.data.len() as i64 - replacement.old_len as i64)
        .sum::<i64>();
    let total = usize::try_from(table.len() as i64 + replacement_delta)
        .map_err(|_| CffError::InvalidIndex)?;
    let mut out = Vec::with_capacity(total);
    let mut cursor = 0;
    for replacement in replacements {
        out.extend_from_slice(&table[cursor..replacement.start]);
        out.extend_from_slice(&replacement.data);
        cursor = replacement
            .start
            .checked_add(replacement.old_len)
            .ok_or(CffError::InvalidIndex)?;
    }
    out.extend_from_slice(&table[cursor..]);

    Ok(out)
}

// ===========================================================================
// CFF2 subsetting
// ===========================================================================
//
// The first three CFF2 structures have fixed ordering:
//   Header (5 bytes)
//   Top DICT DATA (topDictLength bytes)
//   Global Subr INDEX
//
// The remaining structures may appear in any order and are reached by absolute
// offsets. The rewriter keeps that source order, replaces CharStrings,
// FDSelect, and FDArray ranges in place, then translates every Top DICT and
// Private DICT offset by the size deltas of preceding replacements. FDSelect is
// expanded and rebuilt for the new GID order instead of being copied verbatim.
//
// Safety: parse errors and unsupported structures are propagated to callers.

// CFF2 Top DICT parsing

/// Parsed information from a CFF2 Top DICT.
struct Cff2TopDictInfo {
    /// Absolute offset from start of CFF2 table to CharStrings INDEX.
    charstrings_offset: u32,
    /// Absolute offset from start of CFF2 table to FDArray INDEX (mandatory in CFF2).
    fdarray_offset: Option<u32>,
    /// Absolute offset from start of CFF2 table to FDSelect (optional).
    fdselect_offset: Option<u32>,
    /// Absolute offset from start of CFF2 table to ItemVariationStore (optional).
    vstore_offset: Option<u32>,
}

/// Parse CFF2 Top DICT bytes (not wrapped in an INDEX — raw bytes).
///
/// CFF2 Top DICT uses the same DICT encoding as CFF1 but with different
/// operator semantics:
///   op 17     = CharStrings offset
///   op 24     = vstore (ItemVariationStore) offset  ← 1-byte op in CFF2
///   op 12 36  = FDArray offset
///   op 12 37  = FDSelect offset
fn parse_cff2_top_dict(data: &[u8]) -> Result<Cff2TopDictInfo, CffError> {
    let mut charstrings_offset: Option<u32> = None;
    let mut fdarray_offset: Option<u32> = None;
    let mut fdselect_offset: Option<u32> = None;
    let mut vstore_offset: Option<u32> = None;

    let mut pos = 0;
    let mut stack: Vec<i32> = Vec::with_capacity(16);

    while pos < data.len() {
        let b = data[pos];

        match b {
            // 2-byte escape operator (12 + next byte).
            12 => {
                if pos + 1 >= data.len() {
                    return Err(CffError::TooShort);
                }
                let op2 = data[pos + 1];
                match op2 {
                    36 => {
                        // FDArray: top stack value is offset.
                        if let Some(&v) = stack.last() {
                            fdarray_offset = Some(v as u32);
                        }
                    }
                    37 => {
                        // FDSelect: top stack value is offset.
                        if let Some(&v) = stack.last() {
                            fdselect_offset = Some(v as u32);
                        }
                    }
                    _ => {}
                }
                stack.clear();
                pos += 2;
            }
            // In CFF2, op 24 is vstore (1-byte operator, not an operand prefix).
            24 => {
                if let Some(&v) = stack.last() {
                    vstore_offset = Some(v as u32);
                }
                stack.clear();
                pos += 1;
            }
            // 1-byte operators 0..=21 (22-23 are reserved in CFF2).
            0..=21 => {
                if b == 17 {
                    // CharStrings offset.
                    if let Some(&v) = stack.last() {
                        charstrings_offset = Some(v as u32);
                    }
                }
                stack.clear();
                pos += 1;
            }
            // Operands.
            _ => {
                let (val, consumed) = read_dict_integer(data, pos)?;
                stack.push(val);
                pos += consumed;
            }
        }
    }

    let cs_off = charstrings_offset.ok_or(CffError::InvalidDict)?;

    Ok(Cff2TopDictInfo {
        charstrings_offset: cs_off,
        fdarray_offset,
        fdselect_offset,
        vstore_offset,
    })
}

// CFF2 Top DICT rebuilder

/// Positions of the placeholder 5-byte int32 fields in the rebuilt Top DICT.
struct Cff2TopDictPlaceholders {
    charstrings_patch_pos: usize,
    fdarray_patch_pos: Option<usize>,
    fdselect_patch_pos: Option<usize>,
    vstore_patch_pos: Option<usize>,
}

/// Rebuild a CFF2 Top DICT with fixed 5-byte int32 encoding for all offset
/// operators (CharStrings=17, FDArray=12/36, vstore=24). Other operators are
/// copied verbatim. Returns the rebuilt bytes and placeholder positions for
/// later patching.
fn rebuild_cff2_top_dict(orig: &[u8]) -> Result<(Vec<u8>, Cff2TopDictPlaceholders), CffError> {
    let mut out: Vec<u8> = Vec::with_capacity(orig.len() + 24);
    let mut charstrings_patch_pos: Option<usize> = None;
    let mut fdarray_patch_pos: Option<usize> = None;
    let mut fdselect_patch_pos: Option<usize> = None;
    let mut vstore_patch_pos: Option<usize> = None;

    let mut pos = 0;
    let mut operand_start = 0;

    while pos < orig.len() {
        let b = orig[pos];

        match b {
            12 => {
                if pos + 1 >= orig.len() {
                    return Err(CffError::TooShort);
                }
                let op2 = orig[pos + 1];
                match op2 {
                    36 => {
                        // FDArray: replace operand(s) + operator.
                        fdarray_patch_pos = Some(out.len());
                        out.extend_from_slice(&encode_int32_fixed(0));
                        out.push(12);
                        out.push(36);
                        pos += 2;
                        operand_start = pos;
                    }
                    37 => {
                        // FDSelect: replace operand(s) + operator.
                        fdselect_patch_pos = Some(out.len());
                        out.extend_from_slice(&encode_int32_fixed(0));
                        out.push(12);
                        out.push(37);
                        pos += 2;
                        operand_start = pos;
                    }
                    _ => {
                        // Other escape: copy verbatim.
                        out.extend_from_slice(&orig[operand_start..pos + 2]);
                        pos += 2;
                        operand_start = pos;
                    }
                }
            }
            24 => {
                // vstore: replace operand(s) + operator.
                vstore_patch_pos = Some(out.len());
                out.extend_from_slice(&encode_int32_fixed(0));
                out.push(24);
                pos += 1;
                operand_start = pos;
            }
            0..=21 => {
                if b == 17 {
                    // CharStrings: replace operand(s) + operator.
                    charstrings_patch_pos = Some(out.len());
                    out.extend_from_slice(&encode_int32_fixed(0));
                    out.push(17);
                    pos += 1;
                    operand_start = pos;
                } else {
                    // Other operator: copy operands + operator verbatim.
                    out.extend_from_slice(&orig[operand_start..pos + 1]);
                    pos += 1;
                    operand_start = pos;
                }
            }
            _ => {
                // Operand byte: advance (we copy in bulk when we hit the operator).
                let (_, consumed) = read_dict_integer(orig, pos)?;
                pos += consumed;
            }
        }
    }

    let cs_patch = charstrings_patch_pos.ok_or(CffError::InvalidDict)?;

    Ok((
        out,
        Cff2TopDictPlaceholders {
            charstrings_patch_pos: cs_patch,
            fdarray_patch_pos,
            fdselect_patch_pos,
            vstore_patch_pos,
        },
    ))
}

// CFF2 FDSelect parsing and rebuilding

/// Expand an FDSelect format 0, 3, or 4 into one Font DICT index per glyph.
/// Returns the expanded mapping and the number of source bytes consumed.
fn parse_cff2_fdselect(data: &[u8], glyph_count: usize) -> Result<(Vec<u16>, usize), CffError> {
    let format = *data.first().ok_or(CffError::TooShort)?;

    match format {
        0 => {
            let end = 1usize
                .checked_add(glyph_count)
                .ok_or(CffError::InvalidIndex)?;
            let values = data.get(1..end).ok_or(CffError::TooShort)?;

            Ok((values.iter().copied().map(u16::from).collect(), end))
        }
        3 => {
            let range_count = usize::from(read_u16(data, 1)?);
            let ranges_end = 3usize
                .checked_add(range_count.checked_mul(3).ok_or(CffError::InvalidIndex)?)
                .ok_or(CffError::InvalidIndex)?;
            let consumed = ranges_end.checked_add(2).ok_or(CffError::InvalidIndex)?;
            let sentinel = usize::from(read_u16(data, ranges_end)?);
            if range_count == 0 || sentinel != glyph_count || consumed > data.len() {
                return Err(CffError::InvalidIndex);
            }

            let ranges = (0..range_count)
                .map(|index| {
                    let offset = 3 + index * 3;
                    Ok((
                        usize::from(read_u16(data, offset)?),
                        u16::from(*data.get(offset + 2).ok_or(CffError::TooShort)?),
                    ))
                })
                .collect::<Result<Vec<_>, CffError>>()?;
            let values = expand_fdselect_ranges(&ranges, glyph_count)?;

            Ok((values, consumed))
        }
        4 => {
            let range_count =
                usize::try_from(read_u32(data, 1)?).map_err(|_| CffError::InvalidIndex)?;
            let ranges_end = 5usize
                .checked_add(range_count.checked_mul(6).ok_or(CffError::InvalidIndex)?)
                .ok_or(CffError::InvalidIndex)?;
            let consumed = ranges_end.checked_add(4).ok_or(CffError::InvalidIndex)?;
            let sentinel =
                usize::try_from(read_u32(data, ranges_end)?).map_err(|_| CffError::InvalidIndex)?;
            if range_count == 0 || sentinel != glyph_count || consumed > data.len() {
                return Err(CffError::InvalidIndex);
            }

            let ranges = (0..range_count)
                .map(|index| {
                    let offset = 5 + index * 6;
                    Ok((
                        usize::try_from(read_u32(data, offset)?)
                            .map_err(|_| CffError::InvalidIndex)?,
                        read_u16(data, offset + 4)?,
                    ))
                })
                .collect::<Result<Vec<_>, CffError>>()?;
            let values = expand_fdselect_ranges(&ranges, glyph_count)?;

            Ok((values, consumed))
        }
        _ => Err(CffError::InvalidIndex),
    }
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, CffError> {
    let end = offset.checked_add(2).ok_or(CffError::InvalidIndex)?;
    let bytes = data.get(offset..end).ok_or(CffError::TooShort)?;

    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, CffError> {
    let end = offset.checked_add(4).ok_or(CffError::InvalidIndex)?;
    let bytes = data.get(offset..end).ok_or(CffError::TooShort)?;

    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn expand_fdselect_ranges(
    ranges: &[(usize, u16)],
    glyph_count: usize,
) -> Result<Vec<u16>, CffError> {
    if ranges.first().map(|range| range.0) != Some(0)
        || !ranges.windows(2).all(|pair| pair[0].0 < pair[1].0)
        || ranges.iter().any(|range| range.0 >= glyph_count)
    {
        return Err(CffError::InvalidIndex);
    }

    let mut values = vec![0; glyph_count];
    for (index, &(start, fd)) in ranges.iter().enumerate() {
        let end = ranges.get(index + 1).map_or(glyph_count, |range| range.0);
        values[start..end].fill(fd);
    }

    Ok(values)
}

/// Rebuild a compact, canonical FDSelect for the retained glyph order.
fn build_cff2_fdselect(values: &[u16]) -> Result<Vec<u8>, CffError> {
    if values.is_empty() {
        return Err(CffError::InvalidIndex);
    }

    let ranges = values
        .iter()
        .copied()
        .enumerate()
        .filter(|(index, value)| *index == 0 || values[*index - 1] != *value)
        .collect::<Vec<_>>();
    if values.len() <= usize::from(u16::MAX)
        && ranges.len() <= usize::from(u16::MAX)
        && values.iter().all(|value| *value <= u16::from(u8::MAX))
    {
        let mut output = Vec::with_capacity(5 + ranges.len() * 3);
        output.push(3);
        output.extend_from_slice(&(ranges.len() as u16).to_be_bytes());
        for (first, fd) in ranges {
            output.extend_from_slice(&(first as u16).to_be_bytes());
            output.push(fd as u8);
        }
        output.extend_from_slice(&(values.len() as u16).to_be_bytes());

        return Ok(output);
    }

    let range_count = u32::try_from(ranges.len()).map_err(|_| CffError::InvalidIndex)?;
    let sentinel = u32::try_from(values.len()).map_err(|_| CffError::InvalidIndex)?;
    let mut output = Vec::with_capacity(9 + ranges.len() * 6);
    output.push(4);
    output.extend_from_slice(&range_count.to_be_bytes());
    for (first, fd) in ranges {
        output.extend_from_slice(
            &u32::try_from(first)
                .map_err(|_| CffError::InvalidIndex)?
                .to_be_bytes(),
        );
        output.extend_from_slice(&fd.to_be_bytes());
    }
    output.extend_from_slice(&sentinel.to_be_bytes());

    Ok(output)
}

// CFF2 FDArray Private DICT relocation (two-pass aware)

/// Walk a CFF2 FDArray INDEX and patch each Font DICT's Private DICT absolute
/// offset (operator 18: [length, offset]) through `translate_offset`.
fn relocate_fdarray_privates(
    fdarray_bytes: &[u8],
    translate_offset: &impl Fn(u32) -> Result<u32, CffError>,
) -> Result<Vec<u8>, CffError> {
    let (font_dicts, _) = parse_cff2_index(fdarray_bytes)?;

    let new_font_dicts: Vec<Vec<u8>> = font_dicts
        .iter()
        .map(|fd| patch_font_dict_private_offset(fd, translate_offset))
        .collect::<Result<_, _>>()?;

    build_cff2_index(&new_font_dicts)
}

/// Rebuild one Font DICT, patching the op-18 Private DICT absolute offset.
fn patch_font_dict_private_offset(
    fd_bytes: &[u8],
    translate_offset: &impl Fn(u32) -> Result<u32, CffError>,
) -> Result<Vec<u8>, CffError> {
    let mut out: Vec<u8> = Vec::with_capacity(fd_bytes.len() + 10);
    let mut pos = 0;
    let mut operand_start = 0;

    while pos < fd_bytes.len() {
        let b = fd_bytes[pos];

        match b {
            12 => {
                if pos + 1 >= fd_bytes.len() {
                    return Err(CffError::TooShort);
                }
                // 2-byte escape: copy verbatim.
                out.extend_from_slice(&fd_bytes[operand_start..pos + 2]);
                pos += 2;
                operand_start = pos;
            }
            24 => {
                // vstore in Font DICT (unusual): copy verbatim.
                out.extend_from_slice(&fd_bytes[operand_start..pos + 1]);
                pos += 1;
                operand_start = pos;
            }
            0..=21 => {
                if b == 18 {
                    // Private: operands are [length, offset].
                    let operand_slice = &fd_bytes[operand_start..pos];
                    let mut p2 = 0;
                    let mut vals: Vec<i32> = Vec::new();
                    while p2 < operand_slice.len() {
                        let (v, c) = read_dict_integer(operand_slice, p2)?;
                        vals.push(v);
                        p2 += c;
                    }
                    if vals.len() < 2 {
                        return Err(CffError::InvalidDict);
                    } else {
                        let priv_len = vals[vals.len() - 2];
                        let priv_off = vals[vals.len() - 1];
                        let old_offset =
                            u32::try_from(priv_off).map_err(|_| CffError::InvalidDict)?;
                        let new_off = i32::try_from(translate_offset(old_offset)?)
                            .map_err(|_| CffError::InvalidDict)?;
                        out.extend_from_slice(&encode_int32_fixed(priv_len));
                        out.extend_from_slice(&encode_int32_fixed(new_off));
                        out.push(18);
                    }
                    pos += 1;
                    operand_start = pos;
                } else {
                    out.extend_from_slice(&fd_bytes[operand_start..pos + 1]);
                    pos += 1;
                    operand_start = pos;
                }
            }
            _ => {
                let (_, consumed) = read_dict_integer(fd_bytes, pos)?;
                pos += consumed;
            }
        }
    }

    Ok(out)
}

// CFF2 main rewriter

/// Rewrite a CFF2 table for a font subset.
///
/// `gid_remap` maps old GID → new GID (only entries for retained glyphs).
/// Returns a new CFF2 table with only the charstrings for retained GIDs.
///
/// # CFF2 vs CFF1 key differences
///
/// - 5-byte header: `majorVersion(u8=2) | minorVersion(u8) | headerSize(u8) | topDictLength(u16)`
/// - Top DICT is raw bytes (not wrapped in an INDEX)
/// - No charset (GIDs are always sequential: GID 0 = .notdef)
/// - No Encoding table
/// - Operator 24 = `vstore` (ItemVariationStore offset) — 1-byte op in CFF2
/// - FDArray (op 12/36) is mandatory; multi-FD fonts add FDSelect (op 12/37)
/// - Charstrings have no `endchar` terminator; end-of-data terminates each charstring
///
/// # Offset relocation strategy
///
/// Replacements retain the source table's arbitrary subtable order. Each old
/// absolute offset is translated by the Top DICT size delta plus only the
/// rewritten ranges that precede that offset. A provisional fixed-width
/// FDArray determines its final size before Private DICT offsets are patched.
pub fn rewrite_cff2(table: &[u8], gid_remap: &HashMap<u16, u16>) -> Result<Vec<u8>, SubsetError> {
    rewrite_cff2_inner(table, gid_remap).map_err(|error| error.into_subset_error("CFF2"))
}

struct Cff2Replacement {
    start: usize,
    old_len: usize,
    data: Vec<u8>,
}

fn translate_cff2_offset(
    old_offset: u32,
    top_dict_delta: i64,
    replacements: &[Cff2Replacement],
) -> Result<u32, CffError> {
    let old_offset_usize = usize::try_from(old_offset).map_err(|_| CffError::InvalidIndex)?;
    let mut translated = i64::from(old_offset) + top_dict_delta;

    for replacement in replacements {
        if replacement.start < old_offset_usize {
            translated += replacement.data.len() as i64 - replacement.old_len as i64;
        }
    }

    u32::try_from(translated).map_err(|_| CffError::InvalidIndex)
}

fn validate_cff2_replacements(
    replacements: &[Cff2Replacement],
    body_start: usize,
    table_len: usize,
) -> Result<(), CffError> {
    let mut previous_end = body_start;
    for replacement in replacements {
        let end = replacement
            .start
            .checked_add(replacement.old_len)
            .ok_or(CffError::InvalidIndex)?;
        if replacement.start < previous_end || end > table_len {
            return Err(CffError::InvalidIndex);
        }
        previous_end = end;
    }

    Ok(())
}

fn rewrite_cff2_inner(table: &[u8], gid_remap: &HashMap<u16, u16>) -> Result<Vec<u8>, CffError> {
    // -----------------------------------------------------------------------
    // 1. Parse CFF2 header (5 bytes).
    // -----------------------------------------------------------------------
    if table.len() < 5 {
        return Err(CffError::TooShort);
    }
    let major = table[0];
    if major != 2 {
        return Err(CffError::UnsupportedVersion);
    }
    let hdr_size = table[2] as usize;
    let top_dict_len_orig = u16::from_be_bytes([table[3], table[4]]) as usize;

    if hdr_size < 5 || hdr_size + top_dict_len_orig > table.len() {
        return Err(CffError::TooShort);
    }

    // -----------------------------------------------------------------------
    // 2. Parse Top DICT DATA.
    // -----------------------------------------------------------------------
    let top_dict_data = &table[hdr_size..hdr_size + top_dict_len_orig];
    let top_dict_info = parse_cff2_top_dict(top_dict_data)?;

    // The Global Subr INDEX immediately follows the Top DICT. Validate it,
    // while preserving its bytes in place during reconstruction below.
    let body_start = hdr_size + top_dict_len_orig;
    parse_cff2_index(table.get(body_start..).ok_or(CffError::TooShort)?)?;

    // -----------------------------------------------------------------------
    // 3. Find and subset CharStrings INDEX.
    // -----------------------------------------------------------------------
    let cs_off_orig =
        usize::try_from(top_dict_info.charstrings_offset).map_err(|_| CffError::InvalidIndex)?;
    if cs_off_orig
        .checked_add(4)
        .is_none_or(|end| end > table.len())
    {
        return Err(CffError::TooShort);
    }
    let (charstrings_entries, old_cs_size) = parse_cff2_index(&table[cs_off_orig..])?;
    let num_glyphs = charstrings_entries.len();

    // Build reverse remap: new GID → old GID.
    let mut rev_remap: Vec<Option<usize>> = Vec::new();
    for (&old_gid, &new_gid) in gid_remap {
        let new_idx = new_gid as usize;
        if new_idx >= rev_remap.len() {
            rev_remap.resize(new_idx + 1, None);
        }
        rev_remap[new_idx] = Some(old_gid as usize);
    }

    let mut new_charstrings: Vec<Vec<u8>> = Vec::with_capacity(rev_remap.len());
    for slot in &rev_remap {
        match slot {
            Some(old_gid) if *old_gid < num_glyphs => {
                new_charstrings.push(charstrings_entries[*old_gid].clone());
            }
            // Missing or out-of-bounds: empty charstring (CFF2 has no endchar op).
            _ => new_charstrings.push(vec![]),
        }
    }

    let new_charstrings_index = build_cff2_index(&new_charstrings)?;

    // -----------------------------------------------------------------------
    // 4. Remap FDSelect into the retained glyph order, when present.
    // -----------------------------------------------------------------------
    let fdselect_replacement = if let Some(fdselect_offset) = top_dict_info.fdselect_offset {
        let start = usize::try_from(fdselect_offset).map_err(|_| CffError::InvalidIndex)?;
        let (old_values, old_len) =
            parse_cff2_fdselect(table.get(start..).ok_or(CffError::TooShort)?, num_glyphs)?;
        let default_fd = old_values.first().copied().ok_or(CffError::InvalidIndex)?;
        let new_values = rev_remap
            .iter()
            .map(|old_gid| {
                Ok(old_gid
                    .and_then(|old_gid| old_values.get(old_gid).copied())
                    .unwrap_or(default_fd))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Some(Cff2Replacement {
            start,
            old_len,
            data: build_cff2_fdselect(&new_values)?,
        })
    } else {
        None
    };

    // -----------------------------------------------------------------------
    // 5. Rebuild offset-bearing structures once to determine their sizes.
    // -----------------------------------------------------------------------
    let (mut new_top_dict, placeholders) = rebuild_cff2_top_dict(top_dict_data)?;
    let new_top_dict_size = new_top_dict.len();
    let top_dict_delta = new_top_dict_size as i64 - top_dict_len_orig as i64;
    let fdarray_source = if let Some(fdarray_offset) = top_dict_info.fdarray_offset {
        let start = usize::try_from(fdarray_offset).map_err(|_| CffError::InvalidIndex)?;
        let source = table.get(start..).ok_or(CffError::TooShort)?;
        let (_, old_len) = parse_cff2_index(source)?;
        let source = source.get(..old_len).ok_or(CffError::TooShort)?;
        let provisional = relocate_fdarray_privates(source, &|offset| Ok(offset))?;

        Some((start, old_len, source, provisional))
    } else {
        None
    };

    let mut replacements = vec![Cff2Replacement {
        start: cs_off_orig,
        old_len: old_cs_size,
        data: new_charstrings_index,
    }];
    if let Some(replacement) = fdselect_replacement {
        replacements.push(replacement);
    }
    if let Some((start, old_len, _, provisional)) = &fdarray_source {
        replacements.push(Cff2Replacement {
            start: *start,
            old_len: *old_len,
            data: provisional.clone(),
        });
    }
    replacements.sort_by_key(|replacement| replacement.start);
    validate_cff2_replacements(&replacements, body_start, table.len())?;

    // With every replacement size known, relocate Private DICT offsets from
    // their original absolute positions. Fixed-width encoding guarantees the
    // final FDArray has the same size as the provisional one.
    if let Some((start, _, source, provisional)) = fdarray_source {
        let relocated = relocate_fdarray_privates(source, &|offset| {
            translate_cff2_offset(offset, top_dict_delta, &replacements)
        })?;
        if relocated.len() != provisional.len() {
            return Err(CffError::InvalidIndex);
        }
        replacements
            .iter_mut()
            .find(|replacement| replacement.start == start)
            .ok_or(CffError::InvalidIndex)?
            .data = relocated;
    }

    // -----------------------------------------------------------------------
    // 6. Patch every absolute Top DICT offset for the reconstructed layout.
    // -----------------------------------------------------------------------
    let new_cs_abs = translate_cff2_offset(
        top_dict_info.charstrings_offset,
        top_dict_delta,
        &replacements,
    )?;
    patch_int32_at(
        &mut new_top_dict,
        placeholders.charstrings_patch_pos,
        new_cs_abs,
    );

    if let (Some(patch_pos), Some(old_offset)) =
        (placeholders.fdarray_patch_pos, top_dict_info.fdarray_offset)
    {
        let new_offset = translate_cff2_offset(old_offset, top_dict_delta, &replacements)?;
        patch_int32_at(&mut new_top_dict, patch_pos, new_offset);
    }
    if let (Some(patch_pos), Some(old_offset)) = (
        placeholders.fdselect_patch_pos,
        top_dict_info.fdselect_offset,
    ) {
        let new_offset = translate_cff2_offset(old_offset, top_dict_delta, &replacements)?;
        patch_int32_at(&mut new_top_dict, patch_pos, new_offset);
    }
    if let (Some(patch_pos), Some(old_offset)) =
        (placeholders.vstore_patch_pos, top_dict_info.vstore_offset)
    {
        let new_offset = translate_cff2_offset(old_offset, top_dict_delta, &replacements)?;
        patch_int32_at(&mut new_top_dict, patch_pos, new_offset);
    }

    // -----------------------------------------------------------------------
    // 7. Assemble output, preserving all non-rewritten CFF2 data in place.
    // -----------------------------------------------------------------------
    let new_top_dict_len_u16: u16 = new_top_dict_size
        .try_into()
        .map_err(|_| CffError::InvalidDict)?;

    let replacement_delta = replacements
        .iter()
        .map(|replacement| replacement.data.len() as i64 - replacement.old_len as i64)
        .sum::<i64>();
    let total = usize::try_from(table.len() as i64 + top_dict_delta + replacement_delta)
        .map_err(|_| CffError::InvalidIndex)?;

    let mut out = Vec::with_capacity(total);

    // Header: copy original, then update topDictLength at bytes 3-4.
    out.extend_from_slice(&table[..hdr_size]);
    let len_be = new_top_dict_len_u16.to_be_bytes();
    out[3] = len_be[0];
    out[4] = len_be[1];

    out.extend_from_slice(&new_top_dict);
    let mut cursor = body_start;
    for replacement in replacements {
        out.extend_from_slice(&table[cursor..replacement.start]);
        out.extend_from_slice(&replacement.data);
        cursor = replacement.start + replacement.old_len;
    }
    out.extend_from_slice(&table[cursor..]);

    Ok(out)
}

#[cfg(test)]
mod tests;
