use std::{fmt, str::FromStr};

use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize, de::Error as _};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnicodeRange {
    pub start: u32,
    pub end: u32,
}

impl FromStr for UnicodeRange {
    type Err = FontminError;

    fn from_str(value: &str) -> Result<Self> {
        let body = value
            .strip_prefix("U+")
            .or_else(|| value.strip_prefix("u+"))
            .ok_or_else(|| FontminError::config(format!("invalid Unicode range: {value}")))?;
        let (start, end) = match body.split_once('-') {
            Some((start, end)) => (
                parse_unicode_endpoint(start, value)?,
                parse_unicode_endpoint(end, value)?,
            ),
            None => {
                let endpoint = parse_unicode_endpoint(body, value)?;

                (endpoint, endpoint)
            }
        };

        if start > end {
            return Err(FontminError::config(format!(
                "invalid Unicode range: {value}"
            )));
        }

        if start <= 0xDFFF && end >= 0xD800 {
            return Err(FontminError::config(format!(
                "invalid Unicode range: {value}"
            )));
        }

        Ok(Self { start, end })
    }
}

impl fmt::Display for UnicodeRange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start == self.end {
            write!(formatter, "U+{:04X}", self.start)
        } else {
            write!(formatter, "U+{:04X}-{:04X}", self.start, self.end)
        }
    }
}

impl Serialize for UnicodeRange {
    fn serialize<Serializer>(
        &self,
        serializer: Serializer,
    ) -> std::result::Result<Serializer::Ok, Serializer::Error>
    where
        Serializer: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for UnicodeRange {
    fn deserialize<Deserializer>(
        deserializer: Deserializer,
    ) -> std::result::Result<Self, Deserializer::Error>
    where
        Deserializer: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        value.parse().map_err(Deserializer::Error::custom)
    }
}

fn parse_unicode_endpoint(value: &str, original: &str) -> Result<u32> {
    if value.is_empty() || value.len() > 6 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(FontminError::config(format!(
            "invalid Unicode range: {original}"
        )));
    }

    let unicode = u32::from_str_radix(value, 16)
        .map_err(|_| FontminError::config(format!("invalid Unicode range: {original}")))?;

    if char::from_u32(unicode).is_none() {
        return Err(FontminError::config(format!(
            "invalid Unicode range: {original}"
        )));
    }

    Ok(unicode)
}
