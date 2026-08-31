//! COLR v0/v1 glyph closure and subsetting.
//!
//! Version 1 paint graphs are left in place so their relative offsets remain
//! valid. Reachable glyph references and base records are remapped in place;
//! unreachable bytes are harmless padding and are omitted by renderers.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use crate::SubsetError;

const COLR_V0_HEADER_LEN: usize = 14;
const COLR_V1_HEADER_LEN: usize = 34;
const MAX_PAINT_DEPTH: usize = 256;

#[derive(Clone, Copy)]
struct ColrHeader {
    version: u16,
    num_base_records: usize,
    base_records_offset: usize,
    layer_records_offset: usize,
    num_layer_records: usize,
    base_glyph_list_offset: Option<usize>,
    layer_list_offset: Option<usize>,
    clip_list_offset: Option<usize>,
}

#[derive(Clone, Copy)]
struct BaseGlyphRecord {
    gid: u16,
    first_layer_index: usize,
    num_layers: usize,
}

#[derive(Clone, Copy)]
struct LayerRecord {
    gid: u16,
    palette_index: u16,
}

#[derive(Clone, Copy)]
struct BaseGlyphPaintRecord {
    gid: u16,
    paint_offset: usize,
    relative_paint_offset: u32,
}

struct PaintNode {
    children: Vec<usize>,
    glyph_references: Vec<(usize, u16)>,
    known_format: bool,
}

struct PaintGraph<'a> {
    table: &'a [u8],
    base_roots: HashMap<u16, usize>,
    layer_offsets: Vec<usize>,
}

fn invalid(message: impl Into<String>) -> SubsetError {
    SubsetError::InvalidFont(message.into())
}

fn checked_range(
    data: &[u8],
    offset: usize,
    length: usize,
    label: &str,
) -> Result<(), SubsetError> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| invalid(format!("COLR {label} range overflows")))?;
    if end > data.len() {
        return Err(invalid(format!("COLR {label} is truncated")));
    }
    Ok(())
}

fn read_u8(data: &[u8], offset: usize, label: &str) -> Result<u8, SubsetError> {
    data.get(offset)
        .copied()
        .ok_or_else(|| invalid(format!("COLR {label} is truncated")))
}

fn read_u16(data: &[u8], offset: usize, label: &str) -> Result<u16, SubsetError> {
    checked_range(data, offset, 2, label)?;
    Ok(u16::from_be_bytes([data[offset], data[offset + 1]]))
}

fn read_u24(data: &[u8], offset: usize, label: &str) -> Result<u32, SubsetError> {
    checked_range(data, offset, 3, label)?;
    Ok(u32::from_be_bytes([
        0,
        data[offset],
        data[offset + 1],
        data[offset + 2],
    ]))
}

fn read_u32(data: &[u8], offset: usize, label: &str) -> Result<u32, SubsetError> {
    checked_range(data, offset, 4, label)?;
    Ok(u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn usize_from_u32(value: u32, label: &str) -> Result<usize, SubsetError> {
    usize::try_from(value).map_err(|_| invalid(format!("COLR {label} is too large")))
}

fn write_u16(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn write_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn absolute_offset(
    data: &[u8],
    base: usize,
    relative: u32,
    label: &str,
) -> Result<usize, SubsetError> {
    if relative == 0 {
        return Err(invalid(format!("COLR {label} offset is null")));
    }
    let offset = base
        .checked_add(usize_from_u32(relative, label)?)
        .ok_or_else(|| invalid(format!("COLR {label} offset overflows")))?;
    if offset <= base || offset >= data.len() {
        return Err(invalid(format!("COLR {label} offset is outside the table")));
    }
    Ok(offset)
}

fn optional_table_offset(
    data: &[u8],
    raw_offset: u32,
    label: &str,
) -> Result<Option<usize>, SubsetError> {
    if raw_offset == 0 {
        return Ok(None);
    }
    let offset = usize_from_u32(raw_offset, label)?;
    if offset >= data.len() {
        return Err(invalid(format!("COLR {label} offset is outside the table")));
    }
    Ok(Some(offset))
}

impl ColrHeader {
    fn parse(table: &[u8]) -> Result<Self, SubsetError> {
        checked_range(table, 0, COLR_V0_HEADER_LEN, "header")?;
        let version = read_u16(table, 0, "version")?;
        if version > 1 {
            return Err(invalid(format!("unsupported COLR table version {version}")));
        }
        if version == 1 {
            checked_range(table, 0, COLR_V1_HEADER_LEN, "v1 header")?;
        }

        let num_base_records = usize::from(read_u16(table, 2, "base record count")?);
        let base_records_offset = usize_from_u32(
            read_u32(table, 4, "base records offset")?,
            "base records offset",
        )?;
        let layer_records_offset = usize_from_u32(
            read_u32(table, 8, "layer records offset")?,
            "layer records offset",
        )?;
        let num_layer_records = usize::from(read_u16(table, 12, "layer record count")?);

        let (base_glyph_list_offset, layer_list_offset, clip_list_offset) = if version == 1 {
            let base_list = optional_table_offset(
                table,
                read_u32(table, 14, "BaseGlyphList offset")?,
                "BaseGlyphList",
            )?
            .ok_or_else(|| invalid("COLR v1 BaseGlyphList offset is null"))?;
            (
                Some(base_list),
                optional_table_offset(
                    table,
                    read_u32(table, 18, "LayerList offset")?,
                    "LayerList",
                )?,
                optional_table_offset(table, read_u32(table, 22, "ClipList offset")?, "ClipList")?,
            )
        } else {
            (None, None, None)
        };
        if version == 1
            && [base_glyph_list_offset, layer_list_offset, clip_list_offset]
                .into_iter()
                .flatten()
                .any(|offset| offset < COLR_V1_HEADER_LEN)
        {
            return Err(invalid("COLR v1 subtable overlaps the table header"));
        }

        Ok(Self {
            version,
            num_base_records,
            base_records_offset,
            layer_records_offset,
            num_layer_records,
            base_glyph_list_offset,
            layer_list_offset,
            clip_list_offset,
        })
    }
}

fn parse_v0_records(
    table: &[u8],
    header: ColrHeader,
) -> Result<(Vec<BaseGlyphRecord>, Vec<LayerRecord>), SubsetError> {
    if header.num_base_records == 0 && header.num_layer_records == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    let base_bytes = header
        .num_base_records
        .checked_mul(6)
        .ok_or_else(|| invalid("COLR base record count overflows"))?;
    let layer_bytes = header
        .num_layer_records
        .checked_mul(4)
        .ok_or_else(|| invalid("COLR layer record count overflows"))?;
    let header_len = if header.version == 1 {
        COLR_V1_HEADER_LEN
    } else {
        COLR_V0_HEADER_LEN
    };
    if (header.num_base_records != 0 && header.base_records_offset < header_len)
        || (header.num_layer_records != 0 && header.layer_records_offset < header_len)
    {
        return Err(invalid("COLR v0 record data overlaps the table header"));
    }
    checked_range(
        table,
        header.base_records_offset,
        base_bytes,
        "base records",
    )?;
    checked_range(
        table,
        header.layer_records_offset,
        layer_bytes,
        "layer records",
    )?;
    let base_end = header.base_records_offset + base_bytes;
    let layer_end = header.layer_records_offset + layer_bytes;
    if header.num_base_records != 0
        && header.num_layer_records != 0
        && header.base_records_offset < layer_end
        && header.layer_records_offset < base_end
    {
        return Err(invalid("COLR v0 base and layer records overlap"));
    }

    let mut bases = Vec::with_capacity(header.num_base_records);
    for index in 0..header.num_base_records {
        let offset = header.base_records_offset + index * 6;
        bases.push(BaseGlyphRecord {
            gid: read_u16(table, offset, "base glyph ID")?,
            first_layer_index: usize::from(read_u16(table, offset + 2, "first layer index")?),
            num_layers: usize::from(read_u16(table, offset + 4, "layer count")?),
        });
    }
    if !bases.windows(2).all(|pair| pair[0].gid < pair[1].gid) {
        return Err(invalid("COLR base glyph records are not strictly sorted"));
    }

    let mut layers = Vec::with_capacity(header.num_layer_records);
    for index in 0..header.num_layer_records {
        let offset = header.layer_records_offset + index * 4;
        layers.push(LayerRecord {
            gid: read_u16(table, offset, "layer glyph ID")?,
            palette_index: read_u16(table, offset + 2, "layer palette index")?,
        });
    }

    for base in &bases {
        let end = base
            .first_layer_index
            .checked_add(base.num_layers)
            .ok_or_else(|| invalid("COLR layer range overflows"))?;
        if end > layers.len() {
            return Err(invalid(
                "COLR base glyph references layers outside the layer records",
            ));
        }
    }

    Ok((bases, layers))
}

fn parse_base_glyph_list(
    table: &[u8],
    offset: usize,
) -> Result<Vec<BaseGlyphPaintRecord>, SubsetError> {
    let count = usize_from_u32(
        read_u32(table, offset, "BaseGlyphList record count")?,
        "BaseGlyphList record count",
    )?;
    let records_offset = offset
        .checked_add(4)
        .ok_or_else(|| invalid("COLR BaseGlyphList records offset overflows"))?;
    let records_len = count
        .checked_mul(6)
        .ok_or_else(|| invalid("COLR BaseGlyphList record count overflows"))?;
    checked_range(table, records_offset, records_len, "BaseGlyphList records")?;
    let records_end = records_offset + records_len;

    let mut records = Vec::with_capacity(count);
    for index in 0..count {
        let record_offset = records_offset + index * 6;
        let relative_paint_offset = read_u32(
            table,
            record_offset + 2,
            "BaseGlyphPaintRecord paint offset",
        )?;
        let paint_offset = absolute_offset(
            table,
            offset,
            relative_paint_offset,
            "BaseGlyphPaintRecord paint",
        )?;
        if paint_offset < records_end {
            return Err(invalid(
                "COLR BaseGlyphPaintRecord paint overlaps the record array",
            ));
        }
        records.push(BaseGlyphPaintRecord {
            gid: read_u16(table, record_offset, "BaseGlyphPaintRecord glyph ID")?,
            paint_offset,
            relative_paint_offset,
        });
    }
    if !records.windows(2).all(|pair| pair[0].gid < pair[1].gid) {
        return Err(invalid(
            "COLR BaseGlyphPaintRecords are not strictly sorted",
        ));
    }

    Ok(records)
}

fn parse_layer_list(table: &[u8], offset: Option<usize>) -> Result<Vec<usize>, SubsetError> {
    let Some(offset) = offset else {
        return Ok(Vec::new());
    };
    let count = usize_from_u32(
        read_u32(table, offset, "LayerList layer count")?,
        "LayerList layer count",
    )?;
    let offsets_start = offset
        .checked_add(4)
        .ok_or_else(|| invalid("COLR LayerList offsets overflow"))?;
    checked_range(
        table,
        offsets_start,
        count
            .checked_mul(4)
            .ok_or_else(|| invalid("COLR LayerList count overflows"))?,
        "LayerList offsets",
    )?;
    let offsets_end = offsets_start + count * 4;

    (0..count)
        .map(|index| {
            let paint_offset = absolute_offset(
                table,
                offset,
                read_u32(table, offsets_start + index * 4, "LayerList paint offset")?,
                "LayerList paint",
            )?;
            if paint_offset < offsets_end {
                return Err(invalid("COLR LayerList paint overlaps the offset array"));
            }
            Ok(paint_offset)
        })
        .collect()
}

fn paint_size(format: u8) -> Option<usize> {
    match format {
        1 => Some(6),
        2 => Some(5),
        3 => Some(9),
        4 | 6 => Some(16),
        5 | 7 => Some(20),
        8 | 15 | 17 | 18 | 29 => Some(12),
        9 | 19 | 31 => Some(16),
        10 | 20 | 24 => Some(6),
        11 => Some(3),
        12 | 13 => Some(7),
        14 | 16 | 28 | 32 => Some(8),
        21 | 22 | 25 | 26 => Some(10),
        23 | 27 => Some(14),
        30 => Some(12),
        _ => None,
    }
}

impl PaintGraph<'_> {
    fn parse(table: &[u8], header: ColrHeader) -> Result<PaintGraph<'_>, SubsetError> {
        let base_records = parse_base_glyph_list(
            table,
            header
                .base_glyph_list_offset
                .ok_or_else(|| invalid("COLR v1 BaseGlyphList is missing"))?,
        )?;
        let base_roots = base_records
            .iter()
            .map(|record| (record.gid, record.paint_offset))
            .collect();

        Ok(PaintGraph {
            table,
            base_roots,
            layer_offsets: parse_layer_list(table, header.layer_list_offset)?,
        })
    }

    fn direct_child(
        &self,
        paint_offset: usize,
        field_offset: usize,
        parent_size: usize,
    ) -> Result<usize, SubsetError> {
        let child = absolute_offset(
            self.table,
            paint_offset,
            read_u24(
                self.table,
                paint_offset + field_offset,
                "Paint child offset",
            )?,
            "Paint child",
        )?;
        if child < paint_offset + parent_size {
            return Err(invalid("COLR Paint child overlaps its parent table"));
        }
        Ok(child)
    }

    fn validate_subtable_offset(
        &self,
        paint_offset: usize,
        field_offset: usize,
        parent_size: usize,
        label: &str,
    ) -> Result<usize, SubsetError> {
        let subtable = absolute_offset(
            self.table,
            paint_offset,
            read_u24(self.table, paint_offset + field_offset, label)?,
            label,
        )?;
        if subtable < paint_offset + parent_size {
            return Err(invalid(format!("COLR {label} overlaps its parent table")));
        }
        Ok(subtable)
    }

    fn validate_color_line(&self, offset: usize, variable: bool) -> Result<(), SubsetError> {
        let stop_count = usize::from(read_u16(self.table, offset + 1, "ColorLine stop count")?);
        let stop_size = if variable { 10 } else { 6 };
        let size = stop_count
            .checked_mul(stop_size)
            .and_then(|stops| stops.checked_add(3))
            .ok_or_else(|| invalid("COLR ColorLine size overflows"))?;
        checked_range(self.table, offset, size, "ColorLine")
    }

    fn node(&self, paint_offset: usize) -> Result<PaintNode, SubsetError> {
        let format = read_u8(self.table, paint_offset, "Paint format")?;
        let Some(size) = paint_size(format) else {
            return Ok(PaintNode {
                children: Vec::new(),
                glyph_references: Vec::new(),
                known_format: false,
            });
        };
        checked_range(self.table, paint_offset, size, "Paint table")?;

        let mut children = Vec::new();
        let mut glyph_references = Vec::new();
        match format {
            1 => {
                let count = usize::from(read_u8(
                    self.table,
                    paint_offset + 1,
                    "PaintColrLayers layer count",
                )?);
                let first = usize_from_u32(
                    read_u32(
                        self.table,
                        paint_offset + 2,
                        "PaintColrLayers first layer index",
                    )?,
                    "PaintColrLayers first layer index",
                )?;
                let end = first
                    .checked_add(count)
                    .ok_or_else(|| invalid("COLR PaintColrLayers range overflows"))?;
                if end > self.layer_offsets.len() {
                    return Err(invalid(
                        "COLR PaintColrLayers references outside the LayerList",
                    ));
                }
                children.extend_from_slice(&self.layer_offsets[first..end]);
            }
            2 | 3 => {}
            4..=9 => {
                let color_line =
                    self.validate_subtable_offset(paint_offset, 1, size, "Paint color line")?;
                self.validate_color_line(color_line, !format.is_multiple_of(2))?;
            }
            10 => {
                children.push(self.direct_child(paint_offset, 1, size)?);
                glyph_references.push((
                    paint_offset + 4,
                    read_u16(self.table, paint_offset + 4, "PaintGlyph glyph ID")?,
                ));
            }
            11 => {
                let gid = read_u16(self.table, paint_offset + 1, "PaintColrGlyph glyph ID")?;
                let root = self.base_roots.get(&gid).copied().ok_or_else(|| {
                    invalid("COLR PaintColrGlyph references a missing base glyph")
                })?;
                glyph_references.push((paint_offset + 1, gid));
                children.push(root);
            }
            12 | 13 => {
                children.push(self.direct_child(paint_offset, 1, size)?);
                let transform =
                    self.validate_subtable_offset(paint_offset, 4, size, "Paint transform")?;
                checked_range(
                    self.table,
                    transform,
                    if format == 12 { 24 } else { 28 },
                    "Paint transform",
                )?;
            }
            14..=31 => children.push(self.direct_child(paint_offset, 1, size)?),
            32 => {
                children.push(self.direct_child(paint_offset, 1, size)?);
                children.push(self.direct_child(paint_offset, 5, size)?);
            }
            _ => unreachable!(),
        }

        Ok(PaintNode {
            children,
            glyph_references,
            known_format: true,
        })
    }

    fn walk<F>(
        &self,
        roots: impl IntoIterator<Item = usize>,
        reject_unknown: bool,
        completed: &mut HashSet<usize>,
        mut on_glyph: F,
    ) -> Result<(), SubsetError>
    where
        F: FnMut(usize, u16) -> Result<(), SubsetError>,
    {
        for root in roots {
            self.walk_node(
                root,
                0,
                reject_unknown,
                &mut HashSet::new(),
                completed,
                &mut on_glyph,
            )?;
        }
        Ok(())
    }

    fn walk_node<F>(
        &self,
        offset: usize,
        depth: usize,
        reject_unknown: bool,
        path: &mut HashSet<usize>,
        completed: &mut HashSet<usize>,
        on_glyph: &mut F,
    ) -> Result<(), SubsetError>
    where
        F: FnMut(usize, u16) -> Result<(), SubsetError>,
    {
        if completed.contains(&offset) {
            return Ok(());
        }
        if depth > MAX_PAINT_DEPTH {
            return Err(invalid("COLR paint graph exceeds the nesting limit"));
        }
        if !path.insert(offset) {
            return Err(invalid("COLR paint graph contains a cycle"));
        }

        let node = self.node(offset)?;
        if reject_unknown && !node.known_format {
            return Err(SubsetError::Unsupported(
                "COLR paint format newer than OpenType COLR v1",
            ));
        }
        for (field_offset, gid) in node.glyph_references {
            on_glyph(field_offset, gid)?;
        }
        for child in node.children {
            self.walk_node(child, depth + 1, reject_unknown, path, completed, on_glyph)?;
        }

        path.remove(&offset);
        completed.insert(offset);
        Ok(())
    }
}

/// Expand `glyphs` with every outline and reusable color glyph referenced by
/// selected COLR v0/v1 base glyphs.
pub fn expand_glyph_set(table: &[u8], glyphs: &mut BTreeSet<u16>) -> Result<(), SubsetError> {
    let header = ColrHeader::parse(table)?;
    let (base_records, layer_records) = parse_v0_records(table, header)?;
    let graph = (header.version == 1)
        .then(|| PaintGraph::parse(table, header))
        .transpose()?;
    let mut pending = glyphs.iter().copied().collect::<VecDeque<_>>();
    let mut processed_base_gids = HashSet::with_capacity(pending.len());
    let mut completed_paints = HashSet::new();

    while let Some(gid) = pending.pop_front() {
        if !processed_base_gids.insert(gid) {
            continue;
        }

        if let Ok(index) = base_records.binary_search_by_key(&gid, |base| base.gid) {
            let base = base_records[index];
            let end = base.first_layer_index + base.num_layers;
            for layer in &layer_records[base.first_layer_index..end] {
                if glyphs.insert(layer.gid) {
                    pending.push_back(layer.gid);
                }
            }
        }

        if let Some(graph) = &graph {
            if let Some(root) = graph.base_roots.get(&gid).copied() {
                graph.walk([root], false, &mut completed_paints, |_, referenced_gid| {
                    if glyphs.insert(referenced_gid) {
                        pending.push_back(referenced_gid);
                    }
                    Ok(())
                })?;
            }
        }
    }

    Ok(())
}

fn rewrite_v0_records(
    table: &[u8],
    output: &mut [u8],
    header: ColrHeader,
    gid_remap: &HashMap<u16, u16>,
) -> Result<(), SubsetError> {
    let (base_records, layer_records) = parse_v0_records(table, header)?;
    let mut new_bases = Vec::new();
    let mut new_layers = Vec::new();

    for base in base_records {
        let Some(&new_gid) = gid_remap.get(&base.gid) else {
            continue;
        };
        let first_layer = u16::try_from(new_layers.len())
            .map_err(|_| invalid("COLR subset has too many layer records"))?;
        let end = base.first_layer_index + base.num_layers;
        for layer in &layer_records[base.first_layer_index..end] {
            if let Some(&layer_gid) = gid_remap.get(&layer.gid) {
                new_layers.push(LayerRecord {
                    gid: layer_gid,
                    palette_index: layer.palette_index,
                });
            }
        }
        let num_layers = u16::try_from(new_layers.len() - usize::from(first_layer))
            .map_err(|_| invalid("COLR base glyph has too many layers"))?;
        if num_layers != 0 {
            new_bases.push((new_gid, first_layer, num_layers));
        }
    }
    new_bases.sort_unstable_by_key(|record| record.0);

    write_u16(
        output,
        2,
        u16::try_from(new_bases.len())
            .map_err(|_| invalid("COLR subset has too many base glyph records"))?,
    );
    write_u16(
        output,
        12,
        u16::try_from(new_layers.len())
            .map_err(|_| invalid("COLR subset has too many layer records"))?,
    );
    for (index, (gid, first_layer, num_layers)) in new_bases.iter().enumerate() {
        let offset = header.base_records_offset + index * 6;
        write_u16(output, offset, *gid);
        write_u16(output, offset + 2, *first_layer);
        write_u16(output, offset + 4, *num_layers);
    }
    for (index, layer) in new_layers.iter().enumerate() {
        let offset = header.layer_records_offset + index * 4;
        write_u16(output, offset, layer.gid);
        write_u16(output, offset + 2, layer.palette_index);
    }

    Ok(())
}

fn rewrite_clip_list(
    table: &[u8],
    output: &mut [u8],
    header: ColrHeader,
    retained_base_gids: &BTreeSet<u16>,
    gid_remap: &HashMap<u16, u16>,
) -> Result<(), SubsetError> {
    let Some(offset) = header.clip_list_offset else {
        return Ok(());
    };
    if read_u8(table, offset, "ClipList format")? != 1 {
        return Err(invalid("unsupported COLR ClipList format"));
    }
    let count = usize_from_u32(
        read_u32(table, offset + 1, "ClipList record count")?,
        "ClipList record count",
    )?;
    let records_offset = offset
        .checked_add(5)
        .ok_or_else(|| invalid("COLR ClipList records offset overflows"))?;
    checked_range(
        table,
        records_offset,
        count
            .checked_mul(7)
            .ok_or_else(|| invalid("COLR ClipList record count overflows"))?,
        "ClipList records",
    )?;

    let mut rewritten = Vec::with_capacity(count);
    let mut previous_end = None;
    for index in 0..count {
        let record_offset = records_offset + index * 7;
        let start = read_u16(table, record_offset, "Clip start glyph ID")?;
        let end = read_u16(table, record_offset + 2, "Clip end glyph ID")?;
        if start > end || previous_end.is_some_and(|previous| start <= previous) {
            return Err(invalid("COLR ClipList ranges overlap or are not sorted"));
        }
        previous_end = Some(end);
        let relative_clip_box = read_u24(table, record_offset + 4, "ClipBox offset")?;
        let clip_box = absolute_offset(table, offset, relative_clip_box, "ClipBox")?;
        let clip_box_size = match read_u8(table, clip_box, "ClipBox format")? {
            1 => 9,
            2 => 13,
            _ => return Err(invalid("unsupported COLR ClipBox format")),
        };
        checked_range(table, clip_box, clip_box_size, "ClipBox")?;

        let mut mapped = retained_base_gids
            .range(start..=end)
            .map(|gid| {
                gid_remap
                    .get(gid)
                    .copied()
                    .ok_or_else(|| invalid("COLR retained base glyph has no GID mapping"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if mapped.is_empty() {
            continue;
        }
        mapped.sort_unstable();
        if !mapped
            .windows(2)
            .all(|pair| pair[0].checked_add(1) == Some(pair[1]))
        {
            write_u32(output, 22, 0);
            return Ok(());
        }
        rewritten.push((
            mapped[0],
            *mapped.last().expect("mapped is non-empty"),
            relative_clip_box,
        ));
    }

    if rewritten.is_empty() {
        write_u32(output, 22, 0);
        return Ok(());
    }
    write_u32(
        output,
        offset + 1,
        u32::try_from(rewritten.len())
            .map_err(|_| invalid("COLR subset has too many ClipList records"))?,
    );
    for (index, (start, end, relative_clip_box)) in rewritten.iter().enumerate() {
        let record_offset = records_offset + index * 7;
        write_u16(output, record_offset, *start);
        write_u16(output, record_offset + 2, *end);
        let bytes = relative_clip_box.to_be_bytes();
        output[record_offset + 4..record_offset + 7].copy_from_slice(&bytes[1..]);
    }

    Ok(())
}

fn rewrite_v1(
    table: &[u8],
    output: &mut [u8],
    header: ColrHeader,
    gid_remap: &HashMap<u16, u16>,
) -> Result<(), SubsetError> {
    let base_list_offset = header
        .base_glyph_list_offset
        .ok_or_else(|| invalid("COLR v1 BaseGlyphList is missing"))?;
    let base_records = parse_base_glyph_list(table, base_list_offset)?;
    let mut retained = base_records
        .iter()
        .filter_map(|record| {
            gid_remap
                .get(&record.gid)
                .copied()
                .map(|new_gid| (new_gid, *record))
        })
        .collect::<Vec<_>>();
    retained.sort_unstable_by_key(|record| record.0);
    let retained_old_gids = retained
        .iter()
        .map(|(_, record)| record.gid)
        .collect::<BTreeSet<_>>();

    write_u32(
        output,
        base_list_offset,
        u32::try_from(retained.len())
            .map_err(|_| invalid("COLR subset has too many BaseGlyphPaintRecords"))?,
    );
    for (index, (new_gid, record)) in retained.iter().enumerate() {
        let record_offset = base_list_offset + 4 + index * 6;
        write_u16(output, record_offset, *new_gid);
        write_u32(output, record_offset + 2, record.relative_paint_offset);
    }

    let graph = PaintGraph::parse(table, header)?;
    let mut completed_paints = HashSet::new();
    graph.walk(
        retained.iter().map(|(_, record)| record.paint_offset),
        true,
        &mut completed_paints,
        |field_offset, old_gid| {
            let new_gid = gid_remap.get(&old_gid).copied().ok_or_else(|| {
                invalid("COLR paint graph references a glyph omitted from the subset closure")
            })?;
            write_u16(output, field_offset, new_gid);
            Ok(())
        },
    )?;
    rewrite_clip_list(table, output, header, &retained_old_gids, gid_remap)
}

/// Rewrite a COLR v0 or v1 table for the supplied old-to-new GID mapping.
///
/// Call [`expand_glyph_set`] before constructing `gid_remap`; this ensures all
/// glyphs reachable through v0 layers and v1 paint graphs receive a mapping.
pub fn rewrite_colr(table: &[u8], gid_remap: &HashMap<u16, u16>) -> Result<Vec<u8>, SubsetError> {
    let header = ColrHeader::parse(table)?;
    let mut output = table.to_vec();
    rewrite_v0_records(table, &mut output, header, gid_remap)?;
    if header.version == 1 {
        rewrite_v1(table, &mut output, header, gid_remap)?;
    }
    Ok(output)
}
