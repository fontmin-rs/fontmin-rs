pub const ROBOTO: &[u8] = include_bytes!("../../../fixtures/fonts/ttf/roboto-regular.ttf");
pub const SOURCE_SANS_3_REGULAR_CFF: &[u8] =
    include_bytes!("../../../fixtures/fonts/otf/source-sans-3-regular.otf");
pub const SOURCE_SERIF_4_VARIABLE_CFF2: &[u8] =
    include_bytes!("../../../fixtures/fonts/otf/source-serif-4-variable-roman.otf");

pub const HOME_ICON: &str = r#"<svg viewBox="0 0 1000 1000"><path d="M100 500 L500 100 L900 500 L900 900 L100 900 Z"/></svg>"#;

pub const USER_ICON: &str = r#"<svg viewBox="0 0 1000 1000"><path d="M500 100 C620 100 700 180 700 300 C700 420 620 500 500 500 C380 500 300 420 300 300 C300 180 380 100 500 100 Z M250 900 Q500 650 750 900 Z"/></svg>"#;

pub const SVG_FONT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><defs><font id="icons" horiz-adv-x="1000"><font-face font-family="SVG Icons" units-per-em="1000" ascent="850" descent="-150" /><missing-glyph horiz-adv-x="1000" /><glyph glyph-name="home" unicode="&#xE101;" horiz-adv-x="1000" d="M100 100 L900 100 L900 900 L100 900 Z" /><glyph glyph-name="user" unicode="&#xE102;" horiz-adv-x="1000" d="M500 100 C620 100 700 180 700 300 C700 420 620 500 500 500 C380 500 300 420 300 300 C300 180 380 100 500 100 Z" /></font></defs></svg>"#;

pub const LARGE_SVG_FONT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><defs><font id="large" horiz-adv-x="2000"><font-face font-family="Large Icons" units-per-em="2000" ascent="1600" descent="-400" /><glyph glyph-name="box" unicode="&#xE101;" horiz-adv-x="2000" d="M200 200 L1800 200 L1800 1800 L200 1800 Z" /></font></defs></svg>"#;

#[must_use]
pub fn roboto_otf() -> Vec<u8> {
    let mut otf = ROBOTO.to_vec();

    otf[0..4].copy_from_slice(b"OTTO");

    otf
}

#[cfg(test)]
mod tests {
    use super::{
        HOME_ICON, LARGE_SVG_FONT, ROBOTO, SOURCE_SANS_3_REGULAR_CFF, SOURCE_SERIF_4_VARIABLE_CFF2,
        SVG_FONT, USER_ICON, roboto_otf,
    };

    #[test]
    fn exposes_shared_font_fixtures() {
        assert!(ROBOTO.starts_with(&[0x00, 0x01, 0x00, 0x00]));
        assert!(HOME_ICON.contains("<svg"));
        assert!(USER_ICON.contains("<svg"));
        assert!(SVG_FONT.contains("<font"));
        assert!(LARGE_SVG_FONT.contains("Large Icons"));
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
}
