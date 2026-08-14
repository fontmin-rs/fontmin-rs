use miette::{Context, IntoDiagnostic, Result, miette};

pub fn parse_optional_name_ids(value: Option<&str>, flag: &str) -> Result<Vec<u16>> {
    value.map_or_else(
        || Ok(Vec::new()),
        |value| {
            value
                .split(',')
                .map(str::trim)
                .map(|item| {
                    if item.is_empty() {
                        return Err(miette!("empty numeric ID in {flag}"));
                    }

                    let (digits, radix) = item
                        .strip_prefix("0x")
                        .or_else(|| item.strip_prefix("0X"))
                        .map_or((item, 10), |digits| (digits, 16));
                    u16::from_str_radix(digits, radix)
                        .into_diagnostic()
                        .wrap_err_with(|| format!("invalid numeric ID `{item}` in {flag}"))
                })
                .collect()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::parse_optional_name_ids;

    #[test]
    fn parses_decimal_and_hexadecimal_ids() {
        assert_eq!(
            parse_optional_name_ids(Some("1, 0x409,65535"), "--name-ids").unwrap(),
            vec![1, 0x0409, 65_535]
        );
    }

    #[test]
    fn rejects_empty_and_out_of_range_ids() {
        assert!(parse_optional_name_ids(Some("1,,2"), "--name-ids").is_err());
        assert!(parse_optional_name_ids(Some("65536"), "--name-languages").is_err());
    }
}
