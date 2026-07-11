use fontmin::{LayoutSubsetMode, SubsetOptions};
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi(object)]
pub struct JsSubsetOptions {
    pub text: Option<String>,
    pub unicodes: Option<Vec<u32>>,
    pub basic_text: Option<bool>,
    pub preserve_hinting: Option<bool>,
    pub trim: Option<bool>,
    pub keep_notdef: Option<bool>,
    pub keep_layout: Option<String>,
}

#[napi(js_name = "subsetTtf")]
pub fn subset_ttf(input: Buffer, options: Option<JsSubsetOptions>) -> napi::Result<Buffer> {
    let options = subset_options_from_js(options)?;
    let output = fontmin::subset_ttf(&input, options)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;

    Ok(output.into())
}

fn subset_options_from_js(options: Option<JsSubsetOptions>) -> napi::Result<SubsetOptions> {
    let Some(options) = options else {
        return Ok(SubsetOptions::default());
    };

    Ok(SubsetOptions {
        text: options.text,
        unicodes: options.unicodes.unwrap_or_default(),
        basic_text: options.basic_text.unwrap_or(false),
        preserve_hinting: options.preserve_hinting.unwrap_or(false),
        trim: options.trim.unwrap_or(true),
        keep_notdef: options.keep_notdef.unwrap_or(true),
        layout: layout_mode_from_js(options.keep_layout)?,
    })
}

fn layout_mode_from_js(value: Option<String>) -> napi::Result<LayoutSubsetMode> {
    match value.as_deref().unwrap_or("conservative") {
        "drop" => Ok(LayoutSubsetMode::Drop),
        "conservative" => Ok(LayoutSubsetMode::Conservative),
        "preserve" => Ok(LayoutSubsetMode::Preserve),
        other => Err(napi::Error::from_reason(format!(
            "unknown keepLayout value: {other}",
        ))),
    }
}
