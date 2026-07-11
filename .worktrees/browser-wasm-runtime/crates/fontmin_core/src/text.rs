use std::collections::BTreeSet;

use fontmin_diagnostics::{FontminError, Result};

const BASIC_TEXT: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,;:!?-_()[]{}'\"/\\@#$%^&*+=<>|`~";

pub fn collect_chars(
    text: Option<&str>,
    unicodes: &[u32],
    basic_text: bool,
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

    Ok(chars)
}

#[cfg(test)]
mod tests {
    use super::collect_chars;

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
