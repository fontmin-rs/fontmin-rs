use std::path::PathBuf;

use fontmin::{CoverageOptions, CoverageReport, MissingGlyphPolicy};
use miette::{Context, IntoDiagnostic, Result, miette};

use super::unicode::parse_optional_unicodes;

pub async fn run(
    input: PathBuf,
    text: Option<String>,
    text_file: Option<PathBuf>,
    unicodes: Option<String>,
    basic_text: bool,
    json: bool,
) -> Result<i32> {
    let bytes = tokio::fs::read(&input)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", input.display()))?;
    let options = resolve_options(text, text_file, unicodes, basic_text).await?;
    ensure_requested(&options, "coverage")?;
    let report = fontmin::analyze_coverage(&bytes, options).into_diagnostic()?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).into_diagnostic()?
        );
    } else {
        print_report(&report);
    }

    Ok(0)
}

pub async fn resolve_options(
    text: Option<String>,
    text_file: Option<PathBuf>,
    unicodes: Option<String>,
    basic_text: bool,
) -> Result<CoverageOptions> {
    let text = resolve_text(text, text_file).await?;
    let unicodes = parse_optional_unicodes(unicodes.as_deref())?;

    Ok(CoverageOptions {
        text,
        unicodes,
        basic_text,
        ..CoverageOptions::default()
    })
}

pub fn ensure_requested(options: &CoverageOptions, operation: &str) -> Result<()> {
    if options.text.is_none() && options.unicodes.is_empty() && !options.basic_text {
        return Err(miette!(
            "{operation} requires --text, --text-file, --unicodes, or --basic-text"
        ));
    }

    Ok(())
}

pub fn parse_missing_glyph_policy(value: Option<&str>) -> Result<Option<MissingGlyphPolicy>> {
    value
        .map(|value| match value.trim().to_ascii_lowercase().as_str() {
            "ignore" => Ok(MissingGlyphPolicy::Ignore),
            "warn" => Ok(MissingGlyphPolicy::Warn),
            "error" => Ok(MissingGlyphPolicy::Error),
            _ => Err(miette!(
                "missing glyph policy must be `ignore`, `warn`, or `error`: {value}"
            )),
        })
        .transpose()
}

pub fn handle_missing_glyphs(
    report: &CoverageReport,
    policy: MissingGlyphPolicy,
    emit_warning: bool,
    fail_on_warning: bool,
) -> Result<()> {
    if report.is_complete() || policy == MissingGlyphPolicy::Ignore {
        return Ok(());
    }

    if policy == MissingGlyphPolicy::Error || fail_on_warning {
        return report.ensure_complete().into_diagnostic();
    }

    if emit_warning && let Some(message) = report.missing_glyph_message() {
        eprintln!("warning: {message}");
    }

    Ok(())
}

async fn resolve_text(text: Option<String>, text_file: Option<PathBuf>) -> Result<Option<String>> {
    let Some(text_file) = text_file else {
        return Ok(text);
    };
    let file_text = tokio::fs::read_to_string(&text_file)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", text_file.display()))?;

    Ok(Some(match text {
        Some(text) => format!("{text}{file_text}"),
        None => file_text,
    }))
}

fn print_report(report: &CoverageReport) {
    println!(
        "coverage: {:.2}% ({}/{})",
        report.coverage_percent,
        report.supported.len(),
        report.requested.len()
    );
    println!("requested: {}", report.requested.len());
    println!("supported: {}", report.supported.len());
    println!("missing: {}", report.missing.len());

    if let Some(message) = report.missing_glyph_message() {
        println!("{message}");
    }
}

#[cfg(test)]
mod tests {
    use fontmin::{CoverageReport, MissingGlyphPolicy};

    use super::{handle_missing_glyphs, parse_missing_glyph_policy};

    #[test]
    fn parses_missing_glyph_policies() {
        assert_eq!(
            parse_missing_glyph_policy(Some("error")).unwrap(),
            Some(MissingGlyphPolicy::Error)
        );
        assert!(parse_missing_glyph_policy(Some("strict")).is_err());
    }

    #[test]
    fn error_policy_rejects_incomplete_coverage() {
        let report = CoverageReport::new(vec![0x41, 0x20bb7], vec![0x41], vec![0x20bb7]);

        assert!(handle_missing_glyphs(&report, MissingGlyphPolicy::Error, true, false).is_err());
        assert!(handle_missing_glyphs(&report, MissingGlyphPolicy::Ignore, true, false).is_ok());
    }
}
