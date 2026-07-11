use std::path::PathBuf;

use miette::{Context, IntoDiagnostic, Result};
use serde_json::json;

pub async fn run(input: PathBuf, json_output: bool) -> Result<i32> {
    let bytes = tokio::fs::read(&input)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", input.display()))?;
    let info = fontmin::inspect(&bytes).into_diagnostic()?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "path": input.display().to_string(),
                "format": info.format,
                "size": info.size,
                "metadata": info.metadata
            }))
            .into_diagnostic()?,
        );
    } else {
        println!(
            "{}: {:?}, {} bytes, {} glyphs",
            input.display(),
            info.format,
            info.size,
            info.metadata.glyph_count,
        );
    }

    Ok(0)
}
