use std::path::PathBuf;

use miette::{Context, IntoDiagnostic, Result};
use serde_json::json;

pub async fn run(input: PathBuf, json_output: bool, font_number: Option<usize>) -> Result<i32> {
    let bytes = tokio::fs::read(&input)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", input.display()))?;
    if bytes.starts_with(b"ttcf") && font_number.is_none() {
        let info = fontmin::inspect_collection(&bytes)?;

        if json_output {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "path": input.display().to_string(),
                    "format": "collection",
                    "size": info.size,
                    "majorVersion": info.major_version,
                    "minorVersion": info.minor_version,
                    "faces": info.faces,
                }))
                .into_diagnostic()?,
            );
        } else {
            println!(
                "{}: TTC/OTC {}.{}, {} bytes, {} faces",
                input.display(),
                info.major_version,
                info.minor_version,
                info.size,
                info.faces.len(),
            );
            for face in info.faces {
                println!(
                    "  [{}] {:?}, {} bytes, {} glyphs, {}, color: {}",
                    face.index,
                    face.format,
                    face.size,
                    face.metadata.glyph_count,
                    face.metadata.family_name.as_deref().unwrap_or("unnamed"),
                    color_support(&face.capabilities),
                );
            }
        }

        return Ok(0);
    }
    let bytes = super::collection::select_collection_face(bytes, font_number)?;
    let info = fontmin::inspect(&bytes)?;
    let capabilities = fontmin::inspect_capabilities(&bytes)?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "path": input.display().to_string(),
                "format": info.format,
                "size": info.size,
                "metadata": info.metadata,
                "capabilities": capabilities,
                "fontNumber": font_number,
            }))
            .into_diagnostic()?,
        );
    } else {
        println!(
            "{}: {:?}, {} bytes, {} glyphs, color: {}",
            input.display(),
            info.format,
            info.size,
            info.metadata.glyph_count,
            color_support(&capabilities),
        );
    }

    Ok(0)
}

fn color_support(capabilities: &fontmin::FontCapabilityReport) -> &'static str {
    match capabilities.color.subset_support {
        Some(fontmin::CapabilitySupport::Subset) => "subset",
        Some(fontmin::CapabilitySupport::Passthrough) => "passthrough",
        Some(fontmin::CapabilitySupport::Unsupported) => "unsupported",
        None => "none",
    }
}
