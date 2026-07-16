use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};

use crate::UnicodeRange;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CoverageOptions {
    pub text: Option<String>,
    pub unicodes: Vec<u32>,
    pub unicode_ranges: Vec<UnicodeRange>,
    pub basic_text: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MissingGlyphPolicy {
    Ignore,
    #[default]
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageReport {
    pub requested: Vec<u32>,
    pub supported: Vec<u32>,
    pub missing: Vec<u32>,
    pub coverage_percent: f64,
}

impl CoverageReport {
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn new(requested: Vec<u32>, supported: Vec<u32>, missing: Vec<u32>) -> Self {
        let coverage_percent = if requested.is_empty() {
            0.0
        } else {
            supported.len() as f64 / requested.len() as f64 * 100.0
        };

        Self {
            requested,
            supported,
            missing,
            coverage_percent,
        }
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty()
    }

    pub fn ensure_complete(&self) -> Result<()> {
        if self.is_complete() {
            Ok(())
        } else {
            Err(FontminError::missing_glyphs(&self.missing))
        }
    }

    #[must_use]
    pub fn missing_glyph_message(&self) -> Option<String> {
        (!self.missing.is_empty()).then(|| FontminError::missing_glyphs(&self.missing).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::CoverageReport;

    #[test]
    fn reports_percentage_and_completeness() {
        let report = CoverageReport::new(vec![0x41, 0x42], vec![0x41], vec![0x42]);

        assert!((report.coverage_percent - 50.0).abs() < f64::EPSILON);
        assert!(!report.is_complete());
        assert!(report.ensure_complete().is_err());
    }
}
