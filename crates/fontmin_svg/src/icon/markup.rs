use super::path::{ViewBox, numbers};

pub(super) fn view_box(svg: &str) -> Option<ViewBox> {
    let value = attribute_value(svg, "viewBox")?;
    let numbers = numbers(&value);

    if numbers.len() != 4 || numbers[2] <= 0.0 || numbers[3] <= 0.0 {
        return None;
    }

    Some(ViewBox {
        x: numbers[0],
        y: numbers[1],
        width: numbers[2],
        height: numbers[3],
    })
}

pub(super) fn path_data_values(svg: &str) -> Vec<String> {
    let mut values = Vec::new();
    let bytes = svg.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index].eq_ignore_ascii_case(&b'd') && is_attribute_boundary(bytes, index) {
            let mut cursor = index + 1;

            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'=' {
                cursor += 1;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor < bytes.len() && (bytes[cursor] == b'"' || bytes[cursor] == b'\'') {
                    let quote = bytes[cursor];
                    cursor += 1;
                    let value_start = cursor;
                    while cursor < bytes.len() && bytes[cursor] != quote {
                        cursor += 1;
                    }
                    if cursor <= bytes.len() {
                        values.push(svg[value_start..cursor].to_string());
                    }
                    index = cursor;
                }
            }
        }
        index += 1;
    }

    values
}

pub(super) fn attribute_value(svg: &str, name: &str) -> Option<String> {
    let bytes = svg.as_bytes();
    let name_bytes = name.as_bytes();
    let mut index = 0;

    while index + name_bytes.len() <= bytes.len() {
        if bytes[index..].starts_with(name_bytes) && is_attribute_boundary(bytes, index) {
            let mut cursor = index + name_bytes.len();

            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'=' {
                cursor += 1;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if cursor < bytes.len() && (bytes[cursor] == b'"' || bytes[cursor] == b'\'') {
                    let quote = bytes[cursor];
                    cursor += 1;
                    let value_start = cursor;
                    while cursor < bytes.len() && bytes[cursor] != quote {
                        cursor += 1;
                    }
                    return Some(svg[value_start..cursor].to_string());
                }
            }
        }
        index += 1;
    }

    None
}

pub(super) fn attribute_f32(svg: &str, name: &str) -> Option<f32> {
    attribute_value(svg, name)?.parse().ok()
}

pub(super) fn element_tags(svg: &str, name: &str) -> Vec<String> {
    let pattern = format!("<{name}");
    let bytes = svg.as_bytes();
    let mut tags = Vec::new();
    let mut index = 0;

    while let Some(relative_start) = svg[index..].find(&pattern) {
        let start = index + relative_start;
        let after_name = start + pattern.len();

        if !is_element_name_boundary(bytes, after_name) {
            index = after_name;
            continue;
        }

        let Some(relative_end) = svg[after_name..].find('>') else {
            break;
        };
        let end = after_name + relative_end + 1;

        tags.push(svg[start..end].to_string());
        index = end;
    }

    tags
}

pub(super) fn decode_unicode_value(value: &str) -> Option<u32> {
    let value = value.trim();

    if let Some(hex) = value
        .strip_prefix("&#x")
        .or_else(|| value.strip_prefix("&#X"))
        .and_then(|value| value.strip_suffix(';'))
    {
        return u32::from_str_radix(hex, 16).ok();
    }
    if let Some(decimal) = value
        .strip_prefix("&#")
        .and_then(|value| value.strip_suffix(';'))
    {
        return decimal.parse().ok();
    }

    decode_xml_entities(value).chars().next().map(u32::from)
}

fn is_element_name_boundary(bytes: &[u8], index: usize) -> bool {
    index >= bytes.len() || matches!(bytes[index], b' ' | b'\t' | b'\n' | b'\r' | b'/' | b'>')
}

fn is_attribute_boundary(bytes: &[u8], index: usize) -> bool {
    if index > 0 {
        let previous = bytes[index - 1];
        if previous.is_ascii_alphanumeric() || previous == b'-' || previous == b'_' {
            return false;
        }
    }

    true
}

fn decode_xml_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}
