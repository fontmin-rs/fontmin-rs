use std::collections::BTreeMap;
use std::path::PathBuf;

use fontmin::{Otf2TtfOptions, OutputFormat};
use miette::{Context, IntoDiagnostic, Result, miette};

use super::format::parse_output_format;

pub async fn run(
    input: PathBuf,
    output: PathBuf,
    format: String,
    variation: Vec<String>,
) -> Result<i32> {
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
    let variation_coordinates = parse_variations(&variation)?;
    let converted = fontmin::convert_with_options(
        &bytes,
        target,
        &Otf2TtfOptions {
            preserve_hinting: false,
            variation_coordinates,
        },
    )
    .into_diagnostic()?;

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

fn parse_variations(values: &[String]) -> Result<BTreeMap<String, f32>> {
    let mut coordinates = BTreeMap::new();

    for value in values {
        let (tag, number) = value
            .split_once('=')
            .ok_or_else(|| miette!("invalid variation `{value}`; expected TAG=VALUE"))?;
        if tag.len() != 4 || !tag.is_ascii() {
            return Err(miette!(
                "invalid variation axis `{tag}`; expected four ASCII characters"
            ));
        }
        if coordinates.contains_key(tag) {
            return Err(miette!("duplicate variation axis `{tag}`"));
        }

        let number = number
            .parse::<f32>()
            .into_diagnostic()
            .wrap_err_with(|| format!("invalid variation value `{number}` for axis `{tag}`"))?;
        if !number.is_finite() {
            return Err(miette!(
                "invalid variation value `{number}` for axis `{tag}`"
            ));
        }

        coordinates.insert(tag.to_owned(), number);
    }

    Ok(coordinates)
}

#[cfg(test)]
mod tests {
    use super::parse_variations;

    #[test]
    fn parses_finite_variation_coordinates() {
        let values = vec!["wght=700".to_owned(), "opsz=14.5".to_owned()];
        let coordinates = parse_variations(&values).unwrap();

        assert_eq!(coordinates.get("wght"), Some(&700.0));
        assert_eq!(coordinates.get("opsz"), Some(&14.5));
    }

    #[test]
    fn rejects_duplicate_variation_coordinates() {
        let values = vec!["wght=400".to_owned(), "wght=700".to_owned()];

        assert!(parse_variations(&values).is_err());
    }
}
