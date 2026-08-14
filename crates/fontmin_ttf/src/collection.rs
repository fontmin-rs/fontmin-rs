use std::collections::HashSet;

use fontmin_diagnostics::{FontminError, Result};

use crate::sfnt::{
    OwnedSfntFont, OwnedSfntTable, SfntFlavor, read_exact, read_u16, read_u32, write_sfnt,
};

const COLLECTION_HEADER_SIZE: usize = 12;
const SFNT_HEADER_SIZE: usize = 12;
const SFNT_TABLE_RECORD_SIZE: usize = 16;

/// A validated TrueType/OpenType collection header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontCollection {
    pub major_version: u16,
    pub minor_version: u16,
    pub face_offsets: Vec<usize>,
}

/// Reads and validates the header and every face directory in a TTC/OTC file.
pub fn read_font_collection(input: &[u8]) -> Result<FontCollection> {
    if !input.starts_with(b"ttcf") {
        return Err(FontminError::invalid_font(
            "expected a TrueType/OpenType collection",
        ));
    }

    let major_version = read_u16(input, 4)?;
    let minor_version = read_u16(input, 6)?;
    if !matches!((major_version, minor_version), (1 | 2, 0)) {
        return Err(FontminError::invalid_font(format!(
            "unsupported font collection version {major_version}.{minor_version}",
        )));
    }

    let face_count = usize::try_from(read_u32(input, 8)?)
        .map_err(|_| FontminError::invalid_font("font collection face count is too large"))?;
    if face_count == 0 {
        return Err(FontminError::invalid_font(
            "font collection contains no faces",
        ));
    }
    let offsets_end = COLLECTION_HEADER_SIZE
        .checked_add(
            face_count
                .checked_mul(4)
                .ok_or_else(|| FontminError::invalid_font("font collection is too large"))?,
        )
        .ok_or_else(|| FontminError::invalid_font("font collection is too large"))?;
    let header_end = if major_version == 2 {
        validate_dsig(input, offsets_end)?
    } else {
        offsets_end
    };
    read_exact(input, 0, header_end)?;

    let mut face_offsets = Vec::with_capacity(face_count);
    for index in 0..face_count {
        let offset = usize::try_from(read_u32(input, COLLECTION_HEADER_SIZE + index * 4)?)
            .map_err(|_| FontminError::invalid_font("font collection face offset is too large"))?;
        if offset < header_end || !offset.is_multiple_of(4) {
            return Err(FontminError::invalid_font(format!(
                "font collection face {index} has an invalid offset",
            )));
        }
        read_collection_face(input, offset)?;
        face_offsets.push(offset);
    }

    Ok(FontCollection {
        major_version,
        minor_version,
        face_offsets,
    })
}

/// Extracts one zero-based face from a TTC/OTC file as a standalone SFNT.
pub fn extract_font_collection_face(input: &[u8], face_index: usize) -> Result<Vec<u8>> {
    let collection = read_font_collection(input)?;
    let offset = collection
        .face_offsets
        .get(face_index)
        .copied()
        .ok_or_else(|| {
            FontminError::invalid_font(format!(
                "font collection face index {face_index} is out of range ({} faces)",
                collection.face_offsets.len(),
            ))
        })?;
    let face = read_collection_face(input, offset)?;
    let tables = face
        .tables
        .into_iter()
        .map(|table| {
            Ok(OwnedSfntTable {
                tag: table.tag,
                data: read_exact(input, table.offset, table.length)?.to_vec(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    write_sfnt(&OwnedSfntFont {
        flavor: face.flavor,
        tables,
    })
}

struct CollectionFace {
    flavor: SfntFlavor,
    tables: Vec<CollectionTable>,
}

struct CollectionTable {
    tag: String,
    offset: usize,
    length: usize,
}

fn validate_dsig(input: &[u8], offset: usize) -> Result<usize> {
    let header_end = offset
        .checked_add(12)
        .ok_or_else(|| FontminError::invalid_font("font collection header is too large"))?;
    read_exact(input, offset, 12)?;
    let length = usize::try_from(read_u32(input, offset + 4)?)
        .map_err(|_| FontminError::invalid_font("font collection DSIG is too large"))?;
    let data_offset = usize::try_from(read_u32(input, offset + 8)?)
        .map_err(|_| FontminError::invalid_font("font collection DSIG offset is too large"))?;
    if length > 0 {
        read_exact(input, data_offset, length)?;
    }

    Ok(header_end)
}

fn read_collection_face(input: &[u8], offset: usize) -> Result<CollectionFace> {
    let signature: [u8; 4] = read_exact(input, offset, 4)?
        .try_into()
        .map_err(|_| FontminError::invalid_font("font collection face is truncated"))?;
    let flavor = SfntFlavor::from_signature(signature)?;
    let table_count = usize::from(read_u16(input, offset + 4)?);
    if table_count == 0 {
        return Err(FontminError::invalid_font(
            "font collection face contains no tables",
        ));
    }
    let directory_end = offset
        .checked_add(SFNT_HEADER_SIZE)
        .and_then(|value| {
            table_count
                .checked_mul(SFNT_TABLE_RECORD_SIZE)
                .and_then(|size| value.checked_add(size))
        })
        .ok_or_else(|| FontminError::invalid_font("font collection face is too large"))?;
    read_exact(input, offset, directory_end - offset)?;

    let mut seen_tags = HashSet::with_capacity(table_count);
    let mut ranges = Vec::with_capacity(table_count);
    let mut tables = Vec::with_capacity(table_count);
    for index in 0..table_count {
        let record_offset = offset + SFNT_HEADER_SIZE + index * SFNT_TABLE_RECORD_SIZE;
        let tag_bytes = read_exact(input, record_offset, 4)?;
        if !tag_bytes.is_ascii() {
            return Err(FontminError::invalid_font(
                "font collection table tag is not ASCII",
            ));
        }
        let tag = std::str::from_utf8(tag_bytes)
            .map_err(|_| FontminError::invalid_font("font collection table tag is not ASCII"))?
            .to_owned();
        if !seen_tags.insert(tag.clone()) {
            return Err(FontminError::invalid_font(format!(
                "duplicate sfnt table tag `{tag}`",
            )));
        }

        let table_offset = usize::try_from(read_u32(input, record_offset + 8)?)
            .map_err(|_| FontminError::invalid_font("font collection table offset is too large"))?;
        let table_length = usize::try_from(read_u32(input, record_offset + 12)?)
            .map_err(|_| FontminError::invalid_font("font collection table is too large"))?;
        let table_end = table_offset
            .checked_add(table_length)
            .ok_or_else(|| FontminError::invalid_font("font collection table range overflows"))?;
        if table_length > 0 {
            read_exact(input, table_offset, table_length)?;
            if !table_offset.is_multiple_of(4)
                || (offset..directory_end).contains(&table_offset)
                || (offset..directory_end).contains(&table_end.saturating_sub(1))
            {
                return Err(FontminError::invalid_font(format!(
                    "font collection table {tag} has an invalid range",
                )));
            }
            ranges.push((table_offset, table_end, tag.clone()));
        }
        tables.push(CollectionTable {
            tag,
            offset: table_offset,
            length: table_length,
        });
    }

    ranges.sort_unstable_by_key(|(start, _, _)| *start);
    for pair in ranges.windows(2) {
        if pair[0].1 > pair[1].0 {
            return Err(FontminError::invalid_font(format!(
                "font collection tables {} and {} overlap",
                pair[0].2, pair[1].2,
            )));
        }
    }

    Ok(CollectionFace { flavor, tables })
}

#[cfg(test)]
mod tests {
    use fontmin_testing::{ROBOTO, SOURCE_SANS_3_REGULAR_CFF, font_collection};

    use super::{extract_font_collection_face, read_font_collection};

    #[test]
    fn extracts_true_type_and_cff_faces() {
        let input = font_collection(&[ROBOTO, SOURCE_SANS_3_REGULAR_CFF]);
        let info = read_font_collection(&input).unwrap();
        let ttf = extract_font_collection_face(&input, 0).unwrap();
        let otf = extract_font_collection_face(&input, 1).unwrap();

        assert_eq!(info.major_version, 1);
        assert_eq!(info.face_offsets.len(), 2);
        assert_eq!(
            crate::inspect_ttf(&ttf).unwrap().family_name.as_deref(),
            Some("Roboto")
        );
        assert_eq!(
            crate::inspect_sfnt(&otf, crate::SfntFlavor::OpenTypeCff)
                .unwrap()
                .family_name
                .as_deref(),
            Some("Source Sans 3"),
        );
    }

    #[test]
    fn rejects_out_of_range_faces_and_invalid_headers() {
        let input = font_collection(&[ROBOTO]);

        assert!(extract_font_collection_face(&input, 1).is_err());
        assert!(read_font_collection(ROBOTO).is_err());
        assert!(read_font_collection(b"ttcf\0\x01").is_err());
    }
}
