use std::path::PathBuf;

use fontmin::SubsetOptions;
use miette::{Context, IntoDiagnostic, Result};

use super::coverage::{
    ensure_requested, handle_missing_glyphs, parse_missing_glyph_policy, resolve_options,
};
use super::gid::parse_optional_gids;
use super::glyph_name::parse_optional_glyph_names;
use super::layout_tag::{parse_optional_layout_tags, parse_optional_table_tags};
use super::name_id::parse_optional_name_ids;

#[allow(clippy::struct_excessive_bools)]
pub struct SubsetCommandOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub text: Option<String>,
    pub text_file: Option<PathBuf>,
    pub unicodes: Option<String>,
    pub gids: Option<String>,
    pub glyph_names: Option<String>,
    pub retain_gids: bool,
    pub retain_glyph_names: bool,
    pub retain_legacy_cmap: bool,
    pub retain_symbol_cmap: bool,
    pub layout_features: Option<String>,
    pub layout_scripts: Option<String>,
    pub layout_languages: Option<String>,
    pub name_ids: Option<String>,
    pub name_languages: Option<String>,
    pub drop_tables: Option<String>,
    pub pass_through_tables: Option<String>,
    pub basic_text: bool,
    pub missing_glyphs: Option<String>,
    pub report: Option<PathBuf>,
    pub font_number: Option<usize>,
}

pub async fn run(options: SubsetCommandOptions) -> Result<i32> {
    let SubsetCommandOptions {
        input,
        output,
        text,
        text_file,
        unicodes,
        gids,
        glyph_names,
        retain_gids,
        retain_glyph_names,
        retain_legacy_cmap,
        retain_symbol_cmap,
        layout_features,
        layout_scripts,
        layout_languages,
        name_ids,
        name_languages,
        drop_tables,
        pass_through_tables,
        basic_text,
        missing_glyphs,
        report,
        font_number,
    } = options;
    let bytes = tokio::fs::read(&input)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", input.display()))?;
    let bytes = super::collection::select_collection_face(bytes, font_number)?;
    let coverage_options = resolve_options(text, text_file, unicodes, basic_text).await?;
    let gids = parse_optional_gids(gids.as_deref())?;
    let glyph_names = parse_optional_glyph_names(glyph_names.as_deref())?;
    let layout_features =
        parse_optional_layout_tags(layout_features.as_deref(), "--layout-features")?;
    let layout_scripts = parse_optional_layout_tags(layout_scripts.as_deref(), "--layout-scripts")?;
    let layout_languages =
        parse_optional_layout_tags(layout_languages.as_deref(), "--layout-languages")?;
    let name_ids = parse_optional_name_ids(name_ids.as_deref(), "--name-ids")?;
    let name_languages = parse_optional_name_ids(name_languages.as_deref(), "--name-languages")?;
    let drop_tables = parse_optional_table_tags(drop_tables.as_deref(), "--drop-tables")?;
    let pass_through_tables =
        parse_optional_table_tags(pass_through_tables.as_deref(), "--pass-through-tables")?;
    let has_unicode_selection = coverage_options.text.is_some()
        || !coverage_options.unicodes.is_empty()
        || !coverage_options.unicode_ranges.is_empty()
        || coverage_options.basic_text;
    if gids.is_empty() && glyph_names.is_empty() {
        ensure_requested(&coverage_options, "subset")?;
    }
    let policy = parse_missing_glyph_policy(missing_glyphs.as_deref())?.unwrap_or_default();
    if has_unicode_selection && policy != fontmin::MissingGlyphPolicy::Ignore {
        let report = fontmin::analyze_coverage(&bytes, coverage_options.clone())?;
        handle_missing_glyphs(&report, policy, true, false)?;
    }

    let options = SubsetOptions {
        text: coverage_options.text,
        unicodes: coverage_options.unicodes,
        gids,
        glyph_names,
        retain_gids,
        retain_glyph_names,
        retain_legacy_cmap,
        retain_symbol_cmap,
        layout_features,
        layout_scripts,
        layout_languages,
        name_ids,
        name_languages,
        drop_tables,
        pass_through_tables,
        unicode_ranges: coverage_options.unicode_ranges,
        basic_text: coverage_options.basic_text,
        missing_glyphs: policy,
        ..SubsetOptions::default()
    };
    let (subset, report_json) = if report.is_some() {
        let result = fontmin::subset_ttf_with_report(&bytes, options)?;
        let json = serde_json::to_vec_pretty(&result.report).into_diagnostic()?;

        (result.data, Some(json))
    } else {
        (fontmin::subset_ttf(&bytes, options)?, None)
    };

    if report.as_ref().is_some_and(|path| path == &output) {
        return Err(miette::miette!(
            "subset report path must differ from the font output path"
        ));
    }

    if let Some(parent) = output.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
    }

    tokio::fs::write(&output, subset)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to write {}", output.display()))?;

    if let (Some(report), Some(report_json)) = (report, report_json) {
        if let Some(parent) = report.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
        }

        tokio::fs::write(&report, report_json)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to write {}", report.display()))?;
    }

    Ok(0)
}
