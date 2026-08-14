use std::{collections::BTreeMap, path::PathBuf};

use fontmin::{AxisRange, AxisSetting, InstanceOptions, VariationSpaceOptions};
use miette::{Context, IntoDiagnostic, Result, miette};

use super::convert::parse_variations;

pub async fn run(
    input: PathBuf,
    output: PathBuf,
    variation: Vec<String>,
    variation_range: Vec<String>,
    keep_variable: bool,
    downgrade_cff2: bool,
    font_number: Option<usize>,
) -> Result<i32> {
    let bytes = tokio::fs::read(&input)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", input.display()))?;
    let bytes = super::collection::select_collection_face(bytes, font_number)?;
    let variation_coordinates = parse_variations(&variation)?;
    let preserve_design_space = keep_variable || !variation_range.is_empty() || downgrade_cff2;
    let instanced = if preserve_design_space {
        let mut axes: BTreeMap<String, AxisSetting> = variation_coordinates
            .into_iter()
            .map(|(tag, value)| (tag, AxisSetting::Pin(value)))
            .collect();
        for (tag, range) in parse_variation_ranges(&variation_range)? {
            if axes
                .insert(tag.clone(), AxisSetting::Range(range))
                .is_some()
            {
                return Err(miette!(
                    "variation axis `{tag}` cannot be both pinned and ranged"
                ));
            }
        }
        fontmin::reduce_variation_space(
            &bytes,
            &VariationSpaceOptions {
                axes,
                downgrade_cff2,
            },
        )?
    } else {
        fontmin::instantiate_font(
            &bytes,
            &InstanceOptions {
                variation_coordinates,
            },
        )?
    };

    if let Some(parent) = output.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
    }

    tokio::fs::write(&output, instanced)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to write {}", output.display()))?;

    Ok(0)
}

fn parse_variation_ranges(values: &[String]) -> Result<Vec<(String, AxisRange)>> {
    values
        .iter()
        .map(|value| {
            let (tag, range) = value.split_once('=').ok_or_else(|| {
                miette!("invalid variation range `{value}`; expected TAG=MIN:MAX[:DEFAULT]")
            })?;
            if tag.len() != 4 || !tag.is_ascii() {
                return Err(miette!(
                    "invalid variation axis `{tag}`; expected four ASCII characters"
                ));
            }
            let parts = range.split(':').collect::<Vec<_>>();
            if !(2..=3).contains(&parts.len()) {
                return Err(miette!(
                    "invalid variation range `{range}` for axis `{tag}`; expected MIN:MAX[:DEFAULT]"
                ));
            }
            let parse = |number: &str| -> Result<f32> {
                let parsed = number.parse::<f32>().into_diagnostic().wrap_err_with(|| {
                    format!("invalid variation range value `{number}` for axis `{tag}`")
                })?;
                if !parsed.is_finite() {
                    return Err(miette!(
                        "invalid variation range value `{number}` for axis `{tag}`"
                    ));
                }
                Ok(parsed)
            };

            Ok((
                tag.to_owned(),
                AxisRange {
                    min: parse(parts[0])?,
                    max: parse(parts[1])?,
                    default: parts.get(2).map(|value| parse(value)).transpose()?,
                },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_variation_ranges;

    #[test]
    fn parses_ranges_with_optional_defaults() {
        let values = vec!["wght=300:700:500".to_owned(), "wdth=75:125".to_owned()];
        let ranges = parse_variation_ranges(&values).unwrap();

        assert_eq!(ranges[0].1.default, Some(500.0));
        assert_eq!(ranges[1].1.default, None);
    }

    #[test]
    fn rejects_malformed_ranges() {
        assert!(parse_variation_ranges(&["wght=300".to_owned()]).is_err());
        assert!(parse_variation_ranges(&["wght=300:inf".to_owned()]).is_err());
    }
}
