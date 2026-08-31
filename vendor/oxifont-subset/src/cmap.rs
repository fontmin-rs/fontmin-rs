/// `cmap` table rewriter.
use std::collections::{BTreeMap, HashMap};

use crate::tables::SubsetError;

/// Walk a cmap table and build a map from Unicode codepoint to GID.
///
/// Prefers full-Unicode format 12 records, then fills BMP mappings from
/// format 4 records. Malformed candidate records are skipped so another
/// usable encoding record can still provide the mapping.
pub(crate) fn cmap_to_gid_map(cmap_data: &[u8]) -> Result<HashMap<u32, u16>, SubsetError> {
    if cmap_data.len() < 4 {
        return Err(SubsetError::InvalidFont("cmap table too short".into()));
    }
    let num_tables = u16::from_be_bytes([cmap_data[2], cmap_data[3]]) as usize;

    if cmap_data.len() < 4 + num_tables * 8 {
        return Err(SubsetError::InvalidFont(
            "cmap table directory truncated".into(),
        ));
    }

    struct EncodingRecord {
        platform_id: u16,
        encoding_id: u16,
        offset: usize,
    }

    let mut records = Vec::with_capacity(num_tables);
    for i in 0..num_tables {
        let base = 4 + i * 8;
        let platform_id = u16::from_be_bytes([cmap_data[base], cmap_data[base + 1]]);
        let encoding_id = u16::from_be_bytes([cmap_data[base + 2], cmap_data[base + 3]]);
        let offset = u32::from_be_bytes([
            cmap_data[base + 4],
            cmap_data[base + 5],
            cmap_data[base + 6],
            cmap_data[base + 7],
        ]) as usize;
        records.push(EncodingRecord {
            platform_id,
            encoding_id,
            offset,
        });
    }

    // Prefer platform 0/4 or 3/10 format 12 records, then platform 0/3 or
    // 3/1 format 4 records.
    let mut result = HashMap::new();
    let mut found_format12 = false;
    let mut found_format4 = false;

    for record in &records {
        if record.offset + 2 > cmap_data.len() {
            continue;
        }
        let format = u16::from_be_bytes([cmap_data[record.offset], cmap_data[record.offset + 1]]);

        match (record.platform_id, record.encoding_id, format) {
            (0, 4, 12) | (3, 10, 12) if !found_format12 => {
                if let Ok(map) = parse_format12(&cmap_data[record.offset..]) {
                    result.extend(map);
                    found_format12 = true;
                }
            }
            (0, 3, 4) | (3, 1, 4) if !found_format4 => {
                if let Ok(map) = parse_format4(&cmap_data[record.offset..]) {
                    for (codepoint, gid) in map {
                        result.entry(u32::from(codepoint)).or_insert(gid);
                    }
                    found_format4 = true;
                }
            }
            _ => {}
        }
    }

    if result.is_empty() {
        // Fall back to any format 4 record when a font lacks the preferred
        // Unicode platform and encoding combinations.
        for record in &records {
            if record.offset + 2 > cmap_data.len() {
                continue;
            }
            let format =
                u16::from_be_bytes([cmap_data[record.offset], cmap_data[record.offset + 1]]);
            if format == 4 {
                if let Ok(map) = parse_format4(&cmap_data[record.offset..]) {
                    result.extend(
                        map.into_iter()
                            .map(|(codepoint, gid)| (u32::from(codepoint), gid)),
                    );
                    break;
                }
            }
        }
    }

    Ok(result)
}

fn parse_format4(data: &[u8]) -> Result<Vec<(u16, u16)>, SubsetError> {
    if data.len() < 14 {
        return Err(SubsetError::InvalidFont(
            "format 4 sub-table too short".into(),
        ));
    }
    let seg_count = usize::from(u16::from_be_bytes([data[6], data[7]])) / 2;
    if seg_count == 0 {
        return Ok(vec![]);
    }

    let end_code_base = 14usize;
    let start_code_base = end_code_base + seg_count * 2 + 2;
    let id_delta_base = start_code_base + seg_count * 2;
    let id_range_offset_base = id_delta_base + seg_count * 2;
    let glyph_id_array_base = id_range_offset_base + seg_count * 2;

    if data.len() < glyph_id_array_base {
        return Err(SubsetError::InvalidFont(
            "format 4 sub-table truncated".into(),
        ));
    }

    let mut pairs = Vec::new();
    for i in 0..seg_count {
        let end_code =
            u16::from_be_bytes([data[end_code_base + i * 2], data[end_code_base + i * 2 + 1]]);
        if end_code == 0xFFFF {
            break;
        }
        let start_code = u16::from_be_bytes([
            data[start_code_base + i * 2],
            data[start_code_base + i * 2 + 1],
        ]);
        let id_delta = i32::from(i16::from_be_bytes([
            data[id_delta_base + i * 2],
            data[id_delta_base + i * 2 + 1],
        ]));
        let id_range_offset = usize::from(u16::from_be_bytes([
            data[id_range_offset_base + i * 2],
            data[id_range_offset_base + i * 2 + 1],
        ]));

        for codepoint in start_code..=end_code {
            let gid = if id_range_offset == 0 {
                ((i32::from(codepoint) + id_delta) & 0xFFFF) as u16
            } else {
                let range_pointer_offset = id_range_offset_base + i * 2;
                let index = range_pointer_offset
                    + id_range_offset
                    + usize::from(codepoint - start_code) * 2;
                if index + 2 > data.len() {
                    0
                } else {
                    let raw_gid = u16::from_be_bytes([data[index], data[index + 1]]);
                    if raw_gid == 0 {
                        0
                    } else {
                        ((i32::from(raw_gid) + id_delta) & 0xFFFF) as u16
                    }
                }
            };
            if gid != 0 {
                pairs.push((codepoint, gid));
            }
        }
    }

    Ok(pairs)
}

fn parse_format12(data: &[u8]) -> Result<HashMap<u32, u16>, SubsetError> {
    if data.len() < 16 {
        return Err(SubsetError::InvalidFont(
            "format 12 sub-table too short".into(),
        ));
    }
    let declared_len = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let num_groups = u32::from_be_bytes([data[12], data[13], data[14], data[15]]) as usize;
    let groups_end = num_groups
        .checked_mul(12)
        .and_then(|groups_len| 16usize.checked_add(groups_len))
        .ok_or_else(|| SubsetError::InvalidFont("format 12 group count overflow".into()))?;
    if declared_len < groups_end || data.len() < declared_len {
        return Err(SubsetError::InvalidFont(
            "format 12 sub-table truncated".into(),
        ));
    }

    let mut previous_end = None;
    let mut mapping_count = 0usize;
    for i in 0..num_groups {
        let base = 16 + i * 12;
        let start_char =
            u32::from_be_bytes([data[base], data[base + 1], data[base + 2], data[base + 3]]);
        let end_char = u32::from_be_bytes([
            data[base + 4],
            data[base + 5],
            data[base + 6],
            data[base + 7],
        ]);
        let start_glyph = u32::from_be_bytes([
            data[base + 8],
            data[base + 9],
            data[base + 10],
            data[base + 11],
        ]);
        if start_char > end_char || end_char > 0x10_FFFF {
            return Err(SubsetError::InvalidFont(
                "format 12 group has an invalid Unicode range".into(),
            ));
        }
        if previous_end.is_some_and(|previous| start_char <= previous) {
            return Err(SubsetError::InvalidFont(
                "format 12 groups are not strictly ordered".into(),
            ));
        }
        let count = end_char - start_char + 1;
        let end_glyph = start_glyph
            .checked_add(count - 1)
            .ok_or_else(|| SubsetError::InvalidFont("format 12 glyph range overflow".into()))?;
        if end_glyph > u32::from(u16::MAX) {
            return Err(SubsetError::InvalidFont(
                "format 12 glyph ID exceeds u16".into(),
            ));
        }
        mapping_count = mapping_count
            .checked_add(count as usize)
            .ok_or_else(|| SubsetError::InvalidFont("format 12 mapping count overflow".into()))?;
        previous_end = Some(end_char);
    }

    let mut map = HashMap::with_capacity(mapping_count);
    for i in 0..num_groups {
        let base = 16 + i * 12;
        let start_char =
            u32::from_be_bytes([data[base], data[base + 1], data[base + 2], data[base + 3]]);
        let end_char = u32::from_be_bytes([
            data[base + 4],
            data[base + 5],
            data[base + 6],
            data[base + 7],
        ]);
        let start_glyph = u32::from_be_bytes([
            data[base + 8],
            data[base + 9],
            data[base + 10],
            data[base + 11],
        ]);
        let count = end_char - start_char + 1;
        for offset in 0..count {
            map.insert(start_char + offset, (start_glyph + offset) as u16);
        }
    }
    Ok(map)
}

// ---------------------------------------------------------------------------
// cmap format 4 builder
// ---------------------------------------------------------------------------

/// Encode `codepoints_to_new_gid` as a format-4 cmap sub-table.
///
/// Only BMP (≤ 0xFFFF) codepoints are encoded; higher codepoints must go
/// through a format-12 sub-table.
fn build_format4(bmp_map: &BTreeMap<u16, u16>, language: u16) -> Result<Vec<u8>, SubsetError> {
    // Build segments: consecutive codepoints with constant (gid - cp) delta.
    // Terminal sentinel segment: (0xFFFF, 0xFFFF, 1, 0) — required by spec.
    struct Seg {
        start: u16,
        end: u16,
        delta: i32, // signed; applied mod 65536
    }

    let mut segments: Vec<Seg> = Vec::new();

    if !bmp_map.is_empty() {
        let mut iter = bmp_map.iter();
        let (&first_cp, &first_gid) = iter.next().expect("non-empty");
        let mut seg_start = first_cp;
        let mut seg_end = first_cp;
        let mut seg_delta = first_gid as i32 - first_cp as i32;

        for (&cp, &gid) in iter {
            let delta = gid as i32 - cp as i32;
            if cp == seg_end + 1 && delta == seg_delta {
                // Extend current segment.
                seg_end = cp;
            } else {
                segments.push(Seg {
                    start: seg_start,
                    end: seg_end,
                    delta: seg_delta,
                });
                seg_start = cp;
                seg_end = cp;
                seg_delta = delta;
            }
        }
        segments.push(Seg {
            start: seg_start,
            end: seg_end,
            delta: seg_delta,
        });
    }

    // Sentinel.
    segments.push(Seg {
        start: 0xFFFF,
        end: 0xFFFF,
        delta: 1,
    });

    // `segments` always contains at least the terminal sentinel, so `seg_count >= 1`.
    // Perform all header arithmetic in `usize`/`u32` and validate the results fit the
    // u16 fields of the format-4 encoding *before* narrowing. A subset can exceed the
    // format-4 addressable size (~8189 segments) and would otherwise overflow u16 and
    // either panic in debug or emit a corrupt table in release.
    let seg_count = segments.len();

    // Header (14 bytes) + reservedPad (2) + 4 parallel arrays (each seg_count * 2 bytes).
    // Total: 16 + seg_count * 8 bytes. This is the tightest bound: if `length` fits in a
    // u16 then segCountX2, searchRange, entrySelector and rangeShift all fit as well.
    let length = seg_count
        .checked_mul(8)
        .and_then(|arrays| arrays.checked_add(16))
        .filter(|&len| len <= u16::MAX as usize)
        .ok_or_else(|| {
            SubsetError::InvalidFont(format!(
                "cmap format 4 subtable too large: {seg_count} segments exceed the u16-addressable size"
            ))
        })?;

    // searchRange = 2 * 2^floor(log2(segCount)); entrySelector = floor(log2(segCount));
    // rangeShift = 2 * segCount - searchRange. Computed in u32 to avoid overflow.
    let entry_selector = seg_count.ilog2(); // seg_count >= 1
    let search_range = 2u32 * (1u32 << entry_selector);
    let range_shift = seg_count as u32 * 2 - search_range;

    let seg_count = seg_count as u16;
    let search_range = search_range as u16;
    let entry_selector = entry_selector as u16;
    let range_shift = range_shift as u16;
    let length = length as u16;

    let mut out = Vec::with_capacity(length as usize);
    out.extend_from_slice(&4u16.to_be_bytes()); // format
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(&language.to_be_bytes());
    out.extend_from_slice(&(seg_count * 2).to_be_bytes()); // segCountX2
    out.extend_from_slice(&search_range.to_be_bytes());
    out.extend_from_slice(&entry_selector.to_be_bytes());
    out.extend_from_slice(&range_shift.to_be_bytes());
    // endCode array
    for seg in &segments {
        out.extend_from_slice(&seg.end.to_be_bytes());
    }
    out.extend_from_slice(&0u16.to_be_bytes()); // reservedPad
                                                // startCode array
    for seg in &segments {
        out.extend_from_slice(&seg.start.to_be_bytes());
    }
    // idDelta array (signed i16)
    for seg in &segments {
        let d = (seg.delta as i16).to_be_bytes();
        out.extend_from_slice(&d);
    }
    // idRangeOffset array — all 0 (use delta-only encoding)
    for _ in &segments {
        out.extend_from_slice(&0u16.to_be_bytes());
    }
    // No glyphIdArray entries (idRangeOffset all 0).

    Ok(out)
}

// ---------------------------------------------------------------------------
// cmap format 12 builder
// ---------------------------------------------------------------------------

/// Encode `codepoints_to_new_gid` as a format-12 cmap sub-table (full Unicode).
fn build_format12(map: &BTreeMap<u32, u16>, language: u32) -> Vec<u8> {
    // Build sequential map groups (contiguous cp with contiguous gid).
    struct Group {
        start_char: u32,
        end_char: u32,
        start_glyph: u32,
    }

    let mut groups: Vec<Group> = Vec::new();

    if !map.is_empty() {
        let mut iter = map.iter();
        let (&first_cp, &first_gid) = iter.next().expect("non-empty");
        let mut g_start_cp = first_cp;
        let mut g_end_cp = first_cp;
        let mut g_start_gid = first_gid as u32;
        let mut g_end_gid = first_gid as u32;

        for (&cp, &gid) in iter {
            if cp == g_end_cp + 1 && gid as u32 == g_end_gid + 1 {
                g_end_cp = cp;
                g_end_gid = gid as u32;
            } else {
                groups.push(Group {
                    start_char: g_start_cp,
                    end_char: g_end_cp,
                    start_glyph: g_start_gid,
                });
                g_start_cp = cp;
                g_end_cp = cp;
                g_start_gid = gid as u32;
                g_end_gid = gid as u32;
            }
        }
        groups.push(Group {
            start_char: g_start_cp,
            end_char: g_end_cp,
            start_glyph: g_start_gid,
        });
    }

    let num_groups = groups.len() as u32;
    // format(2) + reserved(2) + length(4) + language(4) + numGroups(4) + groups * 12
    let length = 16u32 + num_groups * 12;

    let mut out = Vec::with_capacity(length as usize);
    out.extend_from_slice(&12u16.to_be_bytes()); // format
    out.extend_from_slice(&0u16.to_be_bytes()); // reserved
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(&language.to_be_bytes());
    out.extend_from_slice(&num_groups.to_be_bytes());
    for g in &groups {
        out.extend_from_slice(&g.start_char.to_be_bytes());
        out.extend_from_slice(&g.end_char.to_be_bytes());
        out.extend_from_slice(&g.start_glyph.to_be_bytes());
    }

    out
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build a new `cmap` table containing only the given codepoint → new-GID
/// mappings.
///
/// Emits:
/// - Encoding record (Platform 3 / Encoding 1 / Format 4) for BMP codepoints.
/// - Encoding record (Platform 0 / Encoding 3 / Format 4) — same data.
/// - If any codepoints > 0xFFFF: additionally Format 12 for both platforms.
///
/// # Errors
/// Returns [`SubsetError::InvalidFont`] only if something is structurally
/// impossible (currently infallible; kept for API consistency).
pub fn rewrite_cmap(codepoints_to_new_gid: &BTreeMap<u32, u16>) -> Result<Vec<u8>, SubsetError> {
    rewrite_cmap_with_records(codepoints_to_new_gid, &[])
}

/// A non-canonical source cmap encoding record to retain after GID remapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedEncodingRecord {
    /// OpenType cmap platform identifier.
    pub platform_id: u16,
    /// Platform-specific encoding identifier.
    pub encoding_id: u16,
    /// Source subtable language value.
    pub language: u32,
    /// Character-code to remapped-GID pairs.
    pub mappings: BTreeMap<u32, u16>,
}

/// Build a canonical Unicode cmap plus remapped legacy or symbol records.
pub fn rewrite_cmap_with_records(
    codepoints_to_new_gid: &BTreeMap<u32, u16>,
    retained_records: &[RetainedEncodingRecord],
) -> Result<Vec<u8>, SubsetError> {
    struct OutputRecord {
        platform_id: u16,
        encoding_id: u16,
        subtable: Vec<u8>,
    }

    let bmp_map: BTreeMap<u16, u16> = codepoints_to_new_gid
        .iter()
        .filter(|(&cp, _)| cp < 0xFFFF)
        .map(|(&cp, &gid)| (cp as u16, gid))
        .collect();
    let needs_format12 = codepoints_to_new_gid.keys().any(|&cp| cp >= 0xFFFF);
    let format4 = build_format4(&bmp_map, 0)?;
    let mut records = vec![
        OutputRecord {
            platform_id: 0,
            encoding_id: 3,
            subtable: format4.clone(),
        },
        OutputRecord {
            platform_id: 3,
            encoding_id: 1,
            subtable: format4,
        },
    ];

    if needs_format12 {
        let format12 = build_format12(codepoints_to_new_gid, 0);
        records.extend([
            OutputRecord {
                platform_id: 0,
                encoding_id: 4,
                subtable: format12.clone(),
            },
            OutputRecord {
                platform_id: 3,
                encoding_id: 10,
                subtable: format12,
            },
        ]);
    }

    for record in retained_records {
        if record.mappings.is_empty() {
            continue;
        }
        let can_use_format4 = record.language <= u32::from(u16::MAX)
            && record.mappings.keys().all(|&codepoint| codepoint < 0xFFFF);
        let subtable = if can_use_format4 {
            let bmp_map = record
                .mappings
                .iter()
                .map(|(&codepoint, &gid)| (codepoint as u16, gid))
                .collect();
            build_format4(&bmp_map, record.language as u16)?
        } else {
            build_format12(&record.mappings, record.language)
        };
        records.push(OutputRecord {
            platform_id: record.platform_id,
            encoding_id: record.encoding_id,
            subtable,
        });
    }

    let num_records = u16::try_from(records.len())
        .map_err(|_| SubsetError::InvalidFont("cmap encoding record count exceeds u16".into()))?;
    let header_size = 4usize + records.len() * 8;
    let mut subtables = Vec::<Vec<u8>>::new();
    let mut offsets = Vec::with_capacity(records.len());
    for record in &records {
        if let Some(index) = subtables.iter().position(|table| table == &record.subtable) {
            offsets.push(header_size + subtables[..index].iter().map(Vec::len).sum::<usize>());
        } else {
            offsets.push(header_size + subtables.iter().map(Vec::len).sum::<usize>());
            subtables.push(record.subtable.clone());
        }
    }

    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_be_bytes()); // version
    out.extend_from_slice(&num_records.to_be_bytes());
    for (record, offset) in records.iter().zip(offsets) {
        let offset = u32::try_from(offset)
            .map_err(|_| SubsetError::InvalidFont("cmap table offset exceeds u32".into()))?;
        out.extend_from_slice(&record.platform_id.to_be_bytes());
        out.extend_from_slice(&record.encoding_id.to_be_bytes());
        out.extend_from_slice(&offset.to_be_bytes());
    }

    for subtable in subtables {
        out.extend_from_slice(&subtable);
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn format12_group(start_char: u32, end_char: u32, start_glyph: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&12u16.to_be_bytes());
        data.extend_from_slice(&0u16.to_be_bytes());
        data.extend_from_slice(&28u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&1u32.to_be_bytes());
        data.extend_from_slice(&start_char.to_be_bytes());
        data.extend_from_slice(&end_char.to_be_bytes());
        data.extend_from_slice(&start_glyph.to_be_bytes());
        data
    }

    #[test]
    fn format12_rejects_codepoints_beyond_unicode() {
        let data = format12_group(0x10_FFFF, 0x11_0000, 1);
        assert!(parse_format12(&data).is_err());
    }

    #[test]
    fn format12_rejects_descending_groups() {
        let data = format12_group(0x100, 0x80, 1);
        assert!(parse_format12(&data).is_err());
    }

    #[test]
    fn format12_rejects_glyph_id_overflow() {
        let data = format12_group(0x100, 0x101, u32::from(u16::MAX));
        assert!(parse_format12(&data).is_err());
    }

    #[test]
    fn format4_small_header_fields() {
        // Two isolated codepoints → 2 real segments + 1 sentinel = 3 segments.
        let mut m: BTreeMap<u16, u16> = BTreeMap::new();
        m.insert(0x41, 1);
        m.insert(0x43, 2);
        let data = build_format4(&m, 0).expect("small format 4 must build");
        let seg_count_x2 = u16::from_be_bytes([data[6], data[7]]);
        assert_eq!(seg_count_x2, 3 * 2);
        // searchRange = 2 * 2^floor(log2(3)) = 2 * 2 = 4.
        assert_eq!(u16::from_be_bytes([data[8], data[9]]), 4);
        // entrySelector = floor(log2(3)) = 1.
        assert_eq!(u16::from_be_bytes([data[10], data[11]]), 1);
        // rangeShift = 2*3 - 4 = 2.
        assert_eq!(u16::from_be_bytes([data[12], data[13]]), 2);
    }

    #[test]
    fn format4_large_segment_count_no_overflow() {
        // Build a subset with many isolated segments — a count that would overflow the
        // former u16 `seg_count * 8` / `next_power_of_two` arithmetic. Isolated even
        // codepoints (gaps of 2) each form their own segment.
        let mut m: BTreeMap<u16, u16> = BTreeMap::new();
        // 8000 real segments + 1 sentinel = 8001; length = 16 + 8001*8 = 64024 <= 65535.
        for i in 0..8000u16 {
            m.insert(i * 2, i.wrapping_add(1));
        }
        let data = build_format4(&m, 0).expect("large-but-valid format 4 must build");
        let seg_count_x2 = u16::from_be_bytes([data[6], data[7]]);
        assert_eq!(seg_count_x2 as usize, 8001 * 2);
        let length = u16::from_be_bytes([data[2], data[3]]) as usize;
        assert_eq!(length, 16 + 8001 * 8);
        assert_eq!(data.len(), length);
    }

    #[test]
    fn format4_oversized_rejected_without_panic() {
        // A segment count beyond the u16-addressable format-4 size must return a typed
        // error instead of overflowing/panicking. 8200 isolated segments + sentinel =
        // 8201 → length = 16 + 8201*8 = 65624 > u16::MAX.
        let mut m: BTreeMap<u16, u16> = BTreeMap::new();
        for i in 0..8200u16 {
            m.insert(i * 2, i.wrapping_add(1));
        }
        let result = build_format4(&m, 0);
        assert!(
            matches!(result, Err(SubsetError::InvalidFont(_))),
            "oversized format 4 must be rejected, got {result:?}"
        );
    }

    #[test]
    fn retained_records_keep_identity_language_and_remapped_gids() {
        let unicode = BTreeMap::from([(0x41, 1)]);
        let retained = RetainedEncodingRecord {
            platform_id: 1,
            encoding_id: 0,
            language: 7,
            mappings: BTreeMap::from([(0x41, 2)]),
        };
        let cmap = rewrite_cmap_with_records(&unicode, &[retained]).unwrap();

        assert_eq!(u16::from_be_bytes([cmap[2], cmap[3]]), 3);
        assert_eq!(&cmap[20..24], &[0, 1, 0, 0]);
        let offset = u32::from_be_bytes(cmap[24..28].try_into().unwrap()) as usize;
        assert_eq!(
            u16::from_be_bytes(cmap[offset..offset + 2].try_into().unwrap()),
            4
        );
        assert_eq!(
            u16::from_be_bytes(cmap[offset + 4..offset + 6].try_into().unwrap()),
            7
        );
    }
}
