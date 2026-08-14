//! Variable-font axis pinning and range reduction.

use std::{collections::BTreeMap, sync::OnceLock};

use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};
use wasmi::{Engine, Linker, Module, Store};

const HARFBUZZ_SUBSET_WASM: &[u8] =
    include_bytes!("../../../vendor/harfbuzz-subset-wasm/harfbuzz-subset.wasm");
const HB_MEMORY_MODE_WRITABLE: i32 = 2;
const HB_SUBSET_FLAGS_DOWNGRADE_CFF2: i32 = 0x0000_4000;

/// Coordinates for converting a variable TrueType font into one static face.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct InstanceOptions {
    /// Axis values in `fvar` user units. Unspecified axes use their defaults.
    pub variation_coordinates: BTreeMap<String, f32>,
}

/// A reduced inclusive range for one variable-font axis, in `fvar` user units.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AxisRange {
    pub min: f32,
    pub max: f32,
    /// New default, or the original default clamped into `min..=max` when omitted.
    pub default: Option<f32>,
}

/// Pin one axis or retain it with a smaller variation range.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AxisSetting {
    Pin(f32),
    Range(AxisRange),
}

/// Options for reducing a variable font's design space.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct VariationSpaceOptions {
    /// Axis tags mapped to a pinned value or retained range. Unlisted axes remain variable.
    pub axes: BTreeMap<String, AxisSetting>,
    /// Convert a fully pinned CFF2 font to CFF1 for older renderers.
    pub downgrade_cff2: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct VariationAxis {
    tag: [u8; 4],
    min: f64,
    default: f64,
    max: f64,
}

#[derive(Debug, Clone, Copy)]
enum ValidatedAxisSetting {
    Pin {
        tag: [u8; 4],
        value: f32,
    },
    Range {
        tag: [u8; 4],
        min: f32,
        max: f32,
        default: f32,
    },
}

struct CompiledRuntime {
    engine: Engine,
    module: Module,
}

static COMPILED_RUNTIME: OnceLock<std::result::Result<CompiledRuntime, String>> = OnceLock::new();

/// Pin selected axes and/or narrow retained axis ranges.
///
/// Unlisted axes remain variable. `HarfBuzz` rewrites all affected outline,
/// metric, layout, variation, axis, statistics, and naming tables together.
pub fn reduce_variation_space(input: &[u8], options: &VariationSpaceOptions) -> Result<Vec<u8>> {
    if options.axes.is_empty() {
        return Err(FontminError::config(
            "variable font reduction requires at least one axis setting",
        ));
    }

    let font = fontmin_ttf::read_sfnt_table_directory(input)?;
    let fvar = font
        .iter()
        .find(|table| table.tag == "fvar")
        .and_then(|table| {
            let start = table.offset;
            let len = table.length;
            input.get(start..start.checked_add(len)?)
        })
        .ok_or_else(|| FontminError::unsupported("static OpenType font without fvar axes"))?;
    let axes = parse_variation_axes(fvar)?;
    let settings = validate_axis_settings(&axes, &options.axes)?;
    let output = run_harfbuzz(input, &settings, options.downgrade_cff2)?;

    fontmin_ttf::read_sfnt_table_directory(&output)
        .map_err(|error| FontminError::convert_failed(error.to_string()))?;
    if fontmin_ttf::calculate_table_checksum(&output) != 0xB1B0_AFBA {
        return Err(FontminError::convert_failed(
            "reduced variable font checksum adjustment is invalid",
        ));
    }

    Ok(output)
}

pub(crate) fn parse_variation_axes(fvar: &[u8]) -> Result<Vec<VariationAxis>> {
    let read_u16 = |offset: usize| {
        fvar.get(offset..offset + 2)
            .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
    };
    let read_fixed = |offset: usize| {
        fvar.get(offset..offset + 4).map(|bytes| {
            f64::from(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])) / 65_536.0
        })
    };
    let invalid =
        |message: &str| FontminError::invalid_font(format!("invalid fvar table: {message}"));

    if read_u16(0) != Some(1) || read_u16(2) != Some(0) {
        return Err(invalid("unsupported or truncated version"));
    }
    let axes_offset = usize::from(read_u16(4).ok_or_else(|| invalid("truncated header"))?);
    let axis_count = usize::from(read_u16(8).ok_or_else(|| invalid("truncated header"))?);
    let axis_size = usize::from(read_u16(10).ok_or_else(|| invalid("truncated header"))?);
    if axis_count == 0 {
        return Err(invalid("axisCount is zero"));
    }
    if axis_size < 20 {
        return Err(invalid("axisSize is smaller than 20 bytes"));
    }
    let axes_end = axes_offset
        .checked_add(
            axis_count
                .checked_mul(axis_size)
                .ok_or_else(|| invalid("axis array overflows"))?,
        )
        .ok_or_else(|| invalid("axis array overflows"))?;
    if axes_end > fvar.len() {
        return Err(invalid("axis array is truncated"));
    }

    let mut axes = Vec::with_capacity(axis_count);
    for index in 0..axis_count {
        let offset = axes_offset + index * axis_size;
        let tag = fvar
            .get(offset..offset + 4)
            .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
            .ok_or_else(|| invalid("axis tag is truncated"))?;
        let min = read_fixed(offset + 4).ok_or_else(|| invalid("axis minimum is truncated"))?;
        let default = read_fixed(offset + 8).ok_or_else(|| invalid("axis default is truncated"))?;
        let max = read_fixed(offset + 12).ok_or_else(|| invalid("axis maximum is truncated"))?;
        if !min.is_finite()
            || !default.is_finite()
            || !max.is_finite()
            || min > default
            || default > max
        {
            return Err(invalid("axis range is invalid"));
        }
        if axes.iter().any(|axis: &VariationAxis| axis.tag == tag) {
            return Err(invalid("axis tags are duplicated"));
        }
        axes.push(VariationAxis {
            tag,
            min,
            default,
            max,
        });
    }

    Ok(axes)
}

pub(crate) fn validate_variation_coordinates(
    axes: &[VariationAxis],
    coordinates: &BTreeMap<String, f32>,
) -> Result<Vec<([u8; 4], f32)>> {
    coordinates
        .iter()
        .map(|(tag, &value)| {
            let tag_bytes = parse_axis_tag(tag).ok_or_else(|| {
                FontminError::invalid_font(format!("unknown variation axis `{tag}`"))
            })?;
            let selected_axis =
                axes.iter()
                    .find(|axis| axis.tag == tag_bytes)
                    .ok_or_else(|| {
                        FontminError::invalid_font(format!("unknown variation axis `{tag}`"))
                    })?;
            if !axis_value_in_range(selected_axis, value) {
                return Err(FontminError::invalid_font(format!(
                    "variation axis `{tag}` value {value} is outside [{}, {}]",
                    selected_axis.min, selected_axis.max,
                )));
            }

            Ok((selected_axis.tag, value))
        })
        .collect()
}

fn validate_axis_settings(
    axes: &[VariationAxis],
    settings: &BTreeMap<String, AxisSetting>,
) -> Result<Vec<ValidatedAxisSetting>> {
    settings
        .iter()
        .map(|(tag, setting)| {
            let selected_axis = find_axis(axes, tag)?;

            match *setting {
                AxisSetting::Pin(value) => {
                    validate_value(selected_axis, tag, value)?;
                    Ok(ValidatedAxisSetting::Pin {
                        tag: selected_axis.tag,
                        value,
                    })
                }
                AxisSetting::Range(range) => {
                    validate_value(selected_axis, tag, range.min)?;
                    validate_value(selected_axis, tag, range.max)?;
                    if range.min > range.max {
                        return Err(FontminError::config(format!(
                            "variation axis `{tag}` range minimum {} exceeds maximum {}",
                            range.min, range.max,
                        )));
                    }
                    let effective_default = range.default.unwrap_or_else(|| {
                        axis_default_f32(selected_axis).clamp(range.min, range.max)
                    });
                    validate_value(selected_axis, tag, effective_default)?;
                    if effective_default < range.min || effective_default > range.max {
                        return Err(FontminError::config(format!(
                            "variation axis `{tag}` default {effective_default} is outside [{}, {}]",
                            range.min, range.max,
                        )));
                    }

                    Ok(ValidatedAxisSetting::Range {
                        tag: selected_axis.tag,
                        min: range.min,
                        max: range.max,
                        default: range.default.unwrap_or(f32::NAN),
                    })
                }
            }
        })
        .collect()
}

fn find_axis<'a>(axes: &'a [VariationAxis], tag: &str) -> Result<&'a VariationAxis> {
    let tag_bytes = parse_axis_tag(tag)
        .ok_or_else(|| FontminError::config(format!("unknown variation axis `{tag}`")))?;

    axes.iter()
        .find(|axis| axis.tag == tag_bytes)
        .ok_or_else(|| FontminError::config(format!("unknown variation axis `{tag}`")))
}

fn validate_value(axis: &VariationAxis, tag: &str, value: f32) -> Result<()> {
    if !axis_value_in_range(axis, value) {
        return Err(FontminError::config(format!(
            "variation axis `{tag}` value {value} is outside [{}, {}]",
            axis.min, axis.max,
        )));
    }

    Ok(())
}

fn parse_axis_tag(tag: &str) -> Option<[u8; 4]> {
    <[u8; 4]>::try_from(tag.as_bytes())
        .ok()
        .filter(|_| tag.is_ascii())
}

fn axis_value_in_range(axis: &VariationAxis, value: f32) -> bool {
    let precise_value = f64::from(value);
    value.is_finite() && precise_value >= axis.min && precise_value <= axis.max
}

#[allow(clippy::cast_possible_truncation)]
fn axis_default_f32(axis: &VariationAxis) -> f32 {
    axis.default as f32
}

fn run_harfbuzz(
    input: &[u8],
    settings: &[ValidatedAxisSetting],
    downgrade_cff2: bool,
) -> Result<Vec<u8>> {
    let runtime = compiled_runtime()?;
    let mut store = Store::new(&runtime.engine, ());
    let instance = Linker::<()>::new(&runtime.engine)
        .instantiate(&mut store, &runtime.module)
        .and_then(|instance| instance.start(&mut store))
        .map_err(|error| runtime_error("instantiate HarfBuzz", error))?;
    let memory = instance
        .get_memory(&store, "memory")
        .ok_or_else(|| FontminError::convert_failed("HarfBuzz exports no memory"))?;
    let malloc = instance
        .get_typed_func::<i32, i32>(&store, "malloc")
        .map_err(|error| runtime_error("load HarfBuzz malloc", error))?;
    let free = instance
        .get_typed_func::<i32, ()>(&store, "free")
        .map_err(|error| runtime_error("load HarfBuzz free", error))?;
    let input_len = i32::try_from(input.len())
        .map_err(|_| FontminError::convert_failed("font is too large for HarfBuzz WASM"))?;
    let input_ptr = malloc
        .call(&mut store, input_len)
        .map_err(|error| runtime_error("allocate HarfBuzz input", error))?;
    if input_ptr == 0 {
        return Err(FontminError::convert_failed(
            "HarfBuzz could not allocate the font input",
        ));
    }
    let input_offset = usize::try_from(input_ptr)
        .map_err(|_| FontminError::convert_failed("HarfBuzz returned a negative input pointer"))?;
    memory
        .write(&mut store, input_offset, input)
        .map_err(|error| runtime_error("copy HarfBuzz input", error))?;

    let blob_create = instance
        .get_typed_func::<(i32, i32, i32, i32, i32), i32>(&store, "hb_blob_create")
        .map_err(|error| runtime_error("load hb_blob_create", error))?;
    let blob_destroy = instance
        .get_typed_func::<i32, ()>(&store, "hb_blob_destroy")
        .map_err(|error| runtime_error("load hb_blob_destroy", error))?;
    let face_create = instance
        .get_typed_func::<(i32, i32), i32>(&store, "hb_face_create")
        .map_err(|error| runtime_error("load hb_face_create", error))?;
    let face_destroy = instance
        .get_typed_func::<i32, ()>(&store, "hb_face_destroy")
        .map_err(|error| runtime_error("load hb_face_destroy", error))?;
    let source_blob = blob_create
        .call(
            &mut store,
            (input_ptr, input_len, HB_MEMORY_MODE_WRITABLE, 0, 0),
        )
        .map_err(|error| runtime_error("create HarfBuzz blob", error))?;
    let source_face = face_create
        .call(&mut store, (source_blob, 0))
        .map_err(|error| runtime_error("create HarfBuzz face", error))?;
    blob_destroy
        .call(&mut store, source_blob)
        .map_err(|error| runtime_error("release HarfBuzz source blob", error))?;

    let input_create = instance
        .get_typed_func::<(), i32>(&store, "hb_subset_input_create_or_fail")
        .map_err(|error| runtime_error("load hb_subset_input_create_or_fail", error))?;
    let input_destroy = instance
        .get_typed_func::<i32, ()>(&store, "hb_subset_input_destroy")
        .map_err(|error| runtime_error("load hb_subset_input_destroy", error))?;
    let keep_everything = instance
        .get_typed_func::<i32, ()>(&store, "hb_subset_input_keep_everything")
        .map_err(|error| runtime_error("load hb_subset_input_keep_everything", error))?;
    let subset_input = input_create
        .call(&mut store, ())
        .map_err(|error| runtime_error("create HarfBuzz subset input", error))?;
    if subset_input == 0 {
        face_destroy.call(&mut store, source_face).ok();
        free.call(&mut store, input_ptr).ok();
        return Err(FontminError::convert_failed(
            "HarfBuzz could not allocate a subset input",
        ));
    }
    keep_everything
        .call(&mut store, subset_input)
        .map_err(|error| runtime_error("configure HarfBuzz passthrough", error))?;

    if downgrade_cff2 {
        let get_flags = instance
            .get_typed_func::<i32, i32>(&store, "hb_subset_input_get_flags")
            .map_err(|error| runtime_error("load hb_subset_input_get_flags", error))?;
        let set_flags = instance
            .get_typed_func::<(i32, i32), ()>(&store, "hb_subset_input_set_flags")
            .map_err(|error| runtime_error("load hb_subset_input_set_flags", error))?;
        let flags = get_flags
            .call(&mut store, subset_input)
            .map_err(|error| runtime_error("read HarfBuzz subset flags", error))?;
        set_flags
            .call(
                &mut store,
                (subset_input, flags | HB_SUBSET_FLAGS_DOWNGRADE_CFF2),
            )
            .map_err(|error| runtime_error("set HarfBuzz subset flags", error))?;
    }

    let pin_axis = instance
        .get_typed_func::<(i32, i32, i32, f32), i32>(&store, "hb_subset_input_pin_axis_location")
        .map_err(|error| runtime_error("load hb_subset_input_pin_axis_location", error))?;
    let set_axis_range = instance
        .get_typed_func::<(i32, i32, i32, f32, f32, f32), i32>(
            &store,
            "hb_subset_input_set_axis_range",
        )
        .map_err(|error| runtime_error("load hb_subset_input_set_axis_range", error))?;
    for setting in settings {
        let success = match *setting {
            ValidatedAxisSetting::Pin { tag, value } => pin_axis
                .call(
                    &mut store,
                    (subset_input, source_face, tag_to_i32(tag), value),
                )
                .map_err(|error| runtime_error("pin HarfBuzz variation axis", error))?,
            ValidatedAxisSetting::Range {
                tag,
                min,
                max,
                default,
            } => set_axis_range
                .call(
                    &mut store,
                    (
                        subset_input,
                        source_face,
                        tag_to_i32(tag),
                        min,
                        max,
                        default,
                    ),
                )
                .map_err(|error| runtime_error("set HarfBuzz variation range", error))?,
        };
        if success == 0 {
            input_destroy.call(&mut store, subset_input).ok();
            face_destroy.call(&mut store, source_face).ok();
            free.call(&mut store, input_ptr).ok();
            return Err(FontminError::convert_failed(
                "HarfBuzz rejected a validated variation axis setting",
            ));
        }
    }

    let subset = instance
        .get_typed_func::<(i32, i32), i32>(&store, "hb_subset_or_fail")
        .map_err(|error| runtime_error("load hb_subset_or_fail", error))?;
    let output_face = subset
        .call(&mut store, (source_face, subset_input))
        .map_err(|error| runtime_error("reduce HarfBuzz variation space", error))?;
    input_destroy
        .call(&mut store, subset_input)
        .map_err(|error| runtime_error("release HarfBuzz subset input", error))?;
    if output_face == 0 {
        face_destroy.call(&mut store, source_face).ok();
        free.call(&mut store, input_ptr).ok();
        return Err(FontminError::convert_failed(
            "HarfBuzz could not reduce the variable font",
        ));
    }

    let reference_blob = instance
        .get_typed_func::<i32, i32>(&store, "hb_face_reference_blob")
        .map_err(|error| runtime_error("load hb_face_reference_blob", error))?;
    let blob_get_data = instance
        .get_typed_func::<(i32, i32), i32>(&store, "hb_blob_get_data")
        .map_err(|error| runtime_error("load hb_blob_get_data", error))?;
    let blob_get_length = instance
        .get_typed_func::<i32, i32>(&store, "hb_blob_get_length")
        .map_err(|error| runtime_error("load hb_blob_get_length", error))?;
    let output_blob = reference_blob
        .call(&mut store, output_face)
        .map_err(|error| runtime_error("reference HarfBuzz output blob", error))?;
    let output_ptr = blob_get_data
        .call(&mut store, (output_blob, 0))
        .map_err(|error| runtime_error("read HarfBuzz output pointer", error))?;
    let output_len = blob_get_length
        .call(&mut store, output_blob)
        .map_err(|error| runtime_error("read HarfBuzz output length", error))?;
    if output_ptr == 0 || output_len <= 0 {
        blob_destroy.call(&mut store, output_blob).ok();
        face_destroy.call(&mut store, output_face).ok();
        face_destroy.call(&mut store, source_face).ok();
        free.call(&mut store, input_ptr).ok();
        return Err(FontminError::convert_failed(
            "HarfBuzz produced an empty variable font",
        ));
    }
    let output_size = usize::try_from(output_len)
        .map_err(|_| FontminError::convert_failed("HarfBuzz returned a negative output length"))?;
    let output_offset = usize::try_from(output_ptr)
        .map_err(|_| FontminError::convert_failed("HarfBuzz returned a negative output pointer"))?;
    let mut output = vec![0; output_size];
    memory
        .read(&store, output_offset, &mut output)
        .map_err(|error| runtime_error("copy HarfBuzz output", error))?;

    blob_destroy
        .call(&mut store, output_blob)
        .map_err(|error| runtime_error("release HarfBuzz output blob", error))?;
    face_destroy
        .call(&mut store, output_face)
        .map_err(|error| runtime_error("release HarfBuzz output face", error))?;
    face_destroy
        .call(&mut store, source_face)
        .map_err(|error| runtime_error("release HarfBuzz source face", error))?;
    free.call(&mut store, input_ptr)
        .map_err(|error| runtime_error("release HarfBuzz input bytes", error))?;

    Ok(output)
}

fn compiled_runtime() -> Result<&'static CompiledRuntime> {
    COMPILED_RUNTIME
        .get_or_init(|| {
            let engine = Engine::default();
            let module =
                Module::new(&engine, HARFBUZZ_SUBSET_WASM).map_err(|error| error.to_string())?;

            Ok(CompiledRuntime { engine, module })
        })
        .as_ref()
        .map_err(|message| {
            FontminError::convert_failed(format!("could not compile HarfBuzz WASM: {message}"))
        })
}

fn runtime_error(stage: &str, error: impl std::fmt::Display) -> FontminError {
    FontminError::convert_failed(format!("{stage}: {error}"))
}

fn tag_to_i32(tag: [u8; 4]) -> i32 {
    i32::from_be_bytes(tag)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use fontmin_testing::{ESTEDAD_VARIABLE, ROBOTO, SOURCE_SERIF_4_VARIABLE_CFF2};

    use super::{AxisRange, AxisSetting, VariationSpaceOptions, reduce_variation_space};

    fn table<'a>(font: &'a [u8], tag: &[u8]) -> Option<&'a [u8]> {
        fontmin_ttf::read_sfnt_table_directory(font)
            .ok()?
            .into_iter()
            .find(|table| table.tag.as_bytes() == tag)
            .and_then(|table| {
                let start = table.offset;
                let len = table.length;
                font.get(start..start.checked_add(len)?)
            })
    }

    fn fvar_tags(font: &[u8]) -> Vec<[u8; 4]> {
        let fvar = table(font, b"fvar").expect("fvar table");
        let axes_offset = usize::from(u16::from_be_bytes([fvar[4], fvar[5]]));
        let axis_count = usize::from(u16::from_be_bytes([fvar[8], fvar[9]]));
        let axis_size = usize::from(u16::from_be_bytes([fvar[10], fvar[11]]));

        (0..axis_count)
            .map(|index| {
                let offset = axes_offset + index * axis_size;
                fvar[offset..offset + 4].try_into().unwrap()
            })
            .collect()
    }

    fn fvar_axis(font: &[u8], selected_tag: &[u8]) -> (f64, f64, f64) {
        let fvar = table(font, b"fvar").expect("fvar table");
        let axes_offset = usize::from(u16::from_be_bytes([fvar[4], fvar[5]]));
        let axis_count = usize::from(u16::from_be_bytes([fvar[8], fvar[9]]));
        let axis_size = usize::from(u16::from_be_bytes([fvar[10], fvar[11]]));
        let fixed = |start| {
            f64::from(i32::from_be_bytes(
                fvar[start..start + 4].try_into().unwrap(),
            )) / 65_536.0
        };
        let offset = (0..axis_count)
            .map(|index| axes_offset + index * axis_size)
            .find(|&offset| &fvar[offset..offset + 4] == selected_tag)
            .expect("selected fvar axis");

        (fixed(offset + 4), fixed(offset + 8), fixed(offset + 12))
    }

    #[test]
    fn pins_one_cff2_axis_and_keeps_the_other_variable() {
        let options = VariationSpaceOptions {
            axes: BTreeMap::from([("wght".into(), AxisSetting::Pin(700.0))]),
            downgrade_cff2: false,
        };

        let output = reduce_variation_space(SOURCE_SERIF_4_VARIABLE_CFF2, &options).unwrap();

        assert!(output.starts_with(b"OTTO"));
        assert_eq!(fvar_tags(&output), vec![*b"opsz"]);
        assert!(table(&output, b"CFF2").is_some());
    }

    #[test]
    fn narrows_a_cff2_axis_range_and_updates_fvar() {
        let options = VariationSpaceOptions {
            axes: BTreeMap::from([(
                "wght".into(),
                AxisSetting::Range(AxisRange {
                    min: 400.0,
                    max: 700.0,
                    default: Some(500.0),
                }),
            )]),
            downgrade_cff2: false,
        };

        let output = reduce_variation_space(SOURCE_SERIF_4_VARIABLE_CFF2, &options).unwrap();
        assert_eq!(fvar_axis(&output, b"wght"), (400.0, 500.0, 700.0));
        assert_eq!(fvar_tags(&output), vec![*b"wght", *b"opsz"]);
    }

    #[test]
    fn narrows_a_glyf_axis_and_clamps_the_original_default() {
        let options = VariationSpaceOptions {
            axes: BTreeMap::from([(
                "wght".into(),
                AxisSetting::Range(AxisRange {
                    min: 500.0,
                    max: 700.0,
                    default: None,
                }),
            )]),
            downgrade_cff2: false,
        };

        let output = reduce_variation_space(ESTEDAD_VARIABLE, &options).unwrap();

        assert_eq!(fvar_axis(&output, b"wght"), (500.0, 700.0, 700.0));
        assert_eq!(fvar_tags(&output), vec![*b"wght", *b"wdth"]);
        assert_ne!(table(&output, b"gvar"), table(ESTEDAD_VARIABLE, b"gvar"));
    }

    #[test]
    fn fully_pins_and_downgrades_cff2() {
        let options = VariationSpaceOptions {
            axes: BTreeMap::from([
                ("opsz".into(), AxisSetting::Pin(20.0)),
                ("wght".into(), AxisSetting::Pin(700.0)),
            ]),
            downgrade_cff2: true,
        };

        let output = reduce_variation_space(SOURCE_SERIF_4_VARIABLE_CFF2, &options).unwrap();

        assert!(table(&output, b"fvar").is_none());
        assert!(table(&output, b"CFF2").is_none());
        assert!(table(&output, b"CFF ").is_some());
    }

    #[test]
    fn pins_one_glyf_axis_and_keeps_the_other_variable() {
        let options = VariationSpaceOptions {
            axes: BTreeMap::from([("wght".into(), AxisSetting::Pin(900.0))]),
            downgrade_cff2: false,
        };

        let output = reduce_variation_space(ESTEDAD_VARIABLE, &options).unwrap();

        assert!(output.starts_with(&[0, 1, 0, 0]));
        assert_eq!(fvar_tags(&output), vec![*b"wdth"]);
        assert_ne!(table(&output, b"gvar"), table(ESTEDAD_VARIABLE, b"gvar"));
    }

    #[test]
    fn rejects_static_and_invalid_axis_requests() {
        let empty = VariationSpaceOptions::default();
        assert!(reduce_variation_space(ESTEDAD_VARIABLE, &empty).is_err());

        let pin = |tag: &str, value| VariationSpaceOptions {
            axes: BTreeMap::from([(tag.into(), AxisSetting::Pin(value))]),
            downgrade_cff2: false,
        };
        assert!(reduce_variation_space(ROBOTO, &pin("wght", 400.0)).is_err());
        assert!(reduce_variation_space(ESTEDAD_VARIABLE, &pin("xxxx", 400.0)).is_err());
        assert!(reduce_variation_space(ESTEDAD_VARIABLE, &pin("wght", 1_000.0)).is_err());
    }
}
