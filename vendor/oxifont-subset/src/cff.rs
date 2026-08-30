use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum CffError {
    TooShort,
    InvalidIndex,
    InvalidDict,
    UnsupportedVersion,
    CidFont, // CID-keyed CFF1 fonts — bail to verbatim
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Rewrite a CFF table for a font subset.
///
/// `gid_remap` maps old GID → new GID (only entries for retained glyphs).
/// Returns a new CFF table with only the charstrings for retained glyphs,
/// or the original table verbatim if parsing fails.
pub fn rewrite_cff(table: &[u8], gid_remap: &HashMap<u16, u16>) -> Vec<u8> {
    rewrite_cff_checked(table, gid_remap).0
}

/// As [`rewrite_cff`], also reporting whether the verbatim fallback was taken.
///
/// The fallback produces a table that is only correct under the **original**
/// glyph numbering, so a caller that is renumbering glyphs has to know. See
/// [`crate::SubsetStats::cff_charstrings_verbatim`].
pub(crate) fn rewrite_cff_checked(table: &[u8], gid_remap: &HashMap<u16, u16>) -> (Vec<u8>, bool) {
    match rewrite_cff_inner(table, gid_remap) {
        Ok(result) => (result, false),
        // On any parse error, CID detection, or unsupported structure → safe verbatim copy.
        Err(_) => (table.to_vec(), true),
    }
}

// ---------------------------------------------------------------------------
// Internal structures
// ---------------------------------------------------------------------------

/// Parsed information from the CFF Top DICT.
struct TopDictInfo {
    /// Offset from start of CFF table to CharStrings INDEX.
    charstrings_offset: u32,
    /// Charset offset; None if predefined (0, 1, 2).
    charset_offset: Option<u32>,
    /// (Private DICT length, Private DICT absolute offset in CFF).
    private: Option<(u32, u32)>,
}

// ---------------------------------------------------------------------------
// INDEX parsing and building
// ---------------------------------------------------------------------------

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
fn build_index(entries: &[Vec<u8>]) -> Vec<u8> {
    if entries.is_empty() {
        return vec![0, 0]; // count = 0u16
    }
    let count = entries.len();
    let total_data: usize = entries.iter().map(|e| e.len()).sum();

    // Choose the minimum offSize that can represent total_data + 1.
    let max_offset = total_data + 1;
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
    out.extend_from_slice(&(count as u16).to_be_bytes());
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
        offset += entry.len();
        write_offset(&mut out, offset);
    }

    // Write data.
    for entry in entries {
        out.extend_from_slice(entry);
    }

    out
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

// ---------------------------------------------------------------------------
// DICT parsing
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Top DICT parsing
// ---------------------------------------------------------------------------

/// Parse the Top DICT bytes to extract key offsets.
/// Returns `Err(CffError::CidFont)` if FDArray or FDSelect operators are present.
fn parse_top_dict(data: &[u8]) -> Result<TopDictInfo, CffError> {
    let mut charstrings_offset: Option<u32> = None;
    let mut charset_offset: Option<u32> = None; // None = predefined
    let mut private_length: Option<u32> = None;
    let mut private_offset: Option<u32> = None;

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
                    36 | 37 => {
                        // FDArray (36) or FDSelect (37) → CID font, bail.
                        return Err(CffError::CidFont);
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
                                0..=2 => charset_offset = None, // predefined
                                _ => charset_offset = Some(v as u32),
                            }
                        }
                    }
                    17 => {
                        // CharStrings: single integer operand (offset).
                        if let Some(&v) = stack.last() {
                            charstrings_offset = Some(v as u32);
                        }
                    }
                    18 => {
                        // Private: [length, offset].
                        if let (true, Some(&len_val), Some(&off_val)) = (
                            stack.len() >= 2,
                            stack.get(stack.len().wrapping_sub(2)),
                            stack.last(),
                        ) {
                            private_length = Some(len_val as u32);
                            private_offset = Some(off_val as u32);
                        }
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

    Ok(TopDictInfo {
        charstrings_offset: cs_off,
        charset_offset,
        private,
    })
}

// ---------------------------------------------------------------------------
// Top DICT rebuilder
// ---------------------------------------------------------------------------

/// Rebuild the Top DICT bytes with updated CharStrings, charset, and Private offsets.
///
/// Strategy: use fixed 5-byte int32 encoding for all rewritten operands so that
/// the Top DICT size is determined before computing downstream offsets.
///
/// The original Top DICT is scanned; operands for operators 15/17/18 are replaced
/// with fixed 5-byte placeholders (values filled in later by the caller after all
/// sizes are known). All other bytes are copied verbatim.
///
/// Returns the rebuilt bytes and the byte positions of each placeholder so the
/// caller can patch them after computing final offsets.
fn rebuild_top_dict_with_placeholders(
    orig: &[u8],
    has_charset: bool,
) -> Result<(Vec<u8>, TopDictPlaceholders), CffError> {
    let mut out: Vec<u8> = Vec::with_capacity(orig.len() + 24);
    let mut placeholders = TopDictPlaceholders::default();

    let mut pos = 0;
    let mut operand_start = 0; // start of current operand sequence

    while pos < orig.len() {
        let b = orig[pos];

        match b {
            12 => {
                // 2-byte escape: copy as-is and reset operand_start.
                out.extend_from_slice(&orig[operand_start..pos + 2]);
                pos += 2;
                operand_start = pos;
            }
            0..=21 => {
                match b {
                    15 if has_charset => {
                        // charset: replace operand(s) + operator with 5-byte int32 placeholder + op.
                        placeholders.charset_patch_pos = out.len();
                        out.extend_from_slice(&encode_int32_fixed(0));
                        out.push(15);
                        pos += 1;
                        operand_start = pos;
                    }
                    17 => {
                        // CharStrings: replace.
                        placeholders.charstrings_patch_pos = out.len();
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
                        let priv_len = if vals.len() >= 2 { vals[0] } else { 0 };
                        placeholders.private_len_value = priv_len;
                        placeholders.private_patch_pos = out.len();
                        out.extend_from_slice(&encode_int32_fixed(priv_len));
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

    Ok((out, placeholders))
}

#[derive(Default)]
struct TopDictPlaceholders {
    charstrings_patch_pos: usize,
    charset_patch_pos: usize, // 0 if not patched
    private_patch_pos: usize, // 0 if not patched; points at the 5-byte offset placeholder
    private_len_value: i32,   // preserved from original
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

// ---------------------------------------------------------------------------
// Charset parsing
// ---------------------------------------------------------------------------

/// Parse a CFF charset, returning a Vec<u16> of SIDs indexed by GID (position 0 = .notdef = SID 0).
fn parse_charset(data: &[u8], num_glyphs: usize) -> Result<Vec<u16>, CffError> {
    if num_glyphs == 0 {
        return Ok(vec![]);
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
                for j in 0..=n_left {
                    if gid >= num_glyphs {
                        break;
                    }
                    sids[gid] = first_sid.wrapping_add(j as u16);
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
                for j in 0..=n_left {
                    if gid >= num_glyphs {
                        break;
                    }
                    sids[gid] = first_sid.wrapping_add(j as u16);
                    gid += 1;
                }
            }
        }
        _ => {
            return Err(CffError::InvalidDict);
        }
    }

    Ok(sids)
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

// ---------------------------------------------------------------------------
// Private DICT length determination (for Local Subrs block inclusion)
// ---------------------------------------------------------------------------

/// Scan Private DICT bytes to find the Subrs operand (key=19), which gives the
/// Local Subrs INDEX offset *relative to Private DICT start*.
/// Returns the relative offset if found, or None.
fn find_local_subrs_offset_in_private_dict(priv_data: &[u8]) -> Option<u32> {
    let mut pos = 0;
    let mut stack: Vec<i32> = Vec::with_capacity(8);

    while pos < priv_data.len() {
        let b = priv_data[pos];

        match b {
            12 => {
                // 2-byte escape.
                pos += 2;
                stack.clear();
            }
            0..=21 => {
                if b == 19 {
                    // Subrs: operand is relative offset from Private DICT start.
                    if let Some(&v) = stack.last() {
                        return Some(v as u32);
                    }
                }
                stack.clear();
                pos += 1;
            }
            _ => {
                if let Ok((val, consumed)) = read_dict_integer(priv_data, pos) {
                    stack.push(val);
                    pos += consumed;
                } else {
                    break;
                }
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Main inner function
// ---------------------------------------------------------------------------

fn rewrite_cff_inner(table: &[u8], gid_remap: &HashMap<u16, u16>) -> Result<Vec<u8>, CffError> {
    // -----------------------------------------------------------------------
    // 1. Parse header.
    // -----------------------------------------------------------------------
    if table.len() < 4 {
        return Err(CffError::TooShort);
    }
    let major = table[0];
    let _minor = table[1];
    let hdr_size = table[2] as usize;
    let _cff_off_size = table[3];

    if major != 1 {
        return Err(CffError::UnsupportedVersion);
    }
    if hdr_size < 4 || hdr_size > table.len() {
        return Err(CffError::TooShort);
    }

    let mut pos = hdr_size;

    // -----------------------------------------------------------------------
    // 2. Parse Name INDEX (copy verbatim).
    // -----------------------------------------------------------------------
    let name_index_start = pos;
    let (_, name_consumed) = parse_index(&table[pos..])?;
    let name_index_end = pos + name_consumed;
    let name_index_bytes = table[name_index_start..name_index_end].to_vec();
    pos = name_index_end;

    // -----------------------------------------------------------------------
    // 3. Parse Top DICT INDEX (extract the one top dict entry).
    // -----------------------------------------------------------------------
    let top_dict_index_start = pos;
    let (top_dict_entries, top_dict_consumed) = parse_index(&table[pos..])?;
    pos += top_dict_consumed;

    if top_dict_entries.is_empty() {
        return Err(CffError::InvalidDict);
    }
    let top_dict_raw = &top_dict_entries[0];
    let top_dict_info = parse_top_dict(top_dict_raw)?;
    // ^ returns Err(CidFont) if FDArray/FDSelect found — propagates to verbatim fallback.

    let _ = top_dict_index_start; // consumed; we rebuild it from scratch.

    // -----------------------------------------------------------------------
    // 4. Parse String INDEX (copy verbatim).
    // -----------------------------------------------------------------------
    let string_index_start = pos;
    let (_, string_consumed) = parse_index(&table[pos..])?;
    let string_index_end = pos + string_consumed;
    let string_index_bytes = table[string_index_start..string_index_end].to_vec();
    pos = string_index_end;

    // -----------------------------------------------------------------------
    // 5. Parse Global Subr INDEX (copy verbatim).
    // -----------------------------------------------------------------------
    let global_subr_start = pos;
    let (_, global_subr_consumed) = parse_index(&table[pos..])?;
    let global_subr_end = pos + global_subr_consumed;
    let global_subr_bytes = table[global_subr_start..global_subr_end].to_vec();

    // -----------------------------------------------------------------------
    // 6. Parse CharStrings INDEX — extract per-GID charstrings.
    // -----------------------------------------------------------------------
    let cs_off =
        usize::try_from(top_dict_info.charstrings_offset).map_err(|_| CffError::InvalidIndex)?;
    if cs_off.checked_add(2).is_none_or(|end| end > table.len()) {
        return Err(CffError::TooShort);
    }
    let (charstrings_entries, _) = parse_index(&table[cs_off..])?;
    let num_glyphs = charstrings_entries.len();

    // -----------------------------------------------------------------------
    // 7. Build new CharStrings INDEX with only retained GIDs in new order.
    // -----------------------------------------------------------------------
    // Build reverse remap: new GID → old GID.
    let mut rev_remap: Vec<Option<usize>> = Vec::new();
    for (&old_gid, &new_gid) in gid_remap {
        let new_idx = new_gid as usize;
        if new_idx >= rev_remap.len() {
            rev_remap.resize(new_idx + 1, None);
        }
        rev_remap[new_idx] = Some(old_gid as usize);
    }
    let new_glyph_count = rev_remap.len();

    let mut new_charstrings: Vec<Vec<u8>> = Vec::with_capacity(new_glyph_count);
    for slot in &rev_remap {
        match slot {
            Some(old_gid) => {
                if *old_gid < num_glyphs {
                    new_charstrings.push(charstrings_entries[*old_gid].clone());
                } else {
                    // Old GID out of bounds — use empty charstring.
                    new_charstrings.push(vec![0x0E]); // endchar
                }
            }
            None => {
                // Missing slot — use empty charstring.
                new_charstrings.push(vec![0x0E]); // endchar
            }
        }
    }

    let new_charstrings_index = build_index(&new_charstrings);

    // -----------------------------------------------------------------------
    // 8. Build new charset in format 0.
    // -----------------------------------------------------------------------
    // Only build if original had a non-predefined charset.
    let new_charset_bytes: Option<Vec<u8>> = if let Some(orig_cs_off) = top_dict_info.charset_offset
    {
        let cs_off_usize = orig_cs_off as usize;
        if cs_off_usize >= table.len() {
            return Err(CffError::TooShort);
        }
        let orig_sids = parse_charset(&table[cs_off_usize..], num_glyphs)?;

        // Map new GIDs → SIDs from old charset.
        let mut new_sids: Vec<u16> = Vec::with_capacity(new_glyph_count.saturating_sub(1));
        for slot in rev_remap.iter().skip(1) {
            // Skip new GID 0 (.notdef).
            let old_gid = slot.unwrap_or(0);
            let sid = if old_gid < orig_sids.len() {
                orig_sids[old_gid]
            } else {
                0
            };
            new_sids.push(sid);
        }

        Some(build_charset_format0(&new_sids))
    } else {
        None // predefined charset — referenced by value 0/1/2 in Top DICT, no data to emit
    };

    // -----------------------------------------------------------------------
    // 9. Compute Private DICT block (verbatim slice).
    // -----------------------------------------------------------------------
    let private_block_bytes: Option<Vec<u8>> = if let Some((priv_len, priv_off)) =
        top_dict_info.private
    {
        let p_start = usize::try_from(priv_off).map_err(|_| CffError::InvalidIndex)?;
        let p_end = p_start
            .checked_add(usize::try_from(priv_len).map_err(|_| CffError::InvalidIndex)?)
            .ok_or(CffError::InvalidIndex)?;
        if p_end > table.len() {
            return Err(CffError::TooShort);
        }
        let priv_dict_data = &table[p_start..p_end];

        // Check for Local Subrs inside Private DICT.
        let mut block = priv_dict_data.to_vec();
        if let Some(local_subrs_rel_off) = find_local_subrs_offset_in_private_dict(priv_dict_data) {
            let ls_abs = p_start
                .checked_add(
                    usize::try_from(local_subrs_rel_off).map_err(|_| CffError::InvalidIndex)?,
                )
                .ok_or(CffError::InvalidIndex)?;
            if ls_abs < table.len() {
                let (_, ls_consumed) = parse_index(&table[ls_abs..])?;
                let ls_end = ls_abs
                    .checked_add(ls_consumed)
                    .ok_or(CffError::InvalidIndex)?;
                if ls_end <= table.len() {
                    // Append Local Subrs INDEX right after Private DICT.
                    block.extend_from_slice(&table[ls_abs..ls_end]);
                }
            }
        }
        Some(block)
    } else {
        None
    };

    // -----------------------------------------------------------------------
    // 10. Compute layout and build the new Top DICT with patched offsets.
    // -----------------------------------------------------------------------
    // We must know where each section will land in the output so we can fill in
    // the offset operands in the new Top DICT.
    //
    // Output layout:
    //   [0]               Header (hdr_size bytes)
    //   [hdr_size]        Name INDEX
    //   [after name]      Top DICT INDEX  (rebuilt with 5-byte placeholder operands)
    //   [after top dict]  String INDEX
    //   [after string]    Global Subr INDEX
    //   [after global]    Charset (if non-predefined)
    //   [after charset]   CharStrings INDEX
    //   [after cs]        Private DICT + Local Subrs (if present)
    //
    // The Top DICT INDEX wraps the rebuilt Top DICT bytes (one entry).
    //
    // We first build the rebuilt Top DICT (with placeholder zeros) to learn its
    // size, then compute offsets, then patch.

    let has_charset = new_charset_bytes.is_some();
    let has_private = private_block_bytes.is_some();

    let (mut rebuilt_top_dict, placeholders) =
        rebuild_top_dict_with_placeholders(top_dict_raw, has_charset)?;

    // Wrap rebuilt Top DICT in a new Top DICT INDEX.
    let new_top_dict_index = build_index(&[rebuilt_top_dict.clone()]);

    // Now compute absolute offsets of each section in the output.
    let header_bytes = &table[..hdr_size];
    let offset_after_header = hdr_size;
    let offset_after_name = offset_after_header + name_index_bytes.len();
    let offset_after_top_dict_index = offset_after_name + new_top_dict_index.len();
    let offset_after_string = offset_after_top_dict_index + string_index_bytes.len();
    let offset_after_global_subr = offset_after_string + global_subr_bytes.len();

    // Charset comes first (if any), then CharStrings.
    let (charset_abs_offset, offset_after_charset) = if let Some(ref cs_bytes) = new_charset_bytes {
        let off = offset_after_global_subr;
        (Some(off as u32), off + cs_bytes.len())
    } else {
        (None, offset_after_global_subr)
    };

    let charstrings_abs_offset = offset_after_charset as u32;
    let offset_after_charstrings = offset_after_charset + new_charstrings_index.len();

    let private_abs_offset = if has_private {
        Some(offset_after_charstrings as u32)
    } else {
        None
    };

    // -----------------------------------------------------------------------
    // 11. Patch the offsets in the rebuilt Top DICT bytes.
    // -----------------------------------------------------------------------
    // CharStrings placeholder (always present):
    patch_int32_at(
        &mut rebuilt_top_dict,
        placeholders.charstrings_patch_pos,
        charstrings_abs_offset,
    );

    // Charset placeholder (only if has_charset):
    if has_charset {
        if let Some(cs_abs) = charset_abs_offset {
            patch_int32_at(
                &mut rebuilt_top_dict,
                placeholders.charset_patch_pos,
                cs_abs,
            );
        }
    }

    // Private placeholder (only if has_private):
    if has_private {
        if let Some(priv_abs) = private_abs_offset {
            // Offset placeholder starts after the 5-byte length field.
            patch_int32_at(
                &mut rebuilt_top_dict,
                placeholders.private_patch_pos + 5,
                priv_abs,
            );
        }
    }

    // Rebuild Top DICT INDEX now that the dict bytes are patched.
    let new_top_dict_index_patched = build_index(&[rebuilt_top_dict]);

    // -----------------------------------------------------------------------
    // 12. Assemble output.
    // -----------------------------------------------------------------------
    let mut out: Vec<u8> = Vec::with_capacity(
        hdr_size
            + name_index_bytes.len()
            + new_top_dict_index_patched.len()
            + string_index_bytes.len()
            + global_subr_bytes.len()
            + new_charset_bytes.as_ref().map_or(0, |v| v.len())
            + new_charstrings_index.len()
            + private_block_bytes.as_ref().map_or(0, |v| v.len()),
    );

    out.extend_from_slice(header_bytes);
    out.extend_from_slice(&name_index_bytes);
    out.extend_from_slice(&new_top_dict_index_patched);
    out.extend_from_slice(&string_index_bytes);
    out.extend_from_slice(&global_subr_bytes);

    if let Some(cs_bytes) = &new_charset_bytes {
        out.extend_from_slice(cs_bytes);
    }

    out.extend_from_slice(&new_charstrings_index);

    if let Some(priv_bytes) = &private_block_bytes {
        out.extend_from_slice(priv_bytes);
    }

    Ok(out)
}

// ===========================================================================
// CFF2 subsetting
// ===========================================================================
//
// Implementation notes — layout awareness:
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
// Safety: verbatim fallback on any parse error or unsupported structure.

// ---------------------------------------------------------------------------
// CFF2 Top DICT parsing
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// CFF2 Top DICT rebuilder
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// CFF2 FDSelect parsing and rebuilding
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// CFF2 FDArray Private DICT relocation (two-pass aware)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// CFF2 main rewriter
// ---------------------------------------------------------------------------

/// Rewrite a CFF2 table for a font subset.
///
/// `gid_remap` maps old GID → new GID (only entries for retained glyphs).
/// Returns a new CFF2 table with only the charstrings for retained GIDs,
/// or the original table verbatim if parsing fails.
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
pub fn rewrite_cff2(table: &[u8], gid_remap: &HashMap<u16, u16>) -> Vec<u8> {
    rewrite_cff2_checked(table, gid_remap).0
}

/// As [`rewrite_cff2`], also reporting whether the verbatim fallback was taken.
pub(crate) fn rewrite_cff2_checked(table: &[u8], gid_remap: &HashMap<u16, u16>) -> (Vec<u8>, bool) {
    match rewrite_cff2_inner(table, gid_remap) {
        Ok(result) => (result, false),
        Err(_) => (table.to_vec(), true),
    }
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
        let new_values = rev_remap
            .iter()
            .map(|old_gid| {
                old_gid
                    .and_then(|old_gid| old_values.get(old_gid).copied())
                    .ok_or(CffError::InvalidIndex)
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
mod cff_structure_tests {
    use super::{
        build_cff2_fdselect, build_cff2_index, parse_cff2_fdselect, parse_cff2_index, parse_index,
    };

    #[test]
    fn cff1_index_rejects_zero_offsets() {
        assert!(parse_index(&[0, 1, 1, 0]).is_err());
    }

    #[test]
    fn cff2_index_uses_a_card32_count_and_round_trips() {
        let entries = vec![b"alpha".to_vec(), Vec::new(), b"omega".to_vec()];
        let encoded = build_cff2_index(&entries).unwrap();
        let (decoded, consumed) = parse_cff2_index(&encoded).unwrap();

        assert_eq!(&encoded[..4], &3u32.to_be_bytes());
        assert_eq!(decoded, entries);
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn cff2_fdselect_round_trips_compact_and_wide_font_dict_indices() {
        for values in [vec![0, 0, 2, 2, 1], vec![0, 300, 300, 1]] {
            let encoded = build_cff2_fdselect(&values).unwrap();
            let (decoded, consumed) = parse_cff2_fdselect(&encoded, values.len()).unwrap();

            assert_eq!(decoded, values);
            assert_eq!(consumed, encoded.len());
        }
    }
}
