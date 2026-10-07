//! Freeze variable OpenType layout before the outline instancer removes fvar.

use fontmin_diagnostics::{FontminError, Result};
use fontmin_ttf::{OwnedSfntTable, OwnedTtfFont, TtfFont, read_ttf, write_ttf};

use crate::variation::{VariationAxis, instance_layout_font};

const LAYOUT_TAGS: [&str; 3] = ["GSUB", "GPOS", "GDEF"];

pub(crate) fn resolve(
    font: &TtfFont<'_>,
    axes: &[VariationAxis],
    coordinates: &[([u8; 4], f32)],
) -> Result<Option<Vec<OwnedSfntTable>>> {
    let mut variable = false;
    for tag in ["GSUB", "GPOS"] {
        if let Some(data) = font.table(tag) {
            variable |= validate_feature_variations(data, axes.len())?;
        }
    }
    if let Some(data) = font.table("GDEF") {
        variable |= has_gdef_variations(data)?;
    }
    if !variable {
        return Ok(None);
    }
    // The outline instancer evaluates avar segment maps, but not avar v2's
    // cross-axis deltas. Mixing that location with HarfBuzz's would be unsafe.
    if let Some(data) = font.table("avar")
        && Reader::new(data).u32(0)? != 0x0001_0000
    {
        return Err(FontminError::unsupported(
            "instancing variable layout with avar version other than 1.0",
        ));
    }
    // JSTF can reference GSUB/GPOS lookup indices that HarfBuzz rewrites.
    if font.table("JSTF").is_some() {
        return Err(FontminError::unsupported(
            "instancing variable layout with JSTF lookup references",
        ));
    }
    let output = instance_layout_font(font.data, axes, coordinates)?;
    let instanced = read_ttf(&output)?;
    if instanced.table("fvar").is_some() || glyph_count(font)? != glyph_count(&instanced)? {
        return Err(FontminError::convert_failed(
            "layout instancing did not preserve the static glyph identity",
        ));
    }
    for tag in ["GSUB", "GPOS"] {
        if let Some(data) = instanced.table(tag)
            && validate_feature_variations(data, 0)?
        {
            return Err(FontminError::convert_failed(
                "layout instancing left unresolved FeatureVariations",
            ));
        }
    }
    if let Some(data) = instanced.table("GDEF")
        && has_gdef_variations(data)?
    {
        return Err(FontminError::convert_failed(
            "layout instancing left an unresolved GDEF variation store",
        ));
    }
    Ok(Some(
        LAYOUT_TAGS
            .iter()
            .filter_map(|&tag| {
                instanced.table(tag).map(|data| OwnedSfntTable {
                    tag: tag.into(),
                    data: data.to_vec(),
                })
            })
            .collect(),
    ))
}

pub(crate) fn replace(output: Vec<u8>, layout: Option<Vec<OwnedSfntTable>>) -> Result<Vec<u8>> {
    let Some(mut tables) = layout else {
        return Ok(output);
    };
    let font = read_ttf(&output)?;
    tables.extend(
        font.tables
            .iter()
            .filter(|record| !LAYOUT_TAGS.contains(&record.tag.as_str()))
            .map(|record| OwnedSfntTable {
                tag: record.tag.clone(),
                data: font.table(&record.tag).expect("validated table").to_vec(),
            }),
    );
    write_ttf(&OwnedTtfFont { tables })
}

fn glyph_count(font: &TtfFont<'_>) -> Result<u16> {
    let data = font.table("maxp").ok_or_else(|| invalid("missing maxp"))?;
    Reader::new(data).u16(4)
}

fn has_gdef_variations(data: &[u8]) -> Result<bool> {
    let reader = Reader::new(data);
    let version = reader.u32(0)?;
    if ![0x0001_0000, 0x0001_0002, 0x0001_0003].contains(&version) {
        return Err(FontminError::unsupported(
            "GDEF version for static instancing",
        ));
    }
    reader.bytes(0, if version == 0x0001_0000 { 12 } else { 14 })?;
    Ok(version == 0x0001_0003 && reader.u32(14)? != 0)
}

/// `HarfBuzz` may sanitize unsupported structures away. Reject those before
/// calling it: success here promises format-1 conditions with valid references,
/// including records that do not match this particular location.
fn validate_feature_variations(data: &[u8], axis_count: usize) -> Result<bool> {
    let mut reader = Reader::new(data);
    let version = reader.u32(0)?;
    if ![0x0001_0000, 0x0001_0001].contains(&version) {
        return Err(FontminError::unsupported(
            "layout version for static instancing",
        ));
    }
    reader.bytes(0, 10)?;
    if version == 0x0001_0000 {
        return Ok(false);
    }
    let offset = reader.u32(10)? as usize;
    if offset == 0 {
        return Ok(false);
    }
    let feature_list = reader.relative(0, u32::from(reader.u16(6)?))?;
    let feature_count = usize::from(reader.u16(feature_list)?);
    let lookup_list = reader.relative(0, u32::from(reader.u16(8)?))?;
    let lookup_count = reader.u16(lookup_list)?;
    reader.array(feature_list + 2, feature_count, 6)?;
    reader.array(lookup_list + 2, usize::from(lookup_count), 2)?;
    for index in 0..feature_count {
        let feature = reader.relative(
            feature_list,
            u32::from(reader.u16(feature_list + 6 + index * 6)?),
        )?;
        validate_feature(&mut reader, feature, lookup_count, false)?;
    }
    reader.version(offset, "FeatureVariations version for static instancing")?;
    let count = reader.u32(offset + 4)? as usize;
    reader.array(offset + 8, count, 8)?;
    for index in 0..count {
        let record = offset + 8 + index * 8;
        let condition_offset = reader.u32(record)?;
        if condition_offset != 0 {
            let set = reader.relative(offset, condition_offset)?;
            let count = usize::from(reader.u16(set)?);
            reader.array(set + 2, count, 4)?;
            for index in 0..count {
                let condition = reader.relative(set, reader.u32(set + 2 + index * 4)?)?;
                if reader.u16(condition)? != 1 {
                    return Err(FontminError::unsupported(
                        "FeatureVariations condition format for static instancing (only format 1 is supported)",
                    ));
                }
                let axis = usize::from(reader.u16(condition + 2)?);
                let min = reader.i16(condition + 4)?;
                let max = reader.i16(condition + 6)?;
                if axis >= axis_count || min < -16_384 || max > 16_384 || min > max {
                    return Err(invalid("invalid FeatureVariations axis or condition range"));
                }
            }
        }
        let substitution_offset = reader.u32(record + 4)?;
        if substitution_offset != 0 {
            let substitution = reader.relative(offset, substitution_offset)?;
            reader.version(
                substitution,
                "FeatureTableSubstitution version for static instancing",
            )?;
            let count = usize::from(reader.u16(substitution + 4)?);
            reader.array(substitution + 6, count, 6)?;
            let mut previous = None;
            for index in 0..count {
                let record = substitution + 6 + index * 6;
                let feature_index = usize::from(reader.u16(record)?);
                if feature_index >= feature_count
                    || previous.is_some_and(|previous| previous >= feature_index)
                {
                    return Err(invalid("invalid FeatureVariations feature index"));
                }
                previous = Some(feature_index);
                let feature = reader.relative(substitution, reader.u32(record + 2)?)?;
                validate_feature(&mut reader, feature, lookup_count, true)?;
            }
        }
    }
    Ok(true)
}

fn validate_feature(
    reader: &mut Reader<'_>,
    offset: usize,
    lookup_count: u16,
    alternate: bool,
) -> Result<()> {
    // Alternate FeatureParams formats are feature-specific; do not silently
    // drop parameters we have not validated.
    if alternate && reader.u16(offset)? != 0 {
        return Err(FontminError::unsupported(
            "FeatureVariations with alternate feature parameters for static instancing",
        ));
    }
    let count = usize::from(reader.u16(offset + 2)?);
    reader.array(offset + 4, count, 2)?;
    for index in 0..count {
        if reader.u16(offset + 4 + index * 2)? >= lookup_count {
            return Err(invalid("invalid FeatureVariations lookup index"));
        }
    }
    Ok(())
}

fn invalid(message: &str) -> FontminError {
    FontminError::invalid_font(format!("invalid instance layout: {message}"))
}

struct Reader<'a> {
    data: &'a [u8],
    remaining: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            remaining: 1_000_000,
        }
    }

    fn bytes(&self, offset: usize, length: usize) -> Result<&'a [u8]> {
        offset
            .checked_add(length)
            .and_then(|end| self.data.get(offset..end))
            .ok_or_else(|| invalid("truncated table or offset outside table"))
    }

    fn u16(&self, offset: usize) -> Result<u16> {
        let bytes = self.bytes(offset, 2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn i16(&self, offset: usize) -> Result<i16> {
        let bytes = self.bytes(offset, 2)?;
        Ok(i16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&self, offset: usize) -> Result<u32> {
        let bytes = self.bytes(offset, 4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn relative(&self, base: usize, offset: u32) -> Result<usize> {
        if offset == 0 {
            return Err(invalid("required offset is null"));
        }
        let offset = base
            .checked_add(offset as usize)
            .ok_or_else(|| invalid("offset overflow"))?;
        self.bytes(offset, 2)?;
        Ok(offset)
    }

    fn array(&mut self, offset: usize, count: usize, size: usize) -> Result<()> {
        self.remaining = self
            .remaining
            .checked_sub(count)
            .ok_or_else(|| invalid("FeatureVariations validation limit exceeded"))?;
        let length = count
            .checked_mul(size)
            .ok_or_else(|| invalid("array overflow"))?;
        self.bytes(offset, length)?;
        Ok(())
    }

    fn version(&self, offset: usize, label: &str) -> Result<()> {
        if self.u32(offset)? != 0x0001_0000 {
            return Err(FontminError::unsupported(label));
        }
        Ok(())
    }
}
