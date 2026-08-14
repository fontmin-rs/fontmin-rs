use miette::{Context, IntoDiagnostic, Result, miette};

pub fn parse_optional_gids(value: Option<&str>) -> Result<Vec<u16>> {
    value.map_or_else(|| Ok(Vec::new()), parse_gids)
}

fn parse_gids(value: &str) -> Result<Vec<u16>> {
    let mut gids = Vec::new();

    for item in value.split(',') {
        let item = item.trim();

        if item.is_empty() {
            return Err(miette!("empty glyph ID in --gids"));
        }

        gids.push(parse_gid(item)?);
    }

    if gids.is_empty() {
        return Err(miette!("expected at least one glyph ID"));
    }

    Ok(gids)
}

fn parse_gid(value: &str) -> Result<u16> {
    let (digits, radix) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map_or((value, 10), |digits| (digits, 16));

    u16::from_str_radix(digits, radix)
        .into_diagnostic()
        .wrap_err_with(|| format!("invalid glyph ID `{value}`"))
}

#[cfg(test)]
mod tests {
    use super::parse_optional_gids;

    #[test]
    fn parses_decimal_and_hexadecimal_gids() {
        assert_eq!(
            parse_optional_gids(Some("1, 0x2a,65535")).unwrap(),
            vec![1, 42, 65_535]
        );
    }

    #[test]
    fn rejects_empty_and_out_of_range_gids() {
        assert!(parse_optional_gids(Some("1,,2")).is_err());
        assert!(parse_optional_gids(Some("65536")).is_err());
    }
}
