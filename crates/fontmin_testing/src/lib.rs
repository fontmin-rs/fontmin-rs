use std::path::PathBuf;

use serde::Deserialize;

pub const ESTEDAD_VARIABLE: &[u8] =
    include_bytes!("../../../fixtures/fonts/ttf/estedad-variable.ttf");
pub const FONT_AWESOME_FREE_SOLID: &[u8] =
    include_bytes!("../../../fixtures/fonts/otf/font-awesome-free-solid-900.otf");
pub const NOTO_SANS_SC_COMPACT: &[u8] =
    include_bytes!("../../../fixtures/fonts/ttf/noto-sans-sc-compact.ttf");
pub const NOTO_SANS_SC_VARIABLE_COMPACT: &[u8] =
    include_bytes!("../../../fixtures/fonts/ttf/noto-sans-sc-variable-compact.ttf");
pub const ROBOTO: &[u8] = include_bytes!("../../../fixtures/fonts/ttf/roboto-regular.ttf");
pub const SOURCE_SANS_3_REGULAR_CFF: &[u8] =
    include_bytes!("../../../fixtures/fonts/otf/source-sans-3-regular.otf");
pub const SOURCE_SERIF_4_VARIABLE_CFF2: &[u8] =
    include_bytes!("../../../fixtures/fonts/otf/source-serif-4-variable-roman.otf");

pub const HOME_ICON: &str = r#"<svg viewBox="0 0 1000 1000"><path d="M100 500 L500 100 L900 500 L900 900 L100 900 Z"/></svg>"#;

pub const USER_ICON: &str = r#"<svg viewBox="0 0 1000 1000"><path d="M500 100 C620 100 700 180 700 300 C700 420 620 500 500 500 C380 500 300 420 300 300 C300 180 380 100 500 100 Z M250 900 Q500 650 750 900 Z"/></svg>"#;

pub const SVG_FONT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><defs><font id="icons" horiz-adv-x="1000"><font-face font-family="SVG Icons" units-per-em="1000" ascent="850" descent="-150" /><missing-glyph horiz-adv-x="1000" /><glyph glyph-name="home" unicode="&#xE101;" horiz-adv-x="1000" d="M100 100 L900 100 L900 900 L100 900 Z" /><glyph glyph-name="user" unicode="&#xE102;" horiz-adv-x="1000" d="M500 100 C620 100 700 180 700 300 C700 420 620 500 500 500 C380 500 300 420 300 300 C300 180 380 100 500 100 Z" /></font></defs></svg>"#;

pub const LARGE_SVG_FONT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><defs><font id="large" horiz-adv-x="2000"><font-face font-family="Large Icons" units-per-em="2000" ascent="1600" descent="-400" /><glyph glyph-name="box" unicode="&#xE101;" horiz-adv-x="2000" d="M200 200 L1800 200 L1800 1800 L200 1800 Z" /></font></defs></svg>"#;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MalformedManifest {
    pub schema_version: u32,
    pub cases: Vec<MalformedCase>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MalformedCase {
    pub path: String,
    pub encoding: Option<String>,
    pub operation: String,
    pub expected_diagnostic: ExpectedDiagnostic,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExpectedDiagnostic {
    pub code: String,
    pub message: String,
}

#[must_use]
pub fn malformed_manifest() -> MalformedManifest {
    let path = workspace_root().join("fixtures/malformed/manifest.json");
    let manifest = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    serde_json::from_str(&manifest)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

#[must_use]
pub fn malformed_input(case: &MalformedCase) -> Vec<u8> {
    let path = workspace_root().join(&case.path);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    match case.encoding.as_deref() {
        None => bytes,
        Some("hex") => decode_hex(&path, &bytes),
        Some(encoding) => panic!(
            "unsupported fixture encoding `{encoding}` for {}",
            case.path
        ),
    }
}

#[must_use]
pub fn roboto_otf() -> Vec<u8> {
    let mut otf = ROBOTO.to_vec();

    otf[0..4].copy_from_slice(b"OTTO");

    otf
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn decode_hex(path: &std::path::Path, bytes: &[u8]) -> Vec<u8> {
    let hex = std::str::from_utf8(bytes)
        .unwrap_or_else(|error| panic!("{} is not UTF-8 hex: {error}", path.display()))
        .trim();
    assert!(
        hex.len().is_multiple_of(2),
        "{} contains an odd number of hex digits",
        path.display()
    );

    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("hex pairs are valid UTF-8");
            u8::from_str_radix(pair, 16)
                .unwrap_or_else(|error| panic!("invalid hex in {}: {error}", path.display()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        ESTEDAD_VARIABLE, FONT_AWESOME_FREE_SOLID, HOME_ICON, LARGE_SVG_FONT, NOTO_SANS_SC_COMPACT,
        NOTO_SANS_SC_VARIABLE_COMPACT, ROBOTO, SOURCE_SANS_3_REGULAR_CFF,
        SOURCE_SERIF_4_VARIABLE_CFF2, SVG_FONT, USER_ICON, malformed_input, malformed_manifest,
        roboto_otf,
    };

    #[test]
    fn exposes_shared_font_fixtures() {
        assert!(ROBOTO.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert!(ESTEDAD_VARIABLE.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert!(HOME_ICON.contains("<svg"));
        assert!(USER_ICON.contains("<svg"));
        assert!(SVG_FONT.contains("<font"));
        assert!(LARGE_SVG_FONT.contains("Large Icons"));
        assert!(NOTO_SANS_SC_COMPACT.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert!(NOTO_SANS_SC_VARIABLE_COMPACT.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert!(
            NOTO_SANS_SC_VARIABLE_COMPACT
                .windows(4)
                .any(|tag| tag == b"fvar")
        );
        assert!(
            NOTO_SANS_SC_VARIABLE_COMPACT
                .windows(4)
                .any(|tag| tag == b"gvar")
        );
        assert!(FONT_AWESOME_FREE_SOLID.starts_with(b"OTTO"));
    }

    #[test]
    fn creates_glyf_backed_otf_wrapper() {
        let otf = roboto_otf();

        assert!(otf.starts_with(b"OTTO"));
        assert_eq!(&otf[4..], &ROBOTO[4..]);
    }

    #[test]
    fn exposes_static_cff_fixture() {
        assert!(SOURCE_SANS_3_REGULAR_CFF.starts_with(b"OTTO"));
        assert!(
            SOURCE_SANS_3_REGULAR_CFF
                .windows(4)
                .any(|tag| tag == b"CFF ")
        );
    }

    #[test]
    fn exposes_cff2_variable_fixture() {
        assert!(SOURCE_SERIF_4_VARIABLE_CFF2.starts_with(b"OTTO"));
        assert!(
            SOURCE_SERIF_4_VARIABLE_CFF2
                .windows(4)
                .any(|tag| tag == b"CFF2")
        );
        assert!(
            SOURCE_SERIF_4_VARIABLE_CFF2
                .windows(4)
                .any(|tag| tag == b"fvar")
        );
    }

    #[test]
    fn loads_the_shared_malformed_manifest() {
        let manifest = malformed_manifest();

        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.cases.len(), 9);
        assert!(
            manifest
                .cases
                .iter()
                .all(|case| case.expected_diagnostic.code.starts_with("fontmin::"))
        );
    }

    #[test]
    fn decodes_hex_encoded_malformed_fixtures() {
        let manifest = malformed_manifest();
        let case = manifest
            .cases
            .iter()
            .find(|case| case.encoding.as_deref() == Some("hex"))
            .expect("manifest should contain a hex fixture");

        assert!(malformed_input(case).starts_with(b"OTTO"));
    }
}
