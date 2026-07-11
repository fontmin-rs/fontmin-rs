use std::path::PathBuf;

use fontmin_detect::detect_format;
use miette::{Context, IntoDiagnostic, Result};
use serde_json::json;

pub async fn run(input: PathBuf, json_output: bool) -> Result<i32> {
    let bytes = tokio::fs::read(&input)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", input.display()))?;
    let format = detect_format(&bytes);

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "path": input.display().to_string(),
                "format": format,
                "size": bytes.len()
            }))
            .into_diagnostic()?,
        );
    } else {
        println!("{}: {:?}, {} bytes", input.display(), format, bytes.len());
    }

    Ok(0)
}
