use fontmin_core::FontFormat;

#[must_use]
pub fn detect_format(bytes: &[u8]) -> FontFormat {
    if bytes.starts_with(&[0x00, 0x01, 0x00, 0x00]) || bytes.starts_with(b"true") {
        return FontFormat::Ttf;
    }

    if bytes.starts_with(b"OTTO") {
        return FontFormat::Otf;
    }

    if bytes.starts_with(b"wOFF") {
        return FontFormat::Woff;
    }

    if bytes.starts_with(b"wOF2") {
        return FontFormat::Woff2;
    }

    if looks_like_eot(bytes) {
        return FontFormat::Eot;
    }

    if looks_like_svg_font(bytes) {
        return FontFormat::Svg;
    }

    FontFormat::Unknown
}

fn looks_like_eot(bytes: &[u8]) -> bool {
    bytes.len() >= 12
        && (bytes[8..12] == [0x01, 0x00, 0x02, 0x00] || bytes[8..12] == [0x02, 0x00, 0x02, 0x00])
}

fn looks_like_svg_font(bytes: &[u8]) -> bool {
    let Ok(prefix) = std::str::from_utf8(&bytes[..bytes.len().min(512)]) else {
        return false;
    };

    let trimmed = prefix.trim_start();
    trimmed.starts_with("<svg") || trimmed.starts_with("<?xml") && trimmed.contains("<svg")
}

#[cfg(test)]
mod tests {
    use super::detect_format;
    use fontmin_core::FontFormat;

    #[test]
    fn detects_common_font_magic_bytes() {
        assert_eq!(detect_format(&[0x00, 0x01, 0x00, 0x00]), FontFormat::Ttf);
        assert_eq!(detect_format(b"OTTO"), FontFormat::Otf);
        assert_eq!(detect_format(b"wOFF1234"), FontFormat::Woff);
        assert_eq!(detect_format(b"wOF21234"), FontFormat::Woff2);
        assert_eq!(detect_format(b"<svg><font /></svg>"), FontFormat::Svg);
        assert_eq!(detect_format(b"plain text"), FontFormat::Unknown);
    }
}
