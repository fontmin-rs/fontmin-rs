#![allow(clippy::needless_pass_by_value)]

use std::collections::BTreeMap;

use serde::{Deserialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[must_use]
pub fn runtime_name() -> String {
    "fontmin-rs".to_owned()
}

pub enum TransformResult {
    Bytes(Vec<u8>),
    Empty,
    Json(serde_json::Value),
    Text(String),
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct WasmOtfOptions {
    preserve_hinting: bool,
    variation_coordinates: BTreeMap<String, f32>,
}

impl From<WasmOtfOptions> for fontmin::Otf2TtfOptions {
    fn from(options: WasmOtfOptions) -> Self {
        Self {
            preserve_hinting: options.preserve_hinting,
            variation_coordinates: options.variation_coordinates,
        }
    }
}

impl TransformResult {
    #[must_use]
    pub fn bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            Self::Empty | Self::Json(_) | Self::Text(_) => None,
        }
    }

    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Bytes(_) | Self::Empty | Self::Json(_) => None,
        }
    }
}

pub fn execute_binary(
    operation: &str,
    input: &[u8],
    options_json: &str,
) -> Result<TransformResult, String> {
    match operation {
        "analyzeCoverage" => fontmin::analyze_coverage(input, options(options_json)?)
            .map_err(error_message)
            .and_then(json_result),
        "subsetTtf" => fontmin::subset_ttf(input, options(options_json)?)
            .map(TransformResult::Bytes)
            .map_err(error_message),
        "ttfToWoff" => fontmin::ttf_to_woff(input, &options(options_json)?)
            .map(TransformResult::Bytes)
            .map_err(error_message),
        "woffToTtf" => fontmin::woff_to_ttf(input)
            .map(TransformResult::Bytes)
            .map_err(error_message),
        "ttfToWoff2" => fontmin::ttf_to_woff2(input, &options(options_json)?)
            .map(TransformResult::Bytes)
            .map_err(error_message),
        "woff2ToTtf" => fontmin::woff2_to_ttf(input)
            .map(TransformResult::Bytes)
            .map_err(error_message),
        "validateWoff2" => fontmin::validate_woff2(input)
            .map(|()| TransformResult::Empty)
            .map_err(error_message),
        "ttfToEot" => fontmin::ttf_to_eot(input, &options(options_json)?)
            .map(TransformResult::Bytes)
            .map_err(error_message),
        "eotToTtf" => fontmin::eot_to_ttf(input)
            .map(TransformResult::Bytes)
            .map_err(error_message),
        "ttfToSvg" => fontmin::ttf_to_svg(input, &options(options_json)?)
            .map(TransformResult::Text)
            .map_err(error_message),
        "otfToTtf" => fontmin::otf_to_ttf(
            input,
            &fontmin::Otf2TtfOptions::from(options::<WasmOtfOptions>(options_json)?),
        )
        .map(TransformResult::Bytes)
        .map_err(error_message),
        "inspect" => fontmin::inspect(input)
            .map_err(error_message)
            .and_then(json_result),
        _ => Err(format!("unsupported fontmin WASM operation `{operation}`")),
    }
}

pub fn execute_text(
    operation: &str,
    input: &str,
    options_json: &str,
) -> Result<TransformResult, String> {
    match operation {
        "svgFontToTtf" => fontmin::svg_font_to_ttf(input, &options(options_json)?)
            .map(TransformResult::Bytes)
            .map_err(error_message),
        _ => Err(format!(
            "unsupported fontmin WASM text operation `{operation}`"
        )),
    }
}

pub fn execute_icons(inputs_json: &str, options_json: &str) -> Result<TransformResult, String> {
    let inputs = parse(inputs_json)?;
    let options = options(options_json)?;

    fontmin::svgs_to_ttf(inputs, &options)
        .map(TransformResult::Bytes)
        .map_err(error_message)
}

pub fn execute_css(sources_json: &str, options_json: &str) -> Result<TransformResult, String> {
    let sources: Vec<fontmin::CssFontSource> = parse(sources_json)?;
    let options: fontmin::CssOptions = parse(options_json)?;

    fontmin::generate_font_face_css(&sources, &options)
        .map(TransformResult::Text)
        .map_err(error_message)
}

#[wasm_bindgen]
pub fn transform(operation: String, input: Vec<u8>, options: JsValue) -> Result<JsValue, JsValue> {
    if operation == "inspect" {
        let info =
            fontmin::inspect(&input).map_err(|error| JsValue::from_str(&error_message(error)))?;

        return serde_wasm_bindgen::to_value(&info).map_err(|error| {
            JsValue::from_str(&format!("failed to serialize WASM result: {error}"))
        });
    }

    if operation == "analyzeCoverage" {
        let options_json = options_to_json(options)?;
        let options = parse::<fontmin::CoverageOptions>(&options_json)
            .map_err(|error| JsValue::from_str(&error))?;
        let report = fontmin::analyze_coverage(&input, options)
            .map_err(|error| JsValue::from_str(&error_message(error)))?;

        return serde_wasm_bindgen::to_value(&report).map_err(|error| {
            JsValue::from_str(&format!("failed to serialize WASM result: {error}"))
        });
    }

    let options_json = options_to_json(options)?;
    result_to_js(execute_binary(&operation, &input, &options_json))
}

#[wasm_bindgen]
pub fn transform_text(
    operation: String,
    input: String,
    options: JsValue,
) -> Result<JsValue, JsValue> {
    let options_json = options_to_json(options)?;
    result_to_js(execute_text(&operation, &input, &options_json))
}

#[wasm_bindgen]
pub fn transform_icons(inputs: JsValue, options: JsValue) -> Result<JsValue, JsValue> {
    result_to_js(execute_icons(
        &options_to_json(inputs)?,
        &options_to_json(options)?,
    ))
}

#[wasm_bindgen]
pub fn generate_css(sources: JsValue, options_value: JsValue) -> Result<JsValue, JsValue> {
    let sources = serde_wasm_bindgen::from_value::<Vec<fontmin::CssFontSource>>(sources)
        .map_err(|error| JsValue::from_str(&format!("invalid WASM CSS sources: {error}")))?;
    let options_json = options_to_json(options_value)?;
    let options =
        parse::<fontmin::CssOptions>(&options_json).map_err(|error| JsValue::from_str(&error))?;

    fontmin::generate_font_face_css(&sources, &options)
        .map(|css| JsValue::from_str(&css))
        .map_err(|error| JsValue::from_str(&error_message(error)))
}

fn options<T: DeserializeOwned + Default>(json: &str) -> Result<T, String> {
    parse(json)
}

fn parse<T: DeserializeOwned>(json: &str) -> Result<T, String> {
    serde_json::from_str(json).map_err(|error| format!("invalid WASM options: {error}"))
}

fn error_message(error: fontmin::FontminError) -> String {
    error.to_string()
}

fn json_result<T: serde::Serialize>(value: T) -> Result<TransformResult, String> {
    serde_json::to_value(value)
        .map(TransformResult::Json)
        .map_err(|error| format!("failed to serialize WASM result: {error}"))
}

fn options_to_json(value: JsValue) -> Result<String, JsValue> {
    let value = serde_wasm_bindgen::from_value::<serde_json::Value>(value)
        .map_err(|error| JsValue::from_str(&format!("invalid WASM options: {error}")))?;

    serde_json::to_string(&value)
        .map_err(|error| JsValue::from_str(&format!("invalid WASM options: {error}")))
}

fn result_to_js(result: Result<TransformResult, String>) -> Result<JsValue, JsValue> {
    let value = match result {
        Ok(TransformResult::Bytes(bytes)) => serde_wasm_bindgen::to_value(&bytes),
        Ok(TransformResult::Empty) => Ok(JsValue::UNDEFINED),
        Ok(TransformResult::Json(value)) => serde_wasm_bindgen::to_value(&value),
        Ok(TransformResult::Text(text)) => Ok(JsValue::from_str(&text)),
        Err(message) => return Err(JsValue::from_str(&message)),
    };

    value.map_err(|error| JsValue::from_str(&format!("failed to serialize WASM result: {error}")))
}

#[cfg(test)]
mod tests {
    use fontmin_testing::ROBOTO;

    use super::{TransformResult, execute_binary, execute_css, parse};

    #[test]
    fn dispatches_ttf_conversions_without_native_bindings() {
        let woff = execute_binary("ttfToWoff", ROBOTO, "{}").unwrap();
        let woff2 = execute_binary("ttfToWoff2", ROBOTO, "{}").unwrap();
        let eot = execute_binary("ttfToEot", ROBOTO, "{}").unwrap();
        let svg = execute_binary("ttfToSvg", ROBOTO, "{}").unwrap();

        assert!(woff.bytes().is_some_and(|bytes| bytes.starts_with(b"wOFF")));
        assert!(
            woff2
                .bytes()
                .is_some_and(|bytes| bytes.starts_with(b"wOF2"))
        );
        assert!(eot.bytes().is_some_and(|bytes| bytes.len() > ROBOTO.len()));
        assert!(svg.text().is_some_and(|text| text.starts_with("<svg")));
    }

    #[test]
    fn forwards_unicode_ranges_to_css_generation() {
        let options = parse::<fontmin::CssOptions>(r#"{"unicodeRanges":["U+0020-007E"]}"#).unwrap();
        assert_eq!(options.unicode_ranges.len(), 1);

        let css = execute_css(
            r#"[{"fileName":"roboto.woff2","format":"woff2"}]"#,
            r#"{"unicodeRanges":["U+0020-007E"]}"#,
        )
        .unwrap();

        assert!(matches!(
            css,
            TransformResult::Text(ref value) if value.contains("unicode-range: U+0020-007E;")
        ));
    }

    #[test]
    fn forwards_unicode_ranges_to_ttf_subsetting() {
        let options =
            parse::<fontmin::SubsetOptions>(r#"{"unicodeRanges":["U+0041-0042"]}"#).unwrap();
        assert_eq!(options.unicode_ranges.len(), 1);

        let subset =
            execute_binary("subsetTtf", ROBOTO, r#"{"unicodeRanges":["U+0041-0042"]}"#).unwrap();

        assert!(
            subset
                .bytes()
                .is_some_and(|bytes| bytes.len() < ROBOTO.len())
        );
    }

    #[test]
    fn dispatches_coverage_analysis_as_json() {
        let report = execute_binary("analyzeCoverage", ROBOTO, r#"{"text":"A𠮷"}"#).unwrap();

        assert!(matches!(
            report,
            TransformResult::Json(ref value)
                if value["supported"] == serde_json::json!([65])
                    && value["missing"] == serde_json::json!([134_071])
        ));
    }

    #[test]
    fn rejects_invalid_unicode_ranges_for_css_generation() {
        let result = execute_css(
            r#"[{"fileName":"roboto.woff2","format":"woff2"}]"#,
            r#"{"unicodeRanges":["U+4??"]}"#,
        );

        assert!(matches!(
            result,
            Err(ref error) if error.contains("invalid Unicode range: U+4??")
        ));
    }

    #[test]
    fn rejects_invalid_option_types_instead_of_using_defaults() {
        let result = execute_binary("ttfToWoff2", ROBOTO, r#"{"quality":"high"}"#);

        assert!(matches!(
            result,
            Err(ref error)
                if error.contains("invalid WASM options")
                    && error.contains("invalid type: string")
        ));
    }

    #[test]
    fn rejects_invalid_option_enums_instead_of_using_defaults() {
        let result = execute_binary("subsetTtf", ROBOTO, r#"{"text":"A","layout":"aggressive"}"#);

        assert!(matches!(
            result,
            Err(ref error)
                if error.contains("invalid WASM options")
                    && error.contains("unknown variant `aggressive`")
        ));
    }

    #[test]
    fn rejects_invalid_unicode_range_options_instead_of_using_defaults() {
        let result = execute_binary("subsetTtf", ROBOTO, r#"{"unicodeRanges":["U+4??"]}"#);

        assert!(matches!(
            result,
            Err(ref error)
                if error.contains("invalid WASM options")
                    && error.contains("invalid Unicode range: U+4??")
        ));
    }

    #[test]
    fn accepts_partial_options_with_field_defaults() {
        let result = execute_binary("ttfToWoff", ROBOTO, r#"{"deflate":false}"#);

        assert!(result.is_ok());
    }
}
