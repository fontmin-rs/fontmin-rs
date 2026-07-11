use std::path::PathBuf;

use fontmin::SubsetOptions;
use miette::{Context, IntoDiagnostic, Result, miette};

use super::unicode::parse_optional_unicodes;

pub async fn run(
    input: PathBuf,
    output: PathBuf,
    text: Option<String>,
    text_file: Option<PathBuf>,
    unicodes: Option<String>,
    basic_text: bool,
) -> Result<i32> {
    let bytes = tokio::fs::read(&input)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", input.display()))?;
    let text = subset_text(text, text_file).await?;
    let unicodes = parse_optional_unicodes(unicodes.as_deref())?;

    if text.is_none() && unicodes.is_empty() && !basic_text {
        return Err(miette!(
            "subset requires --text, --text-file, --unicodes, or --basic-text"
        ));
    }

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
