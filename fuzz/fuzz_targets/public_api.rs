#![no_main]

use fontmin::{
    CoverageOptions, EotOptions, MissingGlyphPolicy, Otf2TtfOptions, SubsetOptions, Svg2TtfOptions,
    Ttf2SvgOptions, Woff2Options, WoffOptions,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&operation, input)) = data.split_first() else {
        return;
    };
    if input.len() > 1_048_576 {
        return;
    }

    match operation % 13 {
        0 => {
            let _ = fontmin::inspect(input);
        }
        1 => {
            let _ = fontmin::analyze_coverage(
                input,
                CoverageOptions {
                    text: Some("A中".into()),
                    ..CoverageOptions::default()
                },
            );
        }
        2 => {
            let _ = fontmin::subset_ttf(
                input,
                SubsetOptions {
                    missing_glyphs: MissingGlyphPolicy::Ignore,
                    text: Some("A中".into()),
                    ..SubsetOptions::default()
                },
            );
        }
        3 => {
            if let Ok(woff) = fontmin::ttf_to_woff(input, &WoffOptions::default()) {
                let _ = fontmin::woff_to_ttf(&woff);
            } else {
                let _ = fontmin::woff_to_ttf(input);
            }
        }
        4 => {
            if let Ok(woff2) = fontmin::ttf_to_woff2(input, &Woff2Options::default()) {
                let _ = fontmin::woff2_to_ttf(&woff2);
            } else {
                let _ = fontmin::woff2_to_ttf(input);
            }
        }
        5 => {
            let _ = fontmin::validate_woff2(input);
        }
        6 => {
            if let Ok(eot) = fontmin::ttf_to_eot(input, &EotOptions::default()) {
                let _ = fontmin::eot_to_ttf(&eot);
            } else {
                let _ = fontmin::eot_to_ttf(input);
            }
        }
        7 => {
            let _ = fontmin::otf_to_ttf(input, &Otf2TtfOptions::default());
        }
        8 => {
            if let Ok(svg) = std::str::from_utf8(input) {
                let _ = fontmin::svg_font_to_ttf(svg, &Svg2TtfOptions::default());
            }
        }
        9 => {
            let _ = fontmin::ttf_to_svg(input, &Ttf2SvgOptions::default());
        }
        10 => {
            let _ = fontmin::ttf_to_woff(input, &WoffOptions::default());
        }
        11 => {
            let _ = fontmin::ttf_to_woff2(input, &Woff2Options::default());
        }
        12 => {
            let _ = fontmin::ttf_to_eot(input, &EotOptions::default());
        }
        _ => unreachable!(),
    }
});
