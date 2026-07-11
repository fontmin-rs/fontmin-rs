use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};
use ttf2woff2::BrotliQuality;

const DEFAULT_BROTLI_QUALITY: u8 = 6;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Woff2Options {
    pub quality: Option<u8>,
}

pub fn encode_ttf_to_woff2(input: &[u8], options: &Woff2Options) -> Result<Vec<u8>> {
    if !is_ttf(input) {
        return Err(FontminError::invalid_font(
            "expected TrueType sfnt data for WOFF2 encoding",
        ));
    }

    let quality = BrotliQuality::from(options.quality.unwrap_or(DEFAULT_BROTLI_QUALITY));

    ttf2woff2::encode(input, quality).map_err(woff2_error)
}

fn is_ttf(input: &[u8]) -> bool {
    input.starts_with(&[0x00, 0x01, 0x00, 0x00]) || input.starts_with(b"true")
}

fn woff2_error(error: ttf2woff2::Error) -> FontminError {
    match error {
        ttf2woff2::Error::UnsupportedFormat => {
            FontminError::invalid_font("expected TrueType sfnt data for WOFF2 encoding")
        }
        other => FontminError::ConvertFailed {
            message: format!("failed to encode WOFF2: {other}"),
        },
    }
}
