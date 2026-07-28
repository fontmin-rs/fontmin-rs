#![no_main]

use fontmin::{CoverageOptions, Otf2TtfOptions};
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
            let _ = fontmin::inspect(input);
        }
        1 => {
            let _ = fontmin::analyze_coverage(input, CoverageOptions::default());
        }
        2 => {
            let _ = fontmin::woff_to_ttf(input);
        }
        3 => {
            let _ = fontmin::woff2_to_ttf(input);
        }
        4 => {
            let _ = fontmin::validate_woff2(input);
        }
        5 => {
            let _ = fontmin::eot_to_ttf(input);
        }
        6 => {
            let _ = fontmin::otf_to_ttf(input, &Otf2TtfOptions::default());
        }
        _ => unreachable!(),
    }
});
