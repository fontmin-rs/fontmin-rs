use std::path::PathBuf;

use fontmin::SubsetOptions;
use miette::{Context, IntoDiagnostic, Result};

use super::coverage::{
    ensure_requested, handle_missing_glyphs, parse_missing_glyph_policy, resolve_options,
};

pub async fn run(
    input: PathBuf,
    output: PathBuf,
    text: Option<String>,
    text_file: Option<PathBuf>,
    unicodes: Option<String>,
    basic_text: bool,
    missing_glyphs: Option<String>,
) -> Result<i32> {
    let bytes = tokio::fs::read(&input)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", input.display()))?;
    let coverage_options = resolve_options(text, text_file, unicodes, basic_text).await?;
    ensure_requested(&coverage_options, "subset")?;
    let policy = parse_missing_glyph_policy(missing_glyphs.as_deref())?.unwrap_or_default();
    if policy != fontmin::MissingGlyphPolicy::Ignore {
        let report =
            fontmin::analyze_coverage(&bytes, coverage_options.clone()).into_diagnostic()?;
        handle_missing_glyphs(&report, policy, true, false)?;
    }

    let subset = fontmin::subset_ttf(
        &bytes,
        SubsetOptions {
            text: coverage_options.text,
            unicodes: coverage_options.unicodes,
            unicode_ranges: coverage_options.unicode_ranges,
            basic_text: coverage_options.basic_text,
            missing_glyphs: policy,
            ..SubsetOptions::default()
        },
    )
    .into_diagnostic()?;

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

    Ok(0)
}
