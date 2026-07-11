use miette::{Context, IntoDiagnostic, Result, miette};

pub fn parse_optional_unicodes(value: Option<&str>) -> Result<Vec<u32>> {
    value.map_or_else(|| Ok(Vec::new()), parse_unicodes)
}

fn parse_unicodes(value: &str) -> Result<Vec<u32>> {
    let mut unicodes = Vec::new();

    for item in value.split(',') {
        let item = item.trim();

        if item.is_empty() {
            return Err(miette!("empty unicode code point in --unicodes"));
        }

        unicodes.push(parse_unicode_code_point(item)?);
    }

    if unicodes.is_empty() {
        return Err(miette!("expected at least one unicode code point"));
    }

    Ok(unicodes)
}

fn parse_unicode_code_point(value: &str) -> Result<u32> {
    let (digits, radix) = if let Some(digits) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .or_else(|| value.strip_prefix("U+"))
        .or_else(|| value.strip_prefix("u+"))
    {
        (digits, 16)
    } else {
        (value, 10)
    };

    u32::from_str_radix(digits, radix)
        .into_diagnostic()
        .wrap_err_with(|| format!("invalid unicode code point `{value}`"))
}
