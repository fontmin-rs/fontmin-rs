pub use fontmin_config::FontminConfig;
pub use fontmin_core::{Asset, FontFormat, OutputFormat};
pub use fontmin_diagnostics::{FontminError, Result};
pub use fontmin_subset::{LayoutSubsetMode, SubsetOptions};

pub fn subset_ttf(input: &[u8], options: SubsetOptions) -> Result<Vec<u8>> {
    fontmin_subset::subset_ttf(input, options)
}

pub fn convert(input: &[u8], target: OutputFormat) -> Result<Vec<u8>> {
    match target {
        OutputFormat::Ttf => Ok(input.to_vec()),
        OutputFormat::Woff => Err(FontminError::unsupported("woff")),
        OutputFormat::Woff2 => Err(FontminError::unsupported("woff2")),
        OutputFormat::Eot => Err(FontminError::unsupported("eot")),
        OutputFormat::Svg => Err(FontminError::unsupported("svg")),
        OutputFormat::Css => Err(FontminError::unsupported("css")),
    }
}

#[cfg(test)]
mod tests {
    use fontmin_core::OutputFormat;
    use fontmin_diagnostics::FontminErrorKind;

    use super::convert;

    #[test]
    fn ttf_convert_keeps_bytes_for_now() {
        assert_eq!(convert(b"abc", OutputFormat::Ttf).unwrap(), b"abc");
    }

    #[test]
    fn unsupported_conversions_return_typed_errors() {
        let error = convert(b"abc", OutputFormat::Woff2).unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::UnsupportedFormat);
    }
}
