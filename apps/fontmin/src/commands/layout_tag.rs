use miette::{Result, miette};

pub fn parse_optional_layout_tags(value: Option<&str>, flag: &str) -> Result<Vec<String>> {
    value.map_or_else(
        || Ok(Vec::new()),
        |value| {
            value
                .split(',')
                .map(str::trim)
                .map(|tag| {
                    if tag.is_empty() {
                        Err(miette!("empty OpenType tag in {flag}"))
                    } else {
                        Ok(tag.to_owned())
                    }
                })
                .collect()
        },
    )
}

pub fn parse_optional_table_tags(value: Option<&str>, flag: &str) -> Result<Vec<String>> {
    value.map_or_else(
        || Ok(Vec::new()),
        |value| {
            value
                .split(',')
                .map(str::trim)
                .map(|tag| {
                    let tag = if tag.len() == 3 {
                        format!("{tag} ")
                    } else {
                        tag.to_owned()
                    };
                    if tag.len() != 4
                        || !tag.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
                    {
                        return Err(miette!(
                            "OpenType table tag `{tag}` in {flag} must be three or four printable ASCII bytes"
                        ));
                    }

                    Ok(tag)
                })
                .collect()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{parse_optional_layout_tags, parse_optional_table_tags};

    #[test]
    fn parses_layout_tag_lists() {
        assert_eq!(
            parse_optional_layout_tags(Some("liga, kern"), "--layout-features").unwrap(),
            ["liga", "kern"]
        );
    }

    #[test]
    fn rejects_empty_layout_tags() {
        assert!(parse_optional_layout_tags(Some("latn,,DFLT"), "--layout-scripts").is_err());
    }

    #[test]
    fn validates_table_tags() {
        assert_eq!(
            parse_optional_table_tags(Some("GPOS, SVG"), "--drop-tables").unwrap(),
            ["GPOS", "SVG "]
        );
        assert!(parse_optional_table_tags(Some("name,badxx"), "--drop-tables").is_err());
    }
}
