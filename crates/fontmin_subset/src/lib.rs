use font_subset::{Font, FontReader};
use skrifa::{
    FontRef as SkrifaFontRef, MetadataProvider,
    raw::{
        TableProvider,
        tables::cmap::{CmapIterLimits, CmapSubtable, PlatformId},
    },
};
use std::collections::{BTreeMap, BTreeSet};

use fontmin_core::{
    CoverageOptions, CoverageReport, MissingGlyphPolicy, UnicodeRange, collect_chars_with_ranges,
};
use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};

mod variation;

pub use variation::{
    AxisRange, AxisSetting, InstanceOptions, VariationSpaceOptions, reduce_variation_space,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutSubsetMode {
    /// Remove `GDEF`, `GPOS`, and `GSUB`.
    Drop,
    /// Remap supported layout data and discard subtables that no longer match.
    Conservative,
    /// Remap supported layout data and reject known contextual or variation loss.
    Preserve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct SubsetOptions {
    pub text: Option<String>,
    pub unicodes: Vec<u32>,
    pub unicode_ranges: Vec<UnicodeRange>,
    /// Explicit original glyph IDs to retain in addition to Unicode selection.
    pub gids: Vec<u16>,
    /// PostScript glyph names to retain in addition to other selectors.
    pub glyph_names: Vec<String>,
    pub basic_text: bool,
    /// Retain the `cvt `, `fpgm`, and `prep` TrueType program tables.
    pub preserve_hinting: bool,
    /// Subset glyph data; `false` returns the validated source bytes unchanged.
    pub trim: bool,
    /// Retain the original glyph-zero outline instead of an empty required slot.
    pub keep_notdef: bool,
    /// Preserve original glyph IDs and emit empty slots for unselected IDs.
    pub retain_gids: bool,
    /// Retain PostScript glyph names in a rewritten version 2 `post` table.
    pub retain_glyph_names: bool,
    /// Retain non-Unicode, non-symbol cmap encoding records after GID remapping.
    pub retain_legacy_cmap: bool,
    /// Retain the Windows symbol cmap encoding record after GID remapping.
    pub retain_symbol_cmap: bool,
    /// Control OpenType layout-table retention and remapping.
    pub layout: LayoutSubsetMode,
    /// Four-byte OpenType feature tags to retain, or all features when empty.
    pub layout_features: Vec<String>,
    /// Four-byte OpenType script tags to retain, or all scripts when empty.
    pub layout_scripts: Vec<String>,
    /// OpenType language tags to retain, or all languages when empty. The
    /// special value `default` selects each script's `DefaultLangSys`.
    pub layout_languages: Vec<String>,
    /// OpenType `name` IDs to retain, or all name IDs when empty.
    pub name_ids: Vec<u16>,
    /// Platform-specific `name` language IDs to retain, or all languages when
    /// empty. This filter is combined with `name_ids` using AND semantics.
    pub name_languages: Vec<u16>,
    /// Optional OpenType tables to remove after subsetting. Required outline,
    /// metrics, mapping, and naming tables cannot be removed.
    pub drop_tables: Vec<String>,
    /// Optional source tables to copy verbatim into the subset. Tables already
    /// rewritten by the subset engine cannot be overridden this way.
    pub pass_through_tables: Vec<String>,
    pub missing_glyphs: MissingGlyphPolicy,
}

impl Default for SubsetOptions {
    fn default() -> Self {
        Self {
            text: None,
            unicodes: Vec::new(),
            unicode_ranges: Vec::new(),
            gids: Vec::new(),
            glyph_names: Vec::new(),
            basic_text: false,
            preserve_hinting: false,
            trim: true,
            keep_notdef: true,
            retain_gids: false,
            retain_glyph_names: false,
            retain_legacy_cmap: false,
            retain_symbol_cmap: false,
            layout: LayoutSubsetMode::Conservative,
            layout_features: Vec::new(),
            layout_scripts: Vec::new(),
            layout_languages: Vec::new(),
            name_ids: Vec::new(),
            name_languages: Vec::new(),
            drop_tables: Vec::new(),
            pass_through_tables: Vec::new(),
            missing_glyphs: MissingGlyphPolicy::Warn,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GidMapping {
    pub old_gid: u16,
    pub new_gid: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnicodeGidMapping {
    pub unicode: u32,
    pub old_gid: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlyphNameGidMapping {
    pub glyph_name: String,
    pub old_gid: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsetReport {
    pub original_size: usize,
    pub subset_size: usize,
    pub glyphs_retained: u16,
    pub tables_retained: Vec<String>,
    pub dropped_context_subtables: usize,
    pub cff_charstrings_verbatim: bool,
    pub requested_gids: Vec<u16>,
    pub supported_gids: Vec<u16>,
    pub missing_gids: Vec<u16>,
    pub requested_glyph_names: Vec<String>,
    pub supported_glyph_names: Vec<String>,
    pub missing_glyph_names: Vec<String>,
    pub glyph_name_to_old_gid: Vec<GlyphNameGidMapping>,
    pub old_to_new: Vec<GidMapping>,
    pub new_to_old: Vec<Option<u16>>,
    pub unicode_to_old_gid: Vec<UnicodeGidMapping>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsetResult {
    pub data: Vec<u8>,
    pub report: SubsetReport,
}

impl From<&SubsetOptions> for CoverageOptions {
    fn from(options: &SubsetOptions) -> Self {
        Self {
            text: options.text.clone(),
            unicodes: options.unicodes.clone(),
            unicode_ranges: options.unicode_ranges.clone(),
            basic_text: options.basic_text,
        }
    }
}

pub fn analyze_ttf_coverage(input: &[u8], options: &CoverageOptions) -> Result<CoverageReport> {
    let requested = collect_requested(options, "coverage")?;

    with_font(input, |font| {
        let (_, report) = partition_coverage(font, &requested);

        Ok(report)
    })
}

/// Return the sorted Unicode scalar values mapped to non-zero glyphs by a TTF.
pub fn ttf_unicode_codepoints(input: &[u8]) -> Result<Vec<u32>> {
    let font = fontmin_ttf::read_ttf(input)?;
    let cmap = font
        .table("cmap")
        .ok_or_else(|| FontminError::invalid_font("required cmap table is missing"))?;
    let mappings = oxifont_subset::cmap_to_gid_map_pub(cmap)
        .map_err(|error| FontminError::invalid_font(error.to_string()))?;
    let mut code_points = mappings
        .into_iter()
        .filter_map(|(code_point, gid)| {
            (gid != 0 && char::from_u32(code_point).is_some()).then_some(code_point)
        })
        .collect::<Vec<_>>();
    code_points.sort_unstable();
    code_points.dedup();

    Ok(code_points)
}

/// Instantiate every axis of a `glyf`-backed variable TrueType font.
///
/// The result preserves glyph IDs, evaluates outlines and metrics at the
/// requested location, and removes all variation and TrueType hinting tables.
/// Coordinates must use known four-byte axis tags and stay within each axis's
/// declared inclusive range. Axes omitted from `variation_coordinates` are
/// pinned at their `fvar` defaults.
pub fn instantiate_ttf(input: &[u8], options: &InstanceOptions) -> Result<Vec<u8>> {
    let font = fontmin_ttf::read_ttf(input)?;
    let fvar = font
        .table("fvar")
        .ok_or_else(|| FontminError::unsupported("static TrueType font without fvar axes"))?;
    let axes = variation::parse_variation_axes(fvar)?;
    let coordinates =
        variation::validate_variation_coordinates(&axes, &options.variation_coordinates)?;
    let output = oxifont_subset::instance(input, 0, &coordinates)
        .map_err(|error| FontminError::invalid_font(error.to_string()))?;

    fontmin_ttf::inspect_ttf(&output)
        .map_err(|error| FontminError::convert_failed(error.to_string()))?;
    if fontmin_ttf::calculate_table_checksum(&output) != 0xB1B0_AFBA {
        return Err(FontminError::convert_failed(
            "instanced TTF checksum adjustment is invalid",
        ));
    }

    Ok(output)
}

impl SubsetOptions {
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn subset_ttf(input: &[u8], options: SubsetOptions) -> Result<Vec<u8>> {
    Ok(subset_ttf_with_report(input, options)?.data)
}

#[allow(clippy::needless_pass_by_value)]
pub fn subset_ttf_with_report(input: &[u8], options: SubsetOptions) -> Result<SubsetResult> {
    let table_policy = TablePolicy::from_options(&options)?;
    let requested = collect_chars_with_ranges(
        options.text.as_deref(),
        &options.unicodes,
        options.basic_text,
        &options.unicode_ranges,
    )?;

    let requested_glyph_names = options.glyph_names.iter().cloned().collect::<BTreeSet<_>>();
    if requested_glyph_names.iter().any(String::is_empty) {
        return Err(FontminError::config(
            "glyphNames cannot contain an empty name",
        ));
    }

    if requested.is_empty() && options.gids.is_empty() && requested_glyph_names.is_empty() {
        return Err(FontminError::config(
            "subset requires at least one character from text, unicodes, Unicode ranges, or basicText, one glyph ID from gids, or one PostScript name from glyphNames",
        ));
    }
    let layout_selection = LayoutSelection::from_options(&options)?;

    with_font(input, |font| {
        let (chars, coverage) = partition_coverage(font, &requested);

        if options.missing_glyphs == MissingGlyphPolicy::Error {
            coverage.ensure_complete()?;
        }

        let source = fontmin_ttf::read_ttf(input)?;
        let maxp = required_subset_table(&source, "maxp")?;
        let glyph_count = read_u16_at(maxp, 4, "maxp numGlyphs")?;
        let requested_gids = options.gids.iter().copied().collect::<BTreeSet<_>>();
        let (supported_gids, missing_gids): (BTreeSet<_>, BTreeSet<_>) = requested_gids
            .iter()
            .copied()
            .partition(|gid| *gid < glyph_count);

        if options.missing_glyphs == MissingGlyphPolicy::Error && !missing_gids.is_empty() {
            return Err(missing_gid_error(&missing_gids));
        }
        let glyph_name_selection = resolve_glyph_names(input, &requested_glyph_names)?;

        if options.missing_glyphs == MissingGlyphPolicy::Error
            && !glyph_name_selection.missing.is_empty()
        {
            return Err(missing_glyph_name_error(&glyph_name_selection.missing));
        }

        if chars.is_empty() && supported_gids.is_empty() && glyph_name_selection.mappings.is_empty()
        {
            return Err(FontminError::config(
                "subset request has no characters, glyph IDs, or glyph names supported by the input font",
            ));
        }

        let permissions = font.permissions();
        if !permissions.allow_subsetting {
            return Err(FontminError::invalid_font(
                "font license does not allow subsetting",
            ));
        }

        let cmap = required_subset_table(&source, "cmap")?;
        let cmap_to_gid = oxifont_subset::cmap_to_gid_map_pub(cmap)
            .map_err(|error| FontminError::invalid_font(error.to_string()))?;
        let unicode_to_old_gid = chars
            .iter()
            .filter_map(|character| {
                let unicode = u32::from(*character);
                cmap_to_gid
                    .get(&unicode)
                    .copied()
                    .filter(|gid| *gid != 0)
                    .map(|old_gid| (unicode, old_gid))
            })
            .collect::<BTreeMap<_, _>>();

        if !options.trim {
            return Ok(identity_subset_result(
                input,
                &source,
                glyph_count,
                &requested_gids,
                &supported_gids,
                &missing_gids,
                &glyph_name_selection,
                &unicode_to_old_gid,
            ));
        }

        let mut subset_options = oxifont_subset::SubsetOptions::default()
            .strip_hints(!options.preserve_hinting)
            .retain_gids(options.retain_gids)
            .retain_layout_tables(options.layout != LayoutSubsetMode::Drop);
        if let Some(features) = &layout_selection.features {
            subset_options = subset_options.retain_layout_features(features.iter().copied());
        }
        if let Some(scripts) = &layout_selection.scripts {
            subset_options = subset_options.retain_layout_scripts(scripts.iter().copied());
        }
        if let Some(languages) = &layout_selection.languages {
            subset_options = subset_options.retain_layout_languages(
                languages.iter().copied(),
                layout_selection.retain_default_language,
            );
        }
        if !options.name_ids.is_empty() {
            subset_options = subset_options.retain_name_ids(options.name_ids.iter().copied());
        }
        if !options.name_languages.is_empty() {
            subset_options =
                subset_options.retain_name_languages(options.name_languages.iter().copied());
        }
        if options.layout == LayoutSubsetMode::Preserve {
            ensure_layout_can_be_preserved(input)?;
        }
        let mut old_gid_set = supported_gids.clone();
        old_gid_set.extend(
            glyph_name_selection
                .mappings
                .iter()
                .map(|mapping| mapping.old_gid),
        );
        old_gid_set.extend(unicode_to_old_gid.values().copied());
        old_gid_set.insert(0);
        let (output, mut stats, gid_map) = oxifont_subset::subset_with_gid_set_mapped(
            input,
            &old_gid_set,
            &unicode_to_old_gid,
            &subset_options,
        )
        .map_err(|error| FontminError::invalid_font(error.to_string()))?;
        if options.layout == LayoutSubsetMode::Preserve {
            ensure_layout_was_preserved(input, &output, stats.dropped_context_subtables)?;
        }

        let output = apply_notdef_policy(output, options.keep_notdef)?;
        let output =
            apply_glyph_name_policy(output, input, &source, &gid_map, options.retain_glyph_names)?;
        let output = apply_cmap_policy(
            output,
            input,
            &gid_map,
            &unicode_to_old_gid,
            options.retain_legacy_cmap,
            options.retain_symbol_cmap,
        )?;
        let output = apply_table_policy(output, &source, &table_policy, options.retain_gids)?;
        let output_font = fontmin_ttf::read_ttf(&output)?;
        stats.subset_size = output.len();
        stats.tables_retained = output_font
            .tables
            .iter()
            .filter_map(|record| record.tag.as_bytes().try_into().ok())
            .collect();
        let report = subset_report(
            input.len(),
            output.len(),
            &stats,
            &requested_gids,
            &supported_gids,
            &missing_gids,
            &glyph_name_selection,
            &gid_map,
            &unicode_to_old_gid,
        );

        Ok(SubsetResult {
            data: output,
            report,
        })
    })
}

#[derive(Debug, Default)]
struct TablePolicy {
    drop: BTreeSet<[u8; 4]>,
    pass_through: BTreeSet<[u8; 4]>,
}

impl TablePolicy {
    fn from_options(options: &SubsetOptions) -> Result<Self> {
        let drop = parse_table_tags(&options.drop_tables, "dropTables")?;
        let pass_through = parse_table_tags(&options.pass_through_tables, "passThroughTables")?;

        if let Some(tag) = drop.intersection(&pass_through).next() {
            return Err(FontminError::config(format!(
                "OpenType table `{}` cannot appear in both dropTables and passThroughTables",
                table_tag_name(*tag),
            )));
        }
        if options.layout == LayoutSubsetMode::Preserve
            && [*b"GDEF", *b"GPOS", *b"GSUB"]
                .iter()
                .any(|tag| drop.contains(tag))
        {
            return Err(FontminError::config(
                "dropTables cannot remove layout tables when layout is preserve",
            ));
        }

        Ok(Self { drop, pass_through })
    }

    fn is_empty(&self) -> bool {
        self.drop.is_empty() && self.pass_through.is_empty()
    }
}

const REQUIRED_SUBSET_TABLES: &[[u8; 4]] = &[
    *b"CFF ", *b"CFF2", *b"OS/2", *b"cmap", *b"glyf", *b"head", *b"hhea", *b"hmtx", *b"loca",
    *b"maxp", *b"name", *b"post",
];

const REWRITTEN_SUBSET_TABLES: &[[u8; 4]] = &[
    *b"CBDT", *b"CBLC", *b"CFF ", *b"CFF2", *b"COLR", *b"GDEF", *b"GPOS", *b"GSUB", *b"HVAR",
    *b"MATH", *b"OS/2", *b"SVG ", *b"VVAR", *b"cmap", *b"glyf", *b"gvar", *b"head", *b"hhea",
    *b"hmtx", *b"kern", *b"loca", *b"maxp", *b"name", *b"post", *b"sbix", *b"vhea", *b"vmtx",
];

const GID_SENSITIVE_PASSTHROUGH_TABLES: &[[u8; 4]] = &[
    *b"BASE", *b"EBDT", *b"EBLC", *b"EBSC", *b"Glat", *b"Gloc", *b"JSTF", *b"LTSH", *b"Silf",
    *b"Sill", *b"VORG", *b"ankr", *b"bsln", *b"hdmx", *b"just", *b"kerx", *b"lcar", *b"mort",
    *b"morx", *b"opbd", *b"prop", *b"trak",
];

fn parse_table_tags(values: &[String], field: &str) -> Result<BTreeSet<[u8; 4]>> {
    values
        .iter()
        .map(|value| {
            let bytes = value.as_bytes();
            if bytes.len() != 4 || !bytes.iter().all(|byte| (0x20..=0x7e).contains(byte)) {
                return Err(FontminError::config(format!(
                    "{field} entry `{value}` must be exactly four printable ASCII bytes",
                )));
            }

            Ok(bytes.try_into().expect("four-byte table tag was validated"))
        })
        .collect()
}

fn table_tag_name(tag: [u8; 4]) -> String {
    String::from_utf8_lossy(&tag).into_owned()
}

fn apply_table_policy(
    output: Vec<u8>,
    source: &fontmin_ttf::TtfFont<'_>,
    policy: &TablePolicy,
    retain_gids: bool,
) -> Result<Vec<u8>> {
    if policy.is_empty() {
        return Ok(output);
    }

    let subset = fontmin_ttf::read_ttf(&output)?;
    let mut tables = subset
        .tables
        .iter()
        .map(|record| {
            Ok((
                record.tag.clone(),
                subset
                    .table(&record.tag)
                    .ok_or_else(|| FontminError::invalid_font("subset table disappeared"))?
                    .to_vec(),
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;

    validate_paired_table_drop(&tables, &policy.drop, *b"CBDT", *b"CBLC")?;
    validate_paired_table_drop(&tables, &policy.drop, *b"vhea", *b"vmtx")?;

    let mut changed = false;
    for tag in &policy.drop {
        let name = table_tag_name(*tag);
        if tables.contains_key(&name) && REQUIRED_SUBSET_TABLES.contains(tag) {
            return Err(FontminError::config(format!(
                "dropTables cannot remove required OpenType table `{name}`",
            )));
        }
        changed |= tables.remove(&name).is_some();
    }

    for tag in &policy.pass_through {
        let name = table_tag_name(*tag);
        let Some(data) = source.table(&name) else {
            continue;
        };
        if REWRITTEN_SUBSET_TABLES.contains(tag) {
            return Err(FontminError::config(format!(
                "passThroughTables cannot override subset-rewritten table `{name}`",
            )));
        }
        if tag == b"DSIG" {
            return Err(FontminError::config(
                "passThroughTables cannot retain DSIG because subsetting invalidates its signature",
            ));
        }
        if GID_SENSITIVE_PASSTHROUGH_TABLES.contains(tag) && !retain_gids {
            return Err(FontminError::config(format!(
                "passThroughTables entry `{name}` contains glyph-indexed data; enable retainGids or drop the table",
            )));
        }
        if tables.get(&name).is_none_or(|existing| existing != data) {
            tables.insert(name, data.to_vec());
            changed = true;
        }
    }

    if !changed {
        return Ok(output);
    }

    let tables = tables
        .into_iter()
        .map(|(tag, data)| fontmin_ttf::OwnedSfntTable { tag, data })
        .collect();
    fontmin_ttf::write_ttf(&fontmin_ttf::OwnedTtfFont { tables })
}

fn validate_paired_table_drop(
    tables: &BTreeMap<String, Vec<u8>>,
    drop: &BTreeSet<[u8; 4]>,
    first: [u8; 4],
    second: [u8; 4],
) -> Result<()> {
    let first_name = table_tag_name(first);
    let second_name = table_tag_name(second);
    if tables.contains_key(&first_name)
        && tables.contains_key(&second_name)
        && drop.contains(&first) != drop.contains(&second)
    {
        return Err(FontminError::config(format!(
            "dropTables must remove paired OpenType tables `{first_name}` and `{second_name}` together",
        )));
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn subset_report(
    original_size: usize,
    subset_size: usize,
    stats: &oxifont_subset::SubsetStats,
    requested_gids: &BTreeSet<u16>,
    supported_gids: &BTreeSet<u16>,
    missing_gids: &BTreeSet<u16>,
    glyph_name_selection: &GlyphNameSelection,
    gid_map: &oxifont_subset::SubsetGidMap,
    unicode_to_old_gid: &BTreeMap<u32, u16>,
) -> SubsetReport {
    SubsetReport {
        original_size,
        subset_size,
        glyphs_retained: stats.glyphs_retained,
        tables_retained: stats
            .tables_retained
            .iter()
            .map(|tag| String::from_utf8_lossy(tag).into_owned())
            .collect(),
        dropped_context_subtables: stats.dropped_context_subtables,
        cff_charstrings_verbatim: stats.cff_charstrings_verbatim,
        requested_gids: requested_gids.iter().copied().collect(),
        supported_gids: supported_gids.iter().copied().collect(),
        missing_gids: missing_gids.iter().copied().collect(),
        requested_glyph_names: glyph_name_selection.requested.iter().cloned().collect(),
        supported_glyph_names: glyph_name_selection.supported.iter().cloned().collect(),
        missing_glyph_names: glyph_name_selection.missing.iter().cloned().collect(),
        glyph_name_to_old_gid: glyph_name_selection.mappings.clone(),
        old_to_new: gid_map
            .iter()
            .map(|(old_gid, new_gid)| GidMapping { old_gid, new_gid })
            .collect(),
        new_to_old: gid_map.new_to_old().to_vec(),
        unicode_to_old_gid: unicode_to_old_gid
            .iter()
            .map(|(&unicode, &old_gid)| UnicodeGidMapping { unicode, old_gid })
            .collect(),
    }
}

#[allow(clippy::too_many_arguments)]
fn identity_subset_result(
    input: &[u8],
    source: &fontmin_ttf::TtfFont<'_>,
    glyph_count: u16,
    requested_gids: &BTreeSet<u16>,
    supported_gids: &BTreeSet<u16>,
    missing_gids: &BTreeSet<u16>,
    glyph_name_selection: &GlyphNameSelection,
    unicode_to_old_gid: &BTreeMap<u32, u16>,
) -> SubsetResult {
    let new_to_old = (0..glyph_count).map(Some).collect::<Vec<_>>();
    let old_to_new = new_to_old
        .iter()
        .flatten()
        .copied()
        .map(|old_gid| GidMapping {
            old_gid,
            new_gid: old_gid,
        })
        .collect();

    SubsetResult {
        data: input.to_vec(),
        report: SubsetReport {
            original_size: input.len(),
            subset_size: input.len(),
            glyphs_retained: glyph_count,
            tables_retained: source
                .tables
                .iter()
                .map(|record| record.tag.clone())
                .collect(),
            dropped_context_subtables: 0,
            cff_charstrings_verbatim: false,
            requested_gids: requested_gids.iter().copied().collect(),
            supported_gids: supported_gids.iter().copied().collect(),
            missing_gids: missing_gids.iter().copied().collect(),
            requested_glyph_names: glyph_name_selection.requested.iter().cloned().collect(),
            supported_glyph_names: glyph_name_selection.supported.iter().cloned().collect(),
            missing_glyph_names: glyph_name_selection.missing.iter().cloned().collect(),
            glyph_name_to_old_gid: glyph_name_selection.mappings.clone(),
            old_to_new,
            new_to_old,
            unicode_to_old_gid: unicode_to_old_gid
                .iter()
                .map(|(&unicode, &old_gid)| UnicodeGidMapping { unicode, old_gid })
                .collect(),
        },
    }
}

#[derive(Debug, Default)]
struct GlyphNameSelection {
    requested: BTreeSet<String>,
    supported: BTreeSet<String>,
    missing: BTreeSet<String>,
    mappings: Vec<GlyphNameGidMapping>,
}

#[derive(Debug, Default)]
struct LayoutSelection {
    features: Option<BTreeSet<[u8; 4]>>,
    scripts: Option<BTreeSet<[u8; 4]>>,
    languages: Option<BTreeSet<[u8; 4]>>,
    retain_default_language: bool,
}

impl LayoutSelection {
    fn from_options(options: &SubsetOptions) -> Result<Self> {
        let features = parse_layout_tags(&options.layout_features, "layoutFeatures", false)?;
        let scripts = parse_layout_tags(&options.layout_scripts, "layoutScripts", false)?;
        let mut retain_default_language = false;
        let named_languages = options
            .layout_languages
            .iter()
            .filter(|value| {
                if value.eq_ignore_ascii_case("default") {
                    retain_default_language = true;
                    false
                } else {
                    true
                }
            })
            .cloned()
            .collect::<Vec<_>>();
        let languages = if options.layout_languages.is_empty() {
            None
        } else {
            Some(parse_layout_tags(&named_languages, "layoutLanguages", true)?.unwrap_or_default())
        };

        Ok(Self {
            features,
            scripts,
            languages,
            retain_default_language,
        })
    }
}

fn parse_layout_tags(
    values: &[String],
    field: &str,
    pad_three_byte_tags: bool,
) -> Result<Option<BTreeSet<[u8; 4]>>> {
    if values.is_empty() {
        return Ok(None);
    }

    values
        .iter()
        .map(|value| parse_layout_tag(value, field, pad_three_byte_tags))
        .collect::<Result<BTreeSet<_>>>()
        .map(Some)
}

fn parse_layout_tag(value: &str, field: &str, pad_three_byte_tag: bool) -> Result<[u8; 4]> {
    let bytes = value.as_bytes();
    let mut tag = [b' '; 4];
    let valid_length = bytes.len() == 4 || (pad_three_byte_tag && bytes.len() == 3);

    if !valid_length
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_graphic() || *byte == b' ')
    {
        return Err(FontminError::config(format!(
            "{field} value `{value}` must be a printable four-byte OpenType tag{}",
            if pad_three_byte_tag {
                " (three-byte language tags are padded with a space)"
            } else {
                ""
            }
        )));
    }

    tag[..bytes.len()].copy_from_slice(bytes);

    Ok(tag)
}

fn resolve_glyph_names(input: &[u8], requested: &BTreeSet<String>) -> Result<GlyphNameSelection> {
    if requested.is_empty() {
        return Ok(GlyphNameSelection::default());
    }

    let font = SkrifaFontRef::new(input)
        .map_err(|error| FontminError::invalid_font(format!("invalid font data: {error}")))?;
    let mut supported = BTreeSet::new();
    let mut mappings = Vec::new();

    for (glyph_id, glyph_name) in font.glyph_names().iter() {
        let glyph_name = glyph_name.as_str();
        if !requested.contains(glyph_name) {
            continue;
        }

        let old_gid = u16::try_from(glyph_id.to_u32())
            .map_err(|_| FontminError::invalid_font("glyph ID exceeds u16"))?;
        supported.insert(glyph_name.to_owned());
        mappings.push(GlyphNameGidMapping {
            glyph_name: glyph_name.to_owned(),
            old_gid,
        });
    }

    let missing = requested.difference(&supported).cloned().collect();

    Ok(GlyphNameSelection {
        requested: requested.clone(),
        supported,
        missing,
        mappings,
    })
}

fn missing_gid_error(missing_gids: &BTreeSet<u16>) -> FontminError {
    const LIMIT: usize = 16;

    let visible = missing_gids
        .iter()
        .take(LIMIT)
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = missing_gids.len().saturating_sub(LIMIT);
    let suffix = if remaining == 0 {
        String::new()
    } else {
        format!(", and {remaining} more")
    };

    FontminError::MissingGlyph {
        message: format!("missing glyphs for requested glyph IDs: {visible}{suffix}"),
    }
}

fn missing_glyph_name_error(missing_names: &BTreeSet<String>) -> FontminError {
    const LIMIT: usize = 16;

    let visible = missing_names
        .iter()
        .take(LIMIT)
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = missing_names.len().saturating_sub(LIMIT);
    let suffix = if remaining == 0 {
        String::new()
    } else {
        format!(", and {remaining} more")
    };

    FontminError::MissingGlyph {
        message: format!("missing glyphs for requested glyph names: {visible}{suffix}"),
    }
}

fn ensure_layout_can_be_preserved(input: &[u8]) -> Result<()> {
    let font = fontmin_ttf::read_ttf(input)?;

    for tag in ["GSUB", "GPOS"] {
        let Some(table) = font.table(tag) else {
            continue;
        };
        if read_u16_at(table, 0, "layout table major version")? == 1
            && read_u16_at(table, 2, "layout table minor version")? >= 1
            && read_u32_at(table, 10, "layout FeatureVariations offset")? != 0
        {
            return Err(FontminError::config(format!(
                "keepLayout preserve cannot retain {tag} FeatureVariations; use conservative or drop"
            )));
        }
    }

    Ok(())
}

fn ensure_layout_was_preserved(
    input: &[u8],
    output: &[u8],
    dropped_context_subtables: usize,
) -> Result<()> {
    if dropped_context_subtables != 0 {
        return Err(FontminError::config(format!(
            "keepLayout preserve could not retain {dropped_context_subtables} contextual layout subtables; use conservative or drop"
        )));
    }

    let input_font = fontmin_ttf::read_ttf(input)?;
    let output_font = fontmin_ttf::read_ttf(output)?;
    for tag in ["GDEF", "GPOS", "GSUB"] {
        if input_font.table(tag).is_some() && output_font.table(tag).is_none() {
            return Err(FontminError::config(format!(
                "keepLayout preserve could not retain the {tag} table; use conservative or drop"
            )));
        }
    }

    Ok(())
}

fn apply_notdef_policy(input: Vec<u8>, keep_notdef: bool) -> Result<Vec<u8>> {
    if keep_notdef {
        return Ok(input);
    }

    let font = fontmin_ttf::read_ttf(&input)?;
    let (empty_glyf, empty_loca) = empty_notdef_outline(&font)?;
    let tables = font
        .tables
        .iter()
        .map(|record| {
            let data = match record.tag.as_str() {
                "glyf" => empty_glyf.clone(),
                "loca" => empty_loca.clone(),
                _ => font
                    .table(&record.tag)
                    .ok_or_else(|| {
                        FontminError::invalid_font(format!(
                            "subset table {} points outside the font",
                            record.tag
                        ))
                    })?
                    .to_vec(),
            };

            Ok(fontmin_ttf::OwnedSfntTable {
                tag: record.tag.clone(),
                data,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    fontmin_ttf::write_ttf(&fontmin_ttf::OwnedTtfFont { tables })
}

fn apply_glyph_name_policy(
    output: Vec<u8>,
    source_data: &[u8],
    source: &fontmin_ttf::TtfFont<'_>,
    gid_map: &oxifont_subset::SubsetGidMap,
    retain_glyph_names: bool,
) -> Result<Vec<u8>> {
    if !retain_glyph_names {
        return Ok(output);
    }

    let font = SkrifaFontRef::new(source_data)
        .map_err(|error| FontminError::invalid_font(format!("invalid font data: {error}")))?;
    let source_names = font
        .glyph_names()
        .iter()
        .map(|(glyph_id, glyph_name)| {
            let glyph_id = u16::try_from(glyph_id.to_u32())
                .map_err(|_| FontminError::invalid_font("glyph ID exceeds u16"))?;
            Ok((glyph_id, glyph_name.as_str().to_owned()))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let post = build_post_v2(source.table("post"), gid_map.new_to_old(), &source_names)?;

    replace_subset_table(&output, "post", &post)
}

fn build_post_v2(
    source_post: Option<&[u8]>,
    new_to_old: &[Option<u16>],
    source_names: &BTreeMap<u16, String>,
) -> Result<Vec<u8>> {
    const POST_HEADER_LEN: usize = 32;
    const CUSTOM_NAME_BASE: usize = 258;

    let glyph_count = u16::try_from(new_to_old.len())
        .map_err(|_| FontminError::invalid_font("post glyph count exceeds u16"))?;
    let mut post = vec![0; POST_HEADER_LEN];
    post[..4].copy_from_slice(&0x0002_0000u32.to_be_bytes());
    if let Some(source_post) = source_post.filter(|table| table.len() >= POST_HEADER_LEN) {
        post[4..POST_HEADER_LEN].copy_from_slice(&source_post[4..POST_HEADER_LEN]);
    }
    post.extend_from_slice(&glyph_count.to_be_bytes());

    let mut indices = Vec::with_capacity(new_to_old.len());
    let mut custom_indices = BTreeMap::<String, u16>::new();
    let mut custom_names = Vec::<String>::new();
    for (new_gid, old_gid) in new_to_old.iter().enumerate() {
        let fallback = || {
            if new_gid == 0 {
                ".notdef".to_owned()
            } else {
                let fallback_gid = u16::try_from(new_gid)
                    .expect("post glyph count guarantees every new GID fits u16");
                format!("gid{}", old_gid.unwrap_or(fallback_gid))
            }
        };
        let mut name = old_gid
            .and_then(|old_gid| source_names.get(&old_gid).cloned())
            .unwrap_or_else(fallback);
        if name.len() > u8::MAX as usize {
            name = fallback();
        }

        if let Some(index) = skrifa::raw::tables::post::DEFAULT_GLYPH_NAMES
            .iter()
            .position(|standard| *standard == name)
        {
            indices.push(u16::try_from(index).expect("standard post name index fits u16"));
            continue;
        }

        let index = if let Some(index) = custom_indices.get(&name) {
            *index
        } else {
            let index = CUSTOM_NAME_BASE
                .checked_add(custom_names.len())
                .and_then(|index| u16::try_from(index).ok())
                .ok_or_else(|| {
                    FontminError::invalid_font(
                        "post version 2 cannot encode more custom glyph names",
                    )
                })?;
            custom_indices.insert(name.clone(), index);
            custom_names.push(name);
            index
        };
        indices.push(index);
    }

    for index in indices {
        post.extend_from_slice(&index.to_be_bytes());
    }
    for name in custom_names {
        post.push(u8::try_from(name.len()).expect("custom post name length was validated"));
        post.extend_from_slice(name.as_bytes());
    }

    Ok(post)
}

fn replace_subset_table(output: &[u8], tag: &str, replacement: &[u8]) -> Result<Vec<u8>> {
    let font = fontmin_ttf::read_ttf(output)?;
    let tables = font
        .tables
        .iter()
        .map(|record| {
            let data = if record.tag == tag {
                replacement.to_vec()
            } else {
                font.table(&record.tag)
                    .ok_or_else(|| FontminError::invalid_font("subset table disappeared"))?
                    .to_vec()
            };
            Ok(fontmin_ttf::OwnedSfntTable {
                tag: record.tag.clone(),
                data,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    fontmin_ttf::write_ttf(&fontmin_ttf::OwnedTtfFont { tables })
}

fn apply_cmap_policy(
    output: Vec<u8>,
    source_data: &[u8],
    gid_map: &oxifont_subset::SubsetGidMap,
    unicode_to_old_gid: &BTreeMap<u32, u16>,
    retain_legacy_cmap: bool,
    retain_symbol_cmap: bool,
) -> Result<Vec<u8>> {
    if !retain_legacy_cmap && !retain_symbol_cmap {
        return Ok(output);
    }

    let unicode_to_new_gid = unicode_to_old_gid
        .iter()
        .filter_map(|(&codepoint, &old_gid)| {
            gid_map.new_gid(old_gid).map(|new_gid| (codepoint, new_gid))
        })
        .collect::<BTreeMap<_, _>>();
    let retained_records =
        retained_cmap_records(source_data, gid_map, retain_legacy_cmap, retain_symbol_cmap)?;
    let cmap =
        oxifont_subset::cmap::rewrite_cmap_with_records(&unicode_to_new_gid, &retained_records)
            .map_err(|error| FontminError::invalid_font(error.to_string()))?;

    replace_subset_table(&output, "cmap", &cmap)
}

fn retained_cmap_records(
    source_data: &[u8],
    gid_map: &oxifont_subset::SubsetGidMap,
    retain_legacy_cmap: bool,
    retain_symbol_cmap: bool,
) -> Result<Vec<oxifont_subset::cmap::RetainedEncodingRecord>> {
    let font = SkrifaFontRef::new(source_data)
        .map_err(|error| FontminError::invalid_font(format!("invalid font data: {error}")))?;
    let cmap = font
        .cmap()
        .map_err(|error| FontminError::invalid_font(format!("invalid cmap table: {error}")))?;
    let limits = CmapIterLimits::default_for_font(&font);
    let mut retained = Vec::new();

    for record in cmap.encoding_records() {
        let platform = record.platform_id();
        let encoding_id = record.encoding_id();
        let symbol = platform == PlatformId::Windows && encoding_id == 0;
        let unicode = platform == PlatformId::Unicode
            || (platform == PlatformId::Windows && matches!(encoding_id, 1 | 10));
        let legacy = !symbol && !unicode;
        if !(symbol && retain_symbol_cmap || legacy && retain_legacy_cmap) {
            continue;
        }
        if platform == PlatformId::Unknown {
            return Err(FontminError::config(
                "retainLegacyCmap cannot preserve an unknown cmap platform ID",
            ));
        }

        let subtable = record.subtable(cmap.offset_data()).map_err(|error| {
            FontminError::invalid_font(format!("invalid cmap subtable: {error}"))
        })?;
        let pairs: Box<dyn Iterator<Item = (u32, skrifa::GlyphId)> + '_> = match &subtable {
            CmapSubtable::Format0(_) => {
                Box::new((0..=u32::from(u8::MAX)).filter_map(|codepoint| {
                    subtable
                        .map_codepoint(codepoint)
                        .map(|gid| (codepoint, gid))
                }))
            }
            CmapSubtable::Format4(_)
            | CmapSubtable::Format6(_)
            | CmapSubtable::Format10(_)
            | CmapSubtable::Format12(_)
            | CmapSubtable::Format13(_) => Box::new(subtable.iter_with_limits(limits)),
            _ => {
                let field = if symbol {
                    "retainSymbolCmap"
                } else {
                    "retainLegacyCmap"
                };
                return Err(FontminError::config(format!(
                    "{field} cannot rewrite cmap format {} for platform {} encoding {encoding_id}",
                    subtable.format(),
                    platform as u16,
                )));
            }
        };
        let mappings = pairs
            .filter_map(|(codepoint, glyph_id)| {
                u16::try_from(glyph_id.to_u32())
                    .ok()
                    .filter(|old_gid| *old_gid != 0)
                    .and_then(|old_gid| gid_map.new_gid(old_gid))
                    .map(|new_gid| (codepoint, new_gid))
            })
            .collect::<BTreeMap<_, _>>();
        if mappings.is_empty() {
            continue;
        }
        retained.push(oxifont_subset::cmap::RetainedEncodingRecord {
            platform_id: platform as u16,
            encoding_id,
            language: subtable.language(),
            mappings,
        });
    }

    Ok(retained)
}

fn empty_notdef_outline(font: &fontmin_ttf::TtfFont<'_>) -> Result<(Vec<u8>, Vec<u8>)> {
    let head = required_subset_table(font, "head")?;
    let maxp = required_subset_table(font, "maxp")?;
    let loca = required_subset_table(font, "loca")?;
    let glyf = required_subset_table(font, "glyf")?;
    let index_to_loc_format = read_i16_at(head, 50, "head indexToLocFormat")?;
    let glyph_count = usize::from(read_u16_at(maxp, 4, "maxp numGlyphs")?);
    let entry_count = glyph_count
        .checked_add(1)
        .ok_or_else(|| FontminError::invalid_font("loca entry count overflows"))?;
    let mut offsets = read_loca_offsets(loca, entry_count, index_to_loc_format)?;

    if offsets.len() < 2 {
        return Err(FontminError::invalid_font(
            "subset font does not contain a glyph zero loca entry",
        ));
    }
    if offsets.windows(2).any(|pair| pair[0] > pair[1])
        || offsets.iter().any(|offset| *offset > glyf.len())
    {
        return Err(FontminError::invalid_font(
            "subset font contains invalid loca offsets",
        ));
    }

    let start = offsets[0];
    let end = offsets[1];
    let removed_length = end - start;
    let mut rewritten_glyf = Vec::with_capacity(glyf.len() - removed_length);
    rewritten_glyf.extend_from_slice(&glyf[..start]);
    rewritten_glyf.extend_from_slice(&glyf[end..]);

    for offset in offsets.iter_mut().skip(1) {
        *offset = offset.checked_sub(removed_length).ok_or_else(|| {
            FontminError::invalid_font("subset font contains invalid loca offsets")
        })?;
    }

    let rewritten_loca = write_loca_offsets(loca, &offsets, index_to_loc_format)?;

    Ok((rewritten_glyf, rewritten_loca))
}

fn required_subset_table<'a>(font: &fontmin_ttf::TtfFont<'a>, tag: &str) -> Result<&'a [u8]> {
    font.table(tag)
        .ok_or_else(|| FontminError::invalid_font(format!("subset font is missing {tag} table")))
}

fn read_loca_offsets(input: &[u8], count: usize, format: i16) -> Result<Vec<usize>> {
    let entry_size = match format {
        0 => 2,
        1 => 4,
        _ => {
            return Err(FontminError::invalid_font(format!(
                "unsupported indexToLocFormat {format}"
            )));
        }
    };
    let required_length = count
        .checked_mul(entry_size)
        .ok_or_else(|| FontminError::invalid_font("loca table length overflows"))?;
    if input.len() < required_length {
        return Err(FontminError::invalid_font("loca table is truncated"));
    }

    (0..count)
        .map(|index| {
            let offset = index * entry_size;
            if format == 0 {
                Ok(usize::from(read_u16_at(input, offset, "loca offset")?) * 2)
            } else {
                usize::try_from(read_u32_at(input, offset, "loca offset")?)
                    .map_err(|_| FontminError::invalid_font("loca offset exceeds platform limits"))
            }
        })
        .collect()
}

fn write_loca_offsets(input: &[u8], offsets: &[usize], format: i16) -> Result<Vec<u8>> {
    let mut output = input.to_vec();

    for (index, offset) in offsets.iter().copied().enumerate() {
        if format == 0 {
            if offset % 2 != 0 {
                return Err(FontminError::invalid_font(
                    "short loca offset is not two-byte aligned",
                ));
            }
            let value = u16::try_from(offset / 2)
                .map_err(|_| FontminError::invalid_font("short loca offset is too large"))?;
            output[index * 2..index * 2 + 2].copy_from_slice(&value.to_be_bytes());
        } else {
            let value = u32::try_from(offset)
                .map_err(|_| FontminError::invalid_font("long loca offset is too large"))?;
            output[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
        }
    }

    Ok(output)
}

fn read_u16_at(input: &[u8], offset: usize, field: &str) -> Result<u16> {
    let bytes = input
        .get(offset..offset + 2)
        .ok_or_else(|| FontminError::invalid_font(format!("{field} is truncated")))?;

    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_i16_at(input: &[u8], offset: usize, field: &str) -> Result<i16> {
    let bytes = input
        .get(offset..offset + 2)
        .ok_or_else(|| FontminError::invalid_font(format!("{field} is truncated")))?;

    Ok(i16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u32_at(input: &[u8], offset: usize, field: &str) -> Result<u32> {
    let bytes = input
        .get(offset..offset + 4)
        .ok_or_else(|| FontminError::invalid_font(format!("{field} is truncated")))?;

    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn collect_requested(options: &CoverageOptions, operation: &str) -> Result<BTreeSet<char>> {
    let chars = collect_chars_with_ranges(
        options.text.as_deref(),
        &options.unicodes,
        options.basic_text,
        &options.unicode_ranges,
    )?;

    if chars.is_empty() {
        return Err(FontminError::config(format!(
            "{operation} requires at least one character from text, unicodes, Unicode ranges, or basicText"
        )));
    }

    Ok(chars)
}

fn with_font<T>(input: &[u8], operation: impl FnOnce(&Font<'_>) -> Result<T>) -> Result<T> {
    fontmin_ttf::read_ttf(input)?;

    let reader = FontReader::new(input)
        .map_err(|error| FontminError::invalid_font(format!("invalid font data: {error}")))?;
    let font = reader
        .read()
        .map_err(|error| FontminError::invalid_font(format!("invalid font data: {error}")))?;

    operation(&font)
}

fn partition_coverage(
    font: &Font<'_>,
    requested: &BTreeSet<char>,
) -> (BTreeSet<char>, CoverageReport) {
    let supported = requested
        .iter()
        .copied()
        .filter(|character| font.contains_char(*character))
        .collect::<BTreeSet<_>>();
    let missing = requested
        .difference(&supported)
        .copied()
        .map(u32::from)
        .collect::<Vec<_>>();
    let report = CoverageReport::new(
        requested.iter().copied().map(u32::from).collect(),
        supported.iter().copied().map(u32::from).collect(),
        missing,
    );

    (supported, report)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use fontmin_testing::{NOTO_SANS_SC_VARIABLE_COMPACT, ROBOTO};

    use fontmin_core::{CoverageOptions, MissingGlyphPolicy};
    use fontmin_diagnostics::FontminErrorKind;
    use skrifa::{FontRef as SkrifaFontRef, MetadataProvider, raw::TableProvider};

    use super::{
        InstanceOptions, LayoutSubsetMode, SubsetOptions, analyze_ttf_coverage, instantiate_ttf,
        parse_layout_tag, read_u16_at, resolve_glyph_names, subset_ttf, subset_ttf_with_report,
        ttf_unicode_codepoints,
    };

    fn table_data<'a>(input: &'a [u8], tag: &str) -> &'a [u8] {
        fontmin_ttf::read_ttf(input).unwrap().table(tag).unwrap()
    }

    #[test]
    fn instantiates_glyf_variable_font_at_default_coordinates() {
        let output =
            instantiate_ttf(NOTO_SANS_SC_VARIABLE_COMPACT, &InstanceOptions::default()).unwrap();
        let metadata = fontmin_ttf::inspect_ttf(&output).unwrap();

        assert_eq!(metadata.glyph_count, 5);
        for tag in [
            "fvar", "gvar", "avar", "HVAR", "VVAR", "MVAR", "STAT", "cvt ", "fpgm", "prep", "gasp",
        ] {
            assert!(!metadata.tables.iter().any(|table| table == tag), "{tag}");
        }
        assert_eq!(fontmin_ttf::calculate_table_checksum(&output), 0xB1B0_AFBA);
    }

    #[test]
    fn lists_sorted_unicode_cmap_coverage() {
        let code_points = ttf_unicode_codepoints(NOTO_SANS_SC_VARIABLE_COMPACT).unwrap();

        assert!(code_points.contains(&0x41));
        assert!(code_points.contains(&0x4e2d));
        assert!(code_points.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn requested_glyf_instance_changes_outlines_and_metrics() {
        let default =
            instantiate_ttf(NOTO_SANS_SC_VARIABLE_COMPACT, &InstanceOptions::default()).unwrap();
        let bold = instantiate_ttf(
            NOTO_SANS_SC_VARIABLE_COMPACT,
            &InstanceOptions {
                variation_coordinates: BTreeMap::from([("wght".to_owned(), 900.0)]),
            },
        )
        .unwrap();

        assert_ne!(table_data(&default, "glyf"), table_data(&bold, "glyf"));
        assert_ne!(table_data(&default, "hmtx"), table_data(&bold, "hmtx"));
    }

    #[test]
    fn rejects_unknown_and_out_of_range_instance_coordinates() {
        for (tag, value, expected) in [
            ("WGHT", 400.0, "unknown variation axis `WGHT`"),
            ("wght", 901.0, "outside [100, 900]"),
            ("wght", f32::NAN, "outside [100, 900]"),
        ] {
            let error = instantiate_ttf(
                NOTO_SANS_SC_VARIABLE_COMPACT,
                &InstanceOptions {
                    variation_coordinates: BTreeMap::from([(tag.to_owned(), value)]),
                },
            )
            .unwrap_err();

            assert_eq!(error.kind(), FontminErrorKind::InvalidFont);
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn rejects_static_true_type_instancing() {
        let error = instantiate_ttf(ROBOTO, &InstanceOptions::default()).unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::UnsupportedFormat);
        assert!(error.to_string().contains("without fvar axes"));
    }

    fn layout_list_tags(input: &[u8], table_tag: &str, offset_field: usize) -> Vec<String> {
        let font = fontmin_ttf::read_ttf(input).unwrap();
        let table = font.table(table_tag).unwrap();
        let list_offset = usize::from(read_u16_at(table, offset_field, "layout list").unwrap());
        let count = usize::from(read_u16_at(table, list_offset, "layout count").unwrap());

        (0..count)
            .map(|index| {
                let start = list_offset + 2 + index * 6;
                String::from_utf8(table[start..start + 4].to_vec()).unwrap()
            })
            .collect()
    }

    fn name_record_ids(input: &[u8]) -> Vec<(u16, u16)> {
        let font = fontmin_ttf::read_ttf(input).unwrap();
        let table = font.table("name").unwrap();
        let count = usize::from(read_u16_at(table, 2, "name count").unwrap());

        (0..count)
            .map(|index| {
                let start = 6 + index * 12;
                (
                    read_u16_at(table, start + 6, "name ID").unwrap(),
                    read_u16_at(table, start + 4, "name language ID").unwrap(),
                )
            })
            .collect()
    }

    fn glyph_zero_data_length(input: &[u8]) -> usize {
        let font = fontmin_ttf::read_ttf(input).unwrap();
        let head = font.table("head").unwrap();
        let loca = font.table("loca").unwrap();
        let index_to_loc_format = i16::from_be_bytes(head[50..52].try_into().unwrap());

        match index_to_loc_format {
            0 => {
                let start = usize::from(u16::from_be_bytes(loca[0..2].try_into().unwrap())) * 2;
                let end = usize::from(u16::from_be_bytes(loca[2..4].try_into().unwrap())) * 2;

                end - start
            }
            1 => {
                let start =
                    usize::try_from(u32::from_be_bytes(loca[0..4].try_into().unwrap())).unwrap();
                let end =
                    usize::try_from(u32::from_be_bytes(loca[4..8].try_into().unwrap())).unwrap();

                end - start
            }
            _ => panic!("unsupported indexToLocFormat {index_to_loc_format}"),
        }
    }

    fn with_gsub_feature_variations(input: &[u8]) -> Vec<u8> {
        let font = fontmin_ttf::read_ttf(input).unwrap();
        let tables = font
            .tables
            .iter()
            .map(|record| {
                let mut data = font.table(&record.tag).unwrap().to_vec();
                if record.tag == "GSUB" {
                    data[0..4].copy_from_slice(&0x0001_0001_u32.to_be_bytes());
                    data[10..14].copy_from_slice(&14_u32.to_be_bytes());
                }

                fontmin_ttf::OwnedSfntTable {
                    tag: record.tag.clone(),
                    data,
                }
            })
            .collect();

        fontmin_ttf::write_ttf(&fontmin_ttf::OwnedTtfFont { tables }).unwrap()
    }

    fn without_stored_postscript_names(input: &[u8]) -> Vec<u8> {
        let font = fontmin_ttf::read_ttf(input).unwrap();
        let tables = font
            .tables
            .iter()
            .map(|record| {
                let mut data = font.table(&record.tag).unwrap().to_vec();
                if record.tag == "post" {
                    data[0..4].copy_from_slice(&0x0003_0000_u32.to_be_bytes());
                    data.truncate(32);
                }

                fontmin_ttf::OwnedSfntTable {
                    tag: record.tag.clone(),
                    data,
                }
            })
            .collect();

        fontmin_ttf::write_ttf(&fontmin_ttf::OwnedTtfFont { tables }).unwrap()
    }

    fn with_custom_table(input: &[u8], tag: &str, data: &[u8]) -> Vec<u8> {
        let font = fontmin_ttf::read_ttf(input).unwrap();
        let mut tables = font
            .tables
            .iter()
            .filter(|record| record.tag != tag)
            .map(|record| fontmin_ttf::OwnedSfntTable {
                tag: record.tag.clone(),
                data: font.table(&record.tag).unwrap().to_vec(),
            })
            .collect::<Vec<_>>();
        tables.push(fontmin_ttf::OwnedSfntTable {
            tag: tag.into(),
            data: data.to_vec(),
        });

        fontmin_ttf::write_ttf(&fontmin_ttf::OwnedTtfFont { tables }).unwrap()
    }

    fn with_cmap_alias(
        input: &[u8],
        platform_id: u16,
        encoding_id: u16,
        source_platform_id: u16,
        source_encoding_id: u16,
    ) -> Vec<u8> {
        let font = fontmin_ttf::read_ttf(input).unwrap();
        let cmap = font.table("cmap").unwrap();
        let record_count = usize::from(u16::from_be_bytes(cmap[2..4].try_into().unwrap()));
        let old_header_len = 4 + record_count * 8;
        let source_offset = (0..record_count)
            .find_map(|index| {
                let offset = 4 + index * 8;
                let platform = u16::from_be_bytes(cmap[offset..offset + 2].try_into().unwrap());
                let encoding = u16::from_be_bytes(cmap[offset + 2..offset + 4].try_into().unwrap());
                (platform == source_platform_id && encoding == source_encoding_id)
                    .then(|| u32::from_be_bytes(cmap[offset + 4..offset + 8].try_into().unwrap()))
            })
            .unwrap();

        let mut rewritten = Vec::with_capacity(cmap.len() + 8);
        rewritten.extend_from_slice(&0u16.to_be_bytes());
        rewritten.extend_from_slice(&u16::try_from(record_count + 1).unwrap().to_be_bytes());
        for index in 0..record_count {
            let offset = 4 + index * 8;
            rewritten.extend_from_slice(&cmap[offset..offset + 4]);
            let subtable_offset =
                u32::from_be_bytes(cmap[offset + 4..offset + 8].try_into().unwrap());
            rewritten.extend_from_slice(&(subtable_offset + 8).to_be_bytes());
        }
        rewritten.extend_from_slice(&platform_id.to_be_bytes());
        rewritten.extend_from_slice(&encoding_id.to_be_bytes());
        rewritten.extend_from_slice(&(source_offset + 8).to_be_bytes());
        rewritten.extend_from_slice(&cmap[old_header_len..]);

        with_custom_table(input, "cmap", &rewritten)
    }

    fn cmap_mapping(
        input: &[u8],
        platform_id: u16,
        encoding_id: u16,
        codepoint: u32,
    ) -> Option<u16> {
        let font = SkrifaFontRef::new(input).unwrap();
        let cmap = font.cmap().unwrap();
        cmap.encoding_records()
            .iter()
            .find(|record| {
                record.platform_id() as u16 == platform_id && record.encoding_id() == encoding_id
            })
            .and_then(|record| record.subtable(cmap.offset_data()).ok())
            .and_then(|subtable| subtable.map_codepoint(codepoint))
            .and_then(|glyph_id| u16::try_from(glyph_id.to_u32()).ok())
    }

    #[test]
    fn subsets_ttf_to_a_smaller_valid_opentype_buffer() {
        let output = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                unicodes: Vec::new(),
                unicode_ranges: Vec::new(),
                gids: Vec::new(),
                glyph_names: Vec::new(),
                basic_text: false,
                preserve_hinting: false,
                trim: true,
                keep_notdef: true,
                retain_gids: false,
                retain_glyph_names: false,
                retain_legacy_cmap: false,
                retain_symbol_cmap: false,
                layout: LayoutSubsetMode::Conservative,
                layout_features: Vec::new(),
                layout_scripts: Vec::new(),
                layout_languages: Vec::new(),
                name_ids: Vec::new(),
                name_languages: Vec::new(),
                drop_tables: Vec::new(),
                pass_through_tables: Vec::new(),
                missing_glyphs: MissingGlyphPolicy::Warn,
            },
        )
        .unwrap();

        assert!(output.len() < ROBOTO.len());
        assert!(
            output.starts_with(&[0x00, 0x01, 0x00, 0x00]) || output.starts_with(b"OTTO"),
            "subset output must remain OpenType data",
        );
    }

    #[test]
    fn preserve_hinting_controls_hint_program_tables() {
        let without_hinting = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                preserve_hinting: false,
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let with_hinting = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                preserve_hinting: true,
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let without_hinting_tables = fontmin_ttf::inspect_ttf(&without_hinting).unwrap().tables;
        let with_hinting_tables = fontmin_ttf::inspect_ttf(&with_hinting).unwrap().tables;

        for tag in ["cvt ", "fpgm", "prep"] {
            assert!(
                !without_hinting_tables.iter().any(|table| table == tag),
                "{tag} should be removed when hinting is not preserved",
            );
            assert!(
                with_hinting_tables.iter().any(|table| table == tag),
                "{tag} should remain when hinting is preserved",
            );
        }
    }

    #[test]
    fn keep_notdef_controls_glyph_zero_outline() {
        let without_notdef = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                keep_notdef: false,
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let with_notdef = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                keep_notdef: true,
                ..SubsetOptions::default()
            },
        )
        .unwrap();

        assert_eq!(glyph_zero_data_length(&without_notdef), 0);
        assert!(glyph_zero_data_length(&with_notdef) > 0);
    }

    #[test]
    fn layout_modes_control_layout_table_retention() {
        let subset_with_layout = |layout| {
            subset_ttf(
                ROBOTO,
                SubsetOptions {
                    text: Some("Hello".into()),
                    layout,
                    ..SubsetOptions::default()
                },
            )
        };
        let dropped = subset_with_layout(LayoutSubsetMode::Drop).unwrap();
        let conservative = subset_with_layout(LayoutSubsetMode::Conservative).unwrap();
        let preserve_error = subset_with_layout(LayoutSubsetMode::Preserve).unwrap_err();
        let dropped_tables = fontmin_ttf::inspect_ttf(&dropped).unwrap().tables;
        let conservative_tables = fontmin_ttf::inspect_ttf(&conservative).unwrap().tables;

        for tag in ["GDEF", "GPOS", "GSUB"] {
            assert!(!dropped_tables.iter().any(|table| table == tag));
            assert!(conservative_tables.iter().any(|table| table == tag));
        }
        assert!(
            preserve_error
                .to_string()
                .contains("keepLayout preserve could not retain 31 contextual layout subtables")
        );
    }

    #[test]
    fn layout_tag_filters_retain_only_selected_features_and_scripts() {
        let result = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("office".into()),
                glyph_names: vec!["fi".into()],
                layout_features: vec!["liga".into()],
                layout_scripts: vec!["latn".into()],
                layout_languages: vec!["default".into()],
                ..SubsetOptions::default()
            },
        )
        .unwrap();

        assert_eq!(layout_list_tags(&result, "GSUB", 6), ["liga"]);
        assert_eq!(layout_list_tags(&result, "GSUB", 4), ["latn"]);
        assert!(layout_list_tags(&result, "GPOS", 6).is_empty());
        assert!(layout_list_tags(&result, "GPOS", 4).is_empty());
    }

    #[test]
    fn name_filters_retain_only_selected_ids_and_languages() {
        let result = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("A".into()),
                name_ids: vec![1],
                name_languages: vec![0x0409],
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let records = name_record_ids(&result);

        assert!(!records.is_empty());
        assert!(records.iter().all(|record| *record == (1, 0x0409)));
    }

    #[test]
    fn table_policy_drops_optional_tables_and_restores_custom_tables() {
        let input = with_custom_table(ROBOTO, "TEST", b"custom metadata");
        let result = subset_ttf_with_report(
            &input,
            SubsetOptions {
                text: Some("office".into()),
                drop_tables: vec!["GPOS".into()],
                pass_through_tables: vec!["TEST".into()],
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let output = fontmin_ttf::read_ttf(&result.data).unwrap();

        assert!(output.table("GPOS").is_none());
        assert_eq!(output.table("TEST").unwrap(), b"custom metadata");
        assert!(
            !result
                .report
                .tables_retained
                .iter()
                .any(|tag| tag == "GPOS")
        );
        assert!(
            result
                .report
                .tables_retained
                .iter()
                .any(|tag| tag == "TEST")
        );
        assert_eq!(result.report.subset_size, result.data.len());

        let baseline = subset_ttf(ROBOTO, SubsetOptions::with_text("A")).unwrap();
        let missing_passthrough = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("A".into()),
                pass_through_tables: vec!["TEST".into()],
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        assert_eq!(missing_passthrough, baseline);
    }

    #[test]
    fn table_policy_rejects_conflicts_and_unsafe_overrides() {
        let subset = |input: &[u8], options| subset_ttf(input, options).unwrap_err();

        let invalid = subset(
            ROBOTO,
            SubsetOptions {
                text: Some("A".into()),
                drop_tables: vec!["bad".into()],
                ..SubsetOptions::default()
            },
        );
        assert_eq!(invalid.kind(), FontminErrorKind::Config);
        assert!(invalid.to_string().contains("four printable ASCII"));

        let conflict = subset(
            ROBOTO,
            SubsetOptions {
                text: Some("A".into()),
                drop_tables: vec!["GPOS".into()],
                pass_through_tables: vec!["GPOS".into()],
                ..SubsetOptions::default()
            },
        );
        assert!(
            conflict
                .to_string()
                .contains("both dropTables and passThroughTables")
        );

        let required = subset(
            ROBOTO,
            SubsetOptions {
                text: Some("A".into()),
                drop_tables: vec!["cmap".into()],
                ..SubsetOptions::default()
            },
        );
        assert!(
            required
                .to_string()
                .contains("required OpenType table `cmap`")
        );

        let rewritten = subset(
            ROBOTO,
            SubsetOptions {
                text: Some("A".into()),
                pass_through_tables: vec!["GSUB".into()],
                ..SubsetOptions::default()
            },
        );
        assert!(
            rewritten
                .to_string()
                .contains("subset-rewritten table `GSUB`")
        );

        let input = with_custom_table(ROBOTO, "BASE", b"glyph indexed");
        let default_subset = subset_ttf(
            &input,
            SubsetOptions {
                text: Some("A".into()),
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        assert!(
            fontmin_ttf::read_ttf(&default_subset)
                .unwrap()
                .table("BASE")
                .is_none()
        );
        let unsafe_table = subset(
            &input,
            SubsetOptions {
                text: Some("A".into()),
                pass_through_tables: vec!["BASE".into()],
                ..SubsetOptions::default()
            },
        );
        assert!(unsafe_table.to_string().contains("enable retainGids"));

        let preserve_conflict = subset(
            ROBOTO,
            SubsetOptions {
                text: Some("A".into()),
                layout: LayoutSubsetMode::Preserve,
                drop_tables: vec!["GSUB".into()],
                ..SubsetOptions::default()
            },
        );
        assert!(preserve_conflict.to_string().contains("layout is preserve"));
    }

    #[test]
    fn table_policy_allows_explicit_gid_sensitive_passthrough_with_retained_ids() {
        let input = with_custom_table(ROBOTO, "BASE", b"glyph indexed");
        let result = subset_ttf(
            &input,
            SubsetOptions {
                text: Some("A".into()),
                retain_gids: true,
                pass_through_tables: vec!["BASE".into()],
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let output = fontmin_ttf::read_ttf(&result).unwrap();

        assert_eq!(output.table("BASE").unwrap(), b"glyph indexed");
    }

    #[test]
    fn layout_tag_validation_accepts_padded_languages_and_rejects_invalid_tags() {
        assert_eq!(
            parse_layout_tag("ENG", "layoutLanguages", true).unwrap(),
            *b"ENG "
        );
        assert!(parse_layout_tag("long-tag", "layoutFeatures", false).is_err());
        assert!(parse_layout_tag("éé", "layoutScripts", false).is_err());
    }

    #[test]
    fn preserve_layout_rejects_feature_variations_before_remapping() {
        let input = with_gsub_feature_variations(ROBOTO);
        let error = subset_ttf(
            &input,
            SubsetOptions {
                text: Some("Hello".into()),
                layout: LayoutSubsetMode::Preserve,
                ..SubsetOptions::default()
            },
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("keepLayout preserve cannot retain GSUB FeatureVariations")
        );
    }

    #[test]
    fn subsets_ttf_from_unicode_ranges() {
        let output = subset_ttf(
            ROBOTO,
            SubsetOptions {
                unicode_ranges: vec!["U+0041-0042".parse().unwrap()],
                ..SubsetOptions::default()
            },
        )
        .unwrap();

        assert!(output.len() < ROBOTO.len());
    }

    #[test]
    fn subsets_by_explicit_gid_without_unicode_selection() {
        let result = subset_ttf_with_report(
            ROBOTO,
            SubsetOptions {
                gids: vec![1],
                ..SubsetOptions::default()
            },
        )
        .unwrap();

        assert!(result.data.len() < ROBOTO.len());
        assert_eq!(result.report.requested_gids, vec![1]);
        assert_eq!(result.report.supported_gids, vec![1]);
        assert!(result.report.missing_gids.is_empty());
        assert!(
            result
                .report
                .old_to_new
                .iter()
                .any(|mapping| mapping.old_gid == 1)
        );
        assert_eq!(
            usize::from(result.report.glyphs_retained),
            result.report.new_to_old.len()
        );
    }

    #[test]
    fn subsets_by_postscript_glyph_name_and_reports_the_original_gid() {
        let result = subset_ttf_with_report(
            ROBOTO,
            SubsetOptions {
                glyph_names: vec!["A".into()],
                ..SubsetOptions::default()
            },
        )
        .unwrap();

        assert!(result.data.len() < ROBOTO.len());
        assert_eq!(result.report.requested_glyph_names, ["A"]);
        assert_eq!(result.report.supported_glyph_names, ["A"]);
        assert!(result.report.missing_glyph_names.is_empty());
        assert_eq!(result.report.glyph_name_to_old_gid.len(), 1);
        assert_eq!(result.report.glyph_name_to_old_gid[0].glyph_name, "A");
        assert_eq!(result.report.glyph_name_to_old_gid[0].old_gid, 38);
        assert!(
            result
                .report
                .old_to_new
                .iter()
                .any(|mapping| mapping.old_gid == 38)
        );
    }

    #[test]
    fn retain_glyph_names_rewrites_post_v2_in_new_gid_order() {
        let source_font = SkrifaFontRef::new(ROBOTO).unwrap();
        let (custom_old_gid, custom_name) = source_font
            .glyph_names()
            .iter()
            .find_map(|(glyph_id, glyph_name)| {
                let glyph_name = glyph_name.as_str();
                (!skrifa::raw::tables::post::DEFAULT_GLYPH_NAMES.contains(&glyph_name)).then(|| {
                    (
                        u16::try_from(glyph_id.to_u32()).unwrap(),
                        glyph_name.to_owned(),
                    )
                })
            })
            .expect("Roboto should contain a custom PostScript glyph name");
        let result = subset_ttf_with_report(
            ROBOTO,
            SubsetOptions {
                text: Some("A".into()),
                glyph_names: vec![custom_name.clone()],
                retain_glyph_names: true,
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let output = fontmin_ttf::read_ttf(&result.data).unwrap();
        let post = output.table("post").unwrap();
        let requested_names = BTreeSet::from(["A".to_owned(), custom_name.clone()]);
        let selected = resolve_glyph_names(&result.data, &requested_names)
            .expect("rewritten post table should expose retained names");

        assert_eq!(&post[..4], &0x0002_0000u32.to_be_bytes());
        assert_eq!(selected.supported, requested_names);
        assert_eq!(selected.mappings.len(), 2);
        let output_custom_gid = selected
            .mappings
            .iter()
            .find(|mapping| mapping.glyph_name == custom_name)
            .unwrap()
            .old_gid;
        assert_eq!(
            result.report.new_to_old[usize::from(output_custom_gid)],
            Some(custom_old_gid)
        );
    }

    #[test]
    fn glyph_names_are_removed_by_default() {
        let result = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("A".into()),
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let output = fontmin_ttf::read_ttf(&result).unwrap();

        assert_eq!(
            output.table("post").unwrap()[..4],
            0x0003_0000u32.to_be_bytes()
        );
    }

    #[test]
    fn retain_legacy_cmap_rewrites_macintosh_mappings() {
        let default_result = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("A".into()),
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let retained_result = subset_ttf_with_report(
            ROBOTO,
            SubsetOptions {
                text: Some("A".into()),
                retain_legacy_cmap: true,
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let new_gid = retained_result
            .report
            .old_to_new
            .iter()
            .find(|mapping| mapping.old_gid == 38)
            .unwrap()
            .new_gid;

        assert_eq!(cmap_mapping(&default_result, 1, 0, 0x41), None);
        assert_eq!(
            cmap_mapping(&retained_result.data, 1, 0, 0x41),
            Some(new_gid)
        );
    }

    #[test]
    fn retain_symbol_cmap_rewrites_windows_symbol_mappings() {
        let input = with_cmap_alias(ROBOTO, 3, 0, 3, 1);
        let default_result = subset_ttf(
            &input,
            SubsetOptions {
                text: Some("A".into()),
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let retained_result = subset_ttf_with_report(
            &input,
            SubsetOptions {
                text: Some("A".into()),
                retain_symbol_cmap: true,
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let new_gid = retained_result
            .report
            .old_to_new
            .iter()
            .find(|mapping| mapping.old_gid == 38)
            .unwrap()
            .new_gid;

        assert_eq!(cmap_mapping(&default_result, 3, 0, 0x41), None);
        assert_eq!(
            cmap_mapping(&retained_result.data, 3, 0, 0x41),
            Some(new_gid)
        );
        assert_eq!(cmap_mapping(&retained_result.data, 1, 0, 0x41), None);
    }

    #[test]
    fn strict_policy_rejects_missing_glyph_names() {
        let error = subset_ttf_with_report(
            ROBOTO,
            SubsetOptions {
                glyph_names: vec!["A".into(), "does.not.exist".into()],
                missing_glyphs: MissingGlyphPolicy::Error,
                ..SubsetOptions::default()
            },
        )
        .unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::MissingGlyph);
        assert!(error.to_string().contains("glyph names: `does.not.exist`"));
    }

    #[test]
    fn selects_synthesized_names_when_the_font_does_not_store_names() {
        let input = without_stored_postscript_names(ROBOTO);
        let result = subset_ttf_with_report(
            &input,
            SubsetOptions {
                glyph_names: vec!["gid38".into()],
                ..SubsetOptions::default()
            },
        )
        .unwrap();

        assert_eq!(result.report.supported_glyph_names, ["gid38"]);
        assert_eq!(result.report.glyph_name_to_old_gid[0].old_gid, 38);
    }

    #[test]
    fn reports_unicode_and_gid_mappings_for_mixed_selection() {
        let result = subset_ttf_with_report(
            ROBOTO,
            SubsetOptions {
                text: Some("A".into()),
                gids: vec![1],
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let unicode_mapping = result
            .report
            .unicode_to_old_gid
            .iter()
            .find(|mapping| mapping.unicode == u32::from('A'))
            .unwrap();
        let remapped = result
            .report
            .old_to_new
            .iter()
            .find(|mapping| mapping.old_gid == unicode_mapping.old_gid)
            .unwrap();

        assert_eq!(
            result.report.new_to_old[usize::from(remapped.new_gid)],
            Some(remapped.old_gid)
        );
        assert_eq!(result.report.original_size, ROBOTO.len());
        assert_eq!(result.report.subset_size, result.data.len());
        assert!(
            result
                .report
                .tables_retained
                .iter()
                .any(|tag| tag == "cmap")
        );
    }

    #[test]
    fn retain_gids_preserves_selected_ids_and_reports_empty_slots() {
        let result = subset_ttf_with_report(
            ROBOTO,
            SubsetOptions {
                gids: vec![38],
                retain_gids: true,
                ..SubsetOptions::default()
            },
        )
        .unwrap();
        let font = fontmin_ttf::read_ttf(&result.data).unwrap();
        let maxp = font.table("maxp").unwrap();

        assert_eq!(read_u16_at(maxp, 4, "maxp numGlyphs").unwrap(), 39);
        assert_eq!(
            result
                .report
                .old_to_new
                .iter()
                .find(|mapping| mapping.old_gid == 38)
                .unwrap()
                .new_gid,
            38
        );
        assert_eq!(result.report.new_to_old[1], None);
        assert_eq!(result.report.new_to_old[38], Some(38));
    }

    #[test]
    fn strict_policy_rejects_out_of_range_gids() {
        let error = subset_ttf_with_report(
            ROBOTO,
            SubsetOptions {
                gids: vec![u16::MAX],
                missing_glyphs: MissingGlyphPolicy::Error,
                ..SubsetOptions::default()
            },
        )
        .unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::MissingGlyph);
        assert!(error.to_string().contains("glyph IDs: 65535"));
    }

    #[test]
    fn trim_false_keeps_original_font_data() {
        let output = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                trim: false,
                ..SubsetOptions::default()
            },
        )
        .unwrap();

        assert_eq!(output.len(), ROBOTO.len());
        assert_eq!(output.as_slice(), ROBOTO);
    }

    #[test]
    fn rejects_empty_subset_requests() {
        let error = subset_ttf(ROBOTO, SubsetOptions::default()).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("subset requires at least one character")
        );
    }

    #[test]
    fn rejects_invalid_font_data() {
        let error = subset_ttf(b"not a font", SubsetOptions::with_text("Hello")).unwrap_err();

        assert!(error.to_string().contains("invalid font data"));
    }

    #[test]
    fn reports_supported_and_missing_requested_codepoints() {
        let report = analyze_ttf_coverage(
            ROBOTO,
            &CoverageOptions {
                text: Some("A𠮷".into()),
                ..CoverageOptions::default()
            },
        )
        .unwrap();

        assert_eq!(report.requested, vec![0x41, 0x20bb7]);
        assert_eq!(report.supported, vec![0x41]);
        assert_eq!(report.missing, vec![0x20bb7]);
        assert!((report.coverage_percent - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn strict_missing_glyph_policy_rejects_partial_coverage() {
        let error = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("A𠮷".into()),
                missing_glyphs: MissingGlyphPolicy::Error,
                ..SubsetOptions::default()
            },
        )
        .unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::MissingGlyph);
        assert!(error.to_string().contains("U+20BB7"));
    }

    #[test]
    fn strict_missing_glyph_policy_reports_fully_missing_coverage() {
        let error = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("𠮷".into()),
                missing_glyphs: MissingGlyphPolicy::Error,
                ..SubsetOptions::default()
            },
        )
        .unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::MissingGlyph);
        assert!(error.to_string().contains("U+20BB7"));
    }

    #[test]
    fn warning_policy_keeps_supported_characters() {
        let output = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("A𠮷".into()),
                ..SubsetOptions::default()
            },
        )
        .unwrap();

        assert!(output.len() < ROBOTO.len());
    }
}
