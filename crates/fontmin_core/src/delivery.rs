use std::collections::BTreeSet;

use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};

use crate::UnicodeRange;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontDeliverySlice {
    pub name: String,
    pub unicode_ranges: Vec<UnicodeRange>,
}

pub fn validate_delivery_slices(slices: &[FontDeliverySlice]) -> Result<()> {
    let mut names = BTreeSet::new();

    for slice in slices {
        if slice.name.is_empty()
            || !slice
                .name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(FontminError::config(format!(
                "invalid delivery slice name: {}",
                slice.name
            )));
        }

        if slice.unicode_ranges.is_empty() {
            return Err(FontminError::config(format!(
                "delivery slice `{}` requires at least one Unicode range",
                slice.name
            )));
        }

        if !names.insert(&slice.name) {
            return Err(FontminError::config(format!(
                "duplicate delivery slice name: {}",
                slice.name
            )));
        }
    }

    Ok(())
}
