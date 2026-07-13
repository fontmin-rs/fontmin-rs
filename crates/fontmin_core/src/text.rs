use std::collections::BTreeSet;

use fontmin_diagnostics::{FontminError, Result};

use crate::UnicodeRange;

const BASIC_TEXT: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,;:!?-_()[]{}'\"/\\@#$%^&*+=<>|`~";
const MAX_RANGE_SCALARS: usize = 65_536;

pub fn collect_chars(
    text: Option<&str>,
    unicodes: &[u32],
    basic_text: bool,
) -> Result<BTreeSet<char>> {
    collect_chars_with_ranges(text, unicodes, basic_text, &[])
}

pub fn collect_chars_with_ranges(
    text: Option<&str>,
    unicodes: &[u32],
    basic_text: bool,
    unicode_ranges: &[UnicodeRange],
) -> Result<BTreeSet<char>> {
    let mut chars = BTreeSet::new();

    if basic_text {
        chars.extend(BASIC_TEXT.chars());
    }

    if let Some(text) = text {
        chars.extend(text.chars());
    }

    for codepoint in unicodes {
        let Some(character) = char::from_u32(*codepoint) else {
            return Err(FontminError::config(format!(
                "invalid unicode code point: 0x{codepoint:x}",
            )));
        };
        chars.insert(character);
    }

    let mut expanded_scalars = 0_usize;
    for range in unicode_ranges {
        for value in range.start..=range.end {
            let Some(character) = char::from_u32(value) else {
                continue;
            };
            expanded_scalars += 1;
            if expanded_scalars > MAX_RANGE_SCALARS {
                return Err(FontminError::config(format!(
                    "Unicode range expansion exceeds {MAX_RANGE_SCALARS} scalar values"
                )));
            }
            chars.insert(character);
        }
    }

    Ok(chars)
}

#[cfg(test)]
mod tests {
    use crate::{FontDeliverySlice, UnicodeRange, validate_delivery_slices};

    use super::collect_chars;

    #[test]
    fn canonicalizes_and_serializes_unicode_ranges() {
        let range = "u+4e00-9fff".parse::<UnicodeRange>().unwrap();

        assert_eq!(range.to_string(), "U+4E00-9FFF");
        assert_eq!(serde_json::to_string(&range).unwrap(), "\"U+4E00-9FFF\"");
        assert_eq!(
            serde_json::from_str::<UnicodeRange>("\"U+0020-007E\"")
                .unwrap()
                .to_string(),
            "U+0020-007E"
        );
    }

    #[test]
    fn rejects_invalid_unicode_ranges() {
        for value in [
            "U+",
            "U+110000",
            "U+D800",
            "U+D7FF-E000",
            "U+007E-0020",
            "U+4??",
        ] {
            assert!(value.parse::<UnicodeRange>().is_err(), "{value}");
        }
    }

    #[test]
    fn validates_delivery_slice_names_and_ranges() {
        let latin = FontDeliverySlice {
            name: "latin-basic".into(),
            unicode_ranges: vec!["U+0020-007E".parse().unwrap()],
        };

        assert!(validate_delivery_slices(std::slice::from_ref(&latin)).is_ok());
        assert!(validate_delivery_slices(&[latin.clone(), latin]).is_err());
        assert!(
            validate_delivery_slices(&[FontDeliverySlice {
                name: "../escape".into(),
                unicode_ranges: vec!["U+0020".parse().unwrap()],
            }])
            .is_err()
        );
    }

    #[test]
    fn expands_unicode_ranges_with_a_bounded_budget() {
        let chars =
            super::collect_chars_with_ranges(None, &[], false, &["U+0041-0043".parse().unwrap()])
                .unwrap();

        assert_eq!(chars.into_iter().collect::<String>(), "ABC");
        assert!(super::collect_chars_with_ranges(
            None,
            &[],
            false,
            &["U+10000-20000".parse().unwrap()],
        )
        .is_err());
    }

    #[test]
    fn collects_text_and_unicode_values_once() {
        let chars = collect_chars(Some("abca"), &[0x4e2d], false).unwrap();
        let collected: String = chars.into_iter().collect();

        assert_eq!(collected, "abc中");
    }

    #[test]
    fn rejects_invalid_unicode_values() {
        let error = collect_chars(None, &[0x11_0000], false).unwrap_err();

        assert!(error.to_string().contains("invalid unicode code point"));
    }

    #[test]
    fn includes_basic_text_when_requested() {
        let chars = collect_chars(None, &[], true).unwrap();

        assert!(chars.contains(&'A'));
        assert!(chars.contains(&'z'));
        assert!(chars.contains(&'0'));
    }
}
