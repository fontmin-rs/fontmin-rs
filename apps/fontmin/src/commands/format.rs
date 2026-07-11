use fontmin::OutputFormat;
use miette::{Result, miette};

pub fn parse_output_format(value: &str) -> Result<OutputFormat> {
    match value.trim().to_ascii_lowercase().as_str() {
        "ttf" => Ok(OutputFormat::Ttf),
        "woff" => Ok(OutputFormat::Woff),
        "woff2" => Ok(OutputFormat::Woff2),
        "eot" => Ok(OutputFormat::Eot),
        "svg" => Ok(OutputFormat::Svg),
        "css" => Ok(OutputFormat::Css),
        format => Err(miette!("unsupported output format `{format}`")),
    }
}

pub fn parse_output_formats(value: &str) -> Result<Vec<OutputFormat>> {
    let formats: Result<Vec<_>> = value
        .split(',')
        .map(str::trim)
        .filter(|format| !format.is_empty())
        .map(parse_output_format)
        .collect();
    let formats = formats?;

    if formats.is_empty() {
        return Err(miette!("expected at least one output format"));
    }

    Ok(formats)
}
