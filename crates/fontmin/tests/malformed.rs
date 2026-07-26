use std::panic::{AssertUnwindSafe, catch_unwind};

use fontmin::{
    CoverageOptions, EotOptions, Otf2TtfOptions, OutputFormat, SubsetOptions, Svg2TtfOptions,
    Ttf2SvgOptions, Woff2Options, WoffOptions,
};
use fontmin_testing::{MalformedCase, malformed_input, malformed_manifest};

fn assert_no_panic<T>(
    case: &MalformedCase,
    operation: &str,
    call: impl FnOnce() -> fontmin::Result<T>,
) {
    match catch_unwind(AssertUnwindSafe(call)) {
        Ok(Ok(_)) => {}
        Ok(Err(error)) => {
            assert!(
                error.diagnostic_code().starts_with("fontmin::"),
                "{} returned an unstable diagnostic code for {}",
                case.path,
                operation,
            );
            assert!(
                !error.to_string().is_empty(),
                "{} returned an empty diagnostic for {}",
                case.path,
                operation,
            );
        }
        Err(payload) => {
            let message = payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("non-string panic payload");
            panic!(
                "{} panicked in public API {operation}: {message}",
                case.path
            );
        }
    }
}

#[test]
fn malformed_inputs_do_not_panic_across_public_byte_apis() {
    for case in malformed_manifest().cases {
        let input = malformed_input(&case);

        assert_no_panic(&case, "inspect", || fontmin::inspect(&input));
        assert_no_panic(&case, "analyzeCoverage", || {
            fontmin::analyze_coverage(&input, CoverageOptions::default())
        });
        assert_no_panic(&case, "subsetTtf", || {
            fontmin::subset_ttf(&input, SubsetOptions::default())
        });
        assert_no_panic(&case, "woffToTtf", || fontmin::woff_to_ttf(&input));
        assert_no_panic(&case, "woff2ToTtf", || fontmin::woff2_to_ttf(&input));
        assert_no_panic(&case, "validateWoff2", || fontmin::validate_woff2(&input));
        assert_no_panic(&case, "eotToTtf", || fontmin::eot_to_ttf(&input));
        assert_no_panic(&case, "otfToTtf", || {
            fontmin::otf_to_ttf(&input, &Otf2TtfOptions::default())
        });
        assert_no_panic(&case, "ttfToSvg", || {
            fontmin::ttf_to_svg(&input, &Ttf2SvgOptions::default())
        });
        assert_no_panic(&case, "ttfToWoff", || {
            fontmin::ttf_to_woff(&input, &WoffOptions::default())
        });
        assert_no_panic(&case, "ttfToWoff2", || {
            fontmin::ttf_to_woff2(&input, &Woff2Options::default())
        });
        assert_no_panic(&case, "ttfToEot", || {
            fontmin::ttf_to_eot(&input, &EotOptions::default())
        });
        for target in [
            OutputFormat::Ttf,
            OutputFormat::Woff,
            OutputFormat::Woff2,
            OutputFormat::Eot,
            OutputFormat::Svg,
            OutputFormat::Css,
        ] {
            assert_no_panic(&case, "convert", || fontmin::convert(&input, target));
        }
        if let Ok(svg) = std::str::from_utf8(&input) {
            assert_no_panic(&case, "svgFontToTtf", || {
                fontmin::svg_font_to_ttf(svg, &Svg2TtfOptions::default())
            });
        }
    }
}

#[test]
fn malformed_manifest_locks_stable_diagnostics() {
    for case in malformed_manifest().cases {
        let input = malformed_input(&case);
        let error = match case.operation.as_str() {
            "inspect" => fontmin::inspect(&input).unwrap_err(),
            "otfToTtf" => fontmin::otf_to_ttf(&input, &Otf2TtfOptions::default()).unwrap_err(),
            operation => panic!("unsupported malformed manifest operation `{operation}`"),
        };

        assert_eq!(error.diagnostic_code(), case.expected_diagnostic.code);
        assert_eq!(error.to_string(), case.expected_diagnostic.message);
    }
}
