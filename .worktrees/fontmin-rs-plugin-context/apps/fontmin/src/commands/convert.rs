use std::path::PathBuf;

use fontmin::OutputFormat;
use miette::{Context, IntoDiagnostic, Result, miette};

use super::format::parse_output_format;

pub async fn run(input: PathBuf, output: PathBuf, format: String) -> Result<i32> {
    let target = parse_output_format(&format)?;
    if target == OutputFormat::Css {
        return Err(miette!(
            "convert cannot write CSS directly; use build instead"
        ));
    }

    let bytes = tokio::fs::read(&input)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", input.display()))?;
    let converted = fontmin::convert(&bytes, target).into_diagnostic()?;

    if let Some(parent) = output.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
    }

    tokio::fs::write(&output, converted)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to write {}", output.display()))?;

    Ok(0)
}
