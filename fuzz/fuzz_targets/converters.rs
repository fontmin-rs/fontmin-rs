#![no_main]

use fontmin::{
    EotOptions, MissingGlyphPolicy, Otf2TtfOptions, SubsetOptions, Svg2TtfOptions, Ttf2SvgOptions,
    Woff2Options, WoffOptions,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&operation, input)) = data.split_first() else {
        return;
    };
    if input.len() > 1_048_576 {
        return;
    }

    match operation % 7 {
        0 => {
            let _ = fontmin::subset_ttf(
                input,
                SubsetOptions {
                    missing_glyphs: MissingGlyphPolicy::Ignore,
                    text: Some("A中".into()),
                    ..SubsetOptions::default()
                },
            );
        }
        1 => {
            if let Ok(woff) = fontmin::ttf_to_woff(input, &WoffOptions::default()) {
                let _ = fontmin::woff_to_ttf(&woff);
            }
        }
        2 => {
            if let Ok(woff2) = fontmin::ttf_to_woff2(input, &Woff2Options::default()) {
                let _ = fontmin::woff2_to_ttf(&woff2);
            }
        }
        3 => {
            if let Ok(eot) = fontmin::ttf_to_eot(input, &EotOptions::default()) {
                let _ = fontmin::eot_to_ttf(&eot);
            }
        }
        4 => {
            let _ = fontmin::otf_to_ttf(input, &Otf2TtfOptions::default());
        }
        5 => {
            if let Ok(svg) = std::str::from_utf8(input) {
                let _ = fontmin::svg_font_to_ttf(svg, &Svg2TtfOptions::default());
            }
        }
        6 => {
            let _ = fontmin::ttf_to_svg(input, &Ttf2SvgOptions::default());
        }
        _ => unreachable!(),
    }
});
