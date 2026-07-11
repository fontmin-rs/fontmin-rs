use std::path::PathBuf;

use fontmin::SubsetOptions;
use miette::{Context, IntoDiagnostic, Result};

pub async fn run(input: PathBuf, output: PathBuf, text: String, basic_text: bool) -> Result<i32> {
    let bytes = tokio::fs::read(&input)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", input.display()))?;

    let subset = fontmin::subset_ttf(
        &bytes,
        SubsetOptions {
            text: Some(text),
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
