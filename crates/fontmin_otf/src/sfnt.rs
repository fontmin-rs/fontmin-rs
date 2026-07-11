//! Static CFF sfnt validation and table selection.

use std::collections::BTreeMap;

use allsorts::{
    binary::read::ReadScope,
    layout::{GDEFTable, GPOS, GSUB, LayoutTable},
};
use fontmin_diagnostics::{FontminError, Result};
use fontmin_ttf::OwnedSfntTable;

use crate::glyf::TrueTypeOutlineTables;

const SFNT_HEADER_SIZE: usize = 12;
const SFNT_TABLE_RECORD_SIZE: usize = 16;
const REQUIRED_TABLES: &[&str] = &[
    "cmap", "head", "hhea", "hmtx", "maxp", "name", "OS/2", "post",
];
const PRESERVED_TABLES: &[&str] = &[
    "cmap", "name", "OS/2", "post", "kern", "GDEF", "GSUB", "GPOS", "BASE", "JSTF",
];
const REJECTED_TABLES: &[&str] = &["COLR", "CPAL", "CBDT", "CBLC", "sbix", "SVG "];
const VARIABLE_TABLES: &[&str] = &[
    "fvar", "avar", "HVAR", "VVAR", "MVAR", "gvar", "cvar", "cvt ",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutlineFormat {
    Cff,
    Cff2,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct HorizontalMetric {
    pub advance_width: u16,
    pub left_side_bearing: i16,
}

pub(crate) struct StaticCffSource<'a> {
    pub tables: BTreeMap<String, &'a [u8]>,
    pub num_glyphs: u16,
    pub metrics: Vec<HorizontalMetric>,
    pub outline_format: OutlineFormat,
}

impl<'a> StaticCffSource<'a> {
    pub fn table(&self, tag: &str) -> &'a [u8] {
        self.tables[tag]
    }
}

pub(crate) fn read_static_cff_source(input: &[u8]) -> Result<StaticCffSource<'_>> {
    let source = read_cff_source(input)?;

    if VARIABLE_TABLES
        .iter()
        .any(|tag| source.tables.contains_key(*tag))
    {
        return Err(FontminError::unsupported(
            "variable tables must be instanced before CFF conversion",
        ));
    }

    Ok(source)
}

pub(crate) fn read_cff_source(input: &[u8]) -> Result<StaticCffSource<'_>> {
    if !input.starts_with(b"OTTO") {
        return Err(FontminError::invalid_font(
            "expected OpenType sfnt data for OTF conversion",
        ));
    }

    let records = fontmin_ttf::read_sfnt_table_directory(input)?;
    let directory_end = SFNT_HEADER_SIZE
        .checked_add(
            records
                .len()
                .checked_mul(SFNT_TABLE_RECORD_SIZE)
                .ok_or_else(|| FontminError::invalid_font("OTF table directory is too large"))?,
        )
        .ok_or_else(|| FontminError::invalid_font("OTF table directory is too large"))?;
    let mut ranges = Vec::new();
    let mut tables = BTreeMap::new();

    for record in records {
        if record.length > 0 {
            if !record.offset.is_multiple_of(4) {
                return Err(FontminError::invalid_font(format!(
                    "OTF table {} is not four-byte aligned",
                    record.tag
                )));
            }
            if record.offset < directory_end {
                return Err(FontminError::invalid_font(format!(
                    "OTF table {} starts inside the table directory",
                    record.tag
                )));
            }

            let end = record.offset.checked_add(record.length).ok_or_else(|| {
                FontminError::invalid_font(format!("OTF table {} range overflows", record.tag))
            })?;
            ranges.push((record.offset, end, record.tag.clone()));
        }

        let table = input
            .get(record.offset..record.offset + record.length)
            .ok_or_else(|| {
                FontminError::invalid_font(format!(
                    "OTF table {} points outside the file",
                    record.tag
                ))
            })?;
        tables.insert(record.tag, table);
    }

    ranges.sort_unstable_by_key(|(start, _, _)| *start);
    for pair in ranges.windows(2) {
        let (_, previous_end, previous_tag) = &pair[0];
        let (next_start, _, next_tag) = &pair[1];
        if previous_end > next_start {
            return Err(FontminError::invalid_font(format!(
                "OTF tables {previous_tag} and {next_tag} overlap",
            )));
        }
    }

    for tag in REJECTED_TABLES {
        if tables.contains_key(*tag) {
            return Err(FontminError::unsupported(*tag));
        }
    }
    if tables.contains_key("glyf") || tables.contains_key("loca") {
        return Err(FontminError::unsupported("mixed CFF and glyf outlines"));
    }
    let outline_format = match (tables.contains_key("CFF "), tables.contains_key("CFF2")) {
        (true, false) => OutlineFormat::Cff,
        (false, true) => OutlineFormat::Cff2,
        (true, true) => {
            return Err(FontminError::unsupported("mixed CFF and CFF2 outlines"));
        }
        (false, false) => {
            return Err(FontminError::invalid_font(
                "missing CFF or CFF2 outline table",
            ));
        }
    };

    for tag in REQUIRED_TABLES {
        if !tables.contains_key(*tag) {
            return Err(FontminError::invalid_font(format!(
                "missing required CFF OTF table {tag}",
            )));
        }
    }

    let num_glyphs = read_u16(tables["maxp"], 4, "maxp")?;
    let num_h_metrics = usize::from(read_u16(tables["hhea"], 34, "hhea")?);
    if num_h_metrics == 0 || num_h_metrics > usize::from(num_glyphs) {
        return Err(FontminError::invalid_font(
            "hhea numberOfHMetrics is outside the glyph range",
        ));
    }
    let hmtx = tables["hmtx"];
    let required_hmtx_len = num_h_metrics
        .checked_mul(4)
        .and_then(|length| {
            length.checked_add((usize::from(num_glyphs) - num_h_metrics).checked_mul(2)?)
        })
        .ok_or_else(|| FontminError::invalid_font("hmtx table is too large"))?;
    if hmtx.len() < required_hmtx_len {
        return Err(FontminError::invalid_font("hmtx table is truncated"));
    }

    let mut metrics = Vec::with_capacity(usize::from(num_glyphs));
    let mut last_advance = 0;
    for glyph_id in 0..usize::from(num_glyphs) {
        let metric = if glyph_id < num_h_metrics {
            let offset = glyph_id * 4;
            last_advance = read_u16(hmtx, offset, "hmtx")?;
            HorizontalMetric {
                advance_width: last_advance,
                left_side_bearing: read_i16(hmtx, offset + 2, "hmtx")?,
            }
        } else {
            let offset = num_h_metrics * 4 + (glyph_id - num_h_metrics) * 2;
            HorizontalMetric {
                advance_width: last_advance,
                left_side_bearing: read_i16(hmtx, offset, "hmtx")?,
            }
        };
        metrics.push(metric);
    }

    Ok(StaticCffSource {
        tables,
        num_glyphs,
        metrics,
        outline_format,
    })
}

pub(crate) fn validate_cff2_layout_tables(source: &StaticCffSource<'_>) -> Result<bool> {
    if source.outline_format != OutlineFormat::Cff2 {
        return Ok(false);
    }

    let mut drop_gdef = false;
    if let Some(data) = source.tables.get("GDEF") {
        let gdef = ReadScope::new(data)
            .read::<GDEFTable>()
            .map_err(|error| FontminError::invalid_font(format!("invalid GDEF table: {error}")))?;
        if gdef.opt_item_variation_store.is_some() {
            drop_gdef = true;
        }
    }

    if let Some(data) = source.tables.get("GSUB") {
        let gsub = ReadScope::new(data)
            .read::<LayoutTable<GSUB>>()
            .map_err(|error| FontminError::invalid_font(format!("invalid GSUB table: {error}")))?;
        if gsub.opt_feature_variations.is_some() {
            return Err(FontminError::unsupported(
                "CFF2 GSUB FeatureVariations layout data",
            ));
        }
    }

    if let Some(data) = source.tables.get("GPOS") {
        let gpos = ReadScope::new(data)
            .read::<LayoutTable<GPOS>>()
            .map_err(|error| FontminError::invalid_font(format!("invalid GPOS table: {error}")))?;
        if gpos.opt_feature_variations.is_some() {
            return Err(FontminError::unsupported(
                "CFF2 GPOS FeatureVariations layout data",
            ));
        }
    }

    Ok(drop_gdef)
}

pub(crate) fn output_tables(
    source: &StaticCffSource<'_>,
    original: Option<&StaticCffSource<'_>>,
    drop_gdef: bool,
    outlines: TrueTypeOutlineTables,
) -> Vec<OwnedSfntTable> {
    let mut tables = Vec::new();

    for tag in PRESERVED_TABLES {
        if drop_gdef && *tag == "GDEF" {
            continue;
        }
        let data = original
            .and_then(|font| font.tables.get(*tag))
            .or_else(|| source.tables.get(*tag));
        if let Some(data) = data {
            tables.push(OwnedSfntTable {
                tag: (*tag).into(),
                data: (*data).to_vec(),
            });
        }
    }

    tables.extend([
        OwnedSfntTable {
            tag: "head".into(),
            data: outlines.head,
        },
        OwnedSfntTable {
            tag: "hhea".into(),
            data: outlines.hhea,
        },
        OwnedSfntTable {
            tag: "hmtx".into(),
            data: outlines.hmtx,
        },
        OwnedSfntTable {
            tag: "maxp".into(),
            data: outlines.maxp,
        },
        OwnedSfntTable {
            tag: "glyf".into(),
            data: outlines.glyf,
        },
        OwnedSfntTable {
            tag: "loca".into(),
            data: outlines.loca,
        },
    ]);

    tables
}

fn read_u16(table: &[u8], offset: usize, tag: &str) -> Result<u16> {
    let bytes = table
        .get(offset..offset + 2)
        .ok_or_else(|| FontminError::invalid_font(format!("{tag} table is truncated")))?;

    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_i16(table: &[u8], offset: usize, tag: &str) -> Result<i16> {
    let bytes = table
        .get(offset..offset + 2)
        .ok_or_else(|| FontminError::invalid_font(format!("{tag} table is truncated")))?;

    Ok(i16::from_be_bytes([bytes[0], bytes[1]]))
}
