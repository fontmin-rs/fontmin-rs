use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use fontmin::SubsetOptions;
use miette::{Context, IntoDiagnostic, Result, miette};
use serde_json::json;

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
    let text = subset_text(text, text_file).await?;
    let unicodes = parse_optional_unicodes(unicodes.as_deref())?;

    if text.is_none() && unicodes.is_empty() && !basic_text {
        return Err(miette!(
            "bench requires --text, --text-file, --unicodes, or --basic-text"
        ));
    }

    let started_at = Instant::now();
    let subset = fontmin::subset_ttf(
        &bytes,
        SubsetOptions {
            text,
            unicodes,
            basic_text,
            ..SubsetOptions::default()
        },
    )
    .into_diagnostic()?;
    let elapsed_ms = elapsed_millis(started_at.elapsed());

    if json {
        println!(
            "{}",
            serde_json::to_string(&json!({
                "operation": "subset",
                "inputBytes": bytes.len(),
                "outputBytes": subset.len(),
                "elapsedMs": elapsed_ms,
            }))
            .into_diagnostic()?
        );
    } else {
        println!("fontmin-rs bench subset completed in {elapsed_ms} ms");
        println!("input: {} bytes", bytes.len());
        println!("output: {} bytes", subset.len());
    }

    Ok(0)
}

async fn subset_text(text: Option<String>, text_file: Option<PathBuf>) -> Result<Option<String>> {
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

fn elapsed_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}
