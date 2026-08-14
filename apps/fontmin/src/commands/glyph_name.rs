use miette::{Result, miette};

pub fn parse_optional_glyph_names(value: Option<&str>) -> Result<Vec<String>> {
    value.map_or_else(|| Ok(Vec::new()), parse_glyph_names)
}

fn parse_glyph_names(value: &str) -> Result<Vec<String>> {
    value
        .split(',')
        .map(str::trim)
        .map(|name| {
            if name.is_empty() {
                Err(miette!("empty glyph name in --glyph-names"))
            } else {
                Ok(name.to_owned())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_optional_glyph_names;

    #[test]
    fn parses_comma_separated_glyph_names() {
        assert_eq!(
            parse_optional_glyph_names(Some("A, uni4E00,space")).unwrap(),
            ["A", "uni4E00", "space"]
        );
    }

    #[test]
    fn rejects_empty_glyph_names() {
        assert!(parse_optional_glyph_names(Some("A,,space")).is_err());
    }
}
