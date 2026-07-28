use std::collections::BTreeMap;

use fontmin_core::{FontDeliverySlice, MissingGlyphPolicy, UnicodeRange};
use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeStruct as _};

use crate::config::{CssTarget, LayoutSubsetMode};

#[derive(Debug, Clone, Serialize)]
pub struct BuiltinPluginConfig {
    pub kind: BuiltinPluginKind,
    #[serde(flatten)]
    pub plugin: BuiltinPlugin,
}

impl BuiltinPluginConfig {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.plugin.name()
    }

    #[must_use]
    pub const fn public_name(&self) -> &'static str {
        self.plugin.public_name()
    }
}

impl<'de> Deserialize<'de> for BuiltinPluginConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = BuiltinPluginWire::deserialize(deserializer)?;
        let plugin = BuiltinPlugin::from_wire(&wire.name, wire.options)?;

        Ok(Self {
            kind: wire.kind,
            plugin,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuiltinPluginKind {
    Builtin,
}

#[derive(Debug, Clone)]
pub enum BuiltinPlugin {
    Glyph(GlyphPluginConfig),
    UnicodeSlices(UnicodeSlicesPluginConfig),
    Otf2Ttf(Otf2TtfPluginConfig),
    Ttf2Woff(Ttf2WoffPluginConfig),
    Ttf2Woff2(Ttf2Woff2PluginConfig),
    Ttf2Eot(Ttf2EotPluginConfig),
    Ttf2Svg(Ttf2SvgPluginConfig),
    Svg2Ttf(Svg2TtfPluginConfig),
    Svgs2Ttf(Svgs2TtfPluginConfig),
    Css(CssPluginConfig),
}

impl BuiltinPlugin {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Glyph(_) => "glyph",
            Self::UnicodeSlices(_) => "unicodeSlices",
            Self::Otf2Ttf(_) => "otf2ttf",
            Self::Ttf2Woff(_) => "ttf2woff",
            Self::Ttf2Woff2(_) => "ttf2woff2",
            Self::Ttf2Eot(_) => "ttf2eot",
            Self::Ttf2Svg(_) => "ttf2svg",
            Self::Svg2Ttf(_) => "svg2ttf",
            Self::Svgs2Ttf(_) => "svgs2ttf",
            Self::Css(_) => "css",
        }
    }

    #[must_use]
    pub const fn public_name(&self) -> &'static str {
        match self {
            Self::Glyph(_) => "fontmin:glyph",
            Self::UnicodeSlices(_) => "fontmin:unicode-slices",
            Self::Otf2Ttf(_) => "fontmin:otf2ttf",
            Self::Ttf2Woff(_) => "fontmin:ttf2woff",
            Self::Ttf2Woff2(_) => "fontmin:ttf2woff2",
            Self::Ttf2Eot(_) => "fontmin:ttf2eot",
            Self::Ttf2Svg(_) => "fontmin:ttf2svg",
            Self::Svg2Ttf(_) => "fontmin:svg2ttf",
            Self::Svgs2Ttf(_) => "fontmin:svgs2ttf",
            Self::Css(_) => "fontmin:css",
        }
    }

    fn from_wire<E>(name: &str, options: Option<serde_json::Value>) -> Result<Self, E>
    where
        E: serde::de::Error,
    {
        match name {
            "glyph" => decode_options(name, options).map(Self::Glyph),
            "unicodeSlices" => decode_options(name, options).map(Self::UnicodeSlices),
            "otf2ttf" => decode_options(name, options).map(Self::Otf2Ttf),
            "ttf2woff" => decode_options(name, options).map(Self::Ttf2Woff),
            "ttf2woff2" => decode_options(name, options).map(Self::Ttf2Woff2),
            "ttf2eot" => decode_options(name, options).map(Self::Ttf2Eot),
            "ttf2svg" => decode_options(name, options).map(Self::Ttf2Svg),
            "svg2ttf" => decode_options(name, options).map(Self::Svg2Ttf),
            "svgs2ttf" => decode_options(name, options).map(Self::Svgs2Ttf),
            "css" => decode_options(name, options).map(Self::Css),
            _ => Err(E::custom(format!("unsupported built-in plugin `{name}`"))),
        }
    }
}

impl Serialize for BuiltinPlugin {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("BuiltinPlugin", 2)?;

        state.serialize_field("name", self.name())?;
        match self {
            Self::Glyph(options) => state.serialize_field("options", options)?,
            Self::UnicodeSlices(options) => state.serialize_field("options", options)?,
            Self::Otf2Ttf(options) => state.serialize_field("options", options)?,
            Self::Ttf2Woff(options) => state.serialize_field("options", options)?,
            Self::Ttf2Woff2(options) => state.serialize_field("options", options)?,
            Self::Ttf2Eot(options) => state.serialize_field("options", options)?,
            Self::Ttf2Svg(options) => state.serialize_field("options", options)?,
            Self::Svg2Ttf(options) => state.serialize_field("options", options)?,
            Self::Svgs2Ttf(options) => state.serialize_field("options", options)?,
            Self::Css(options) => state.serialize_field("options", options)?,
        }
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BuiltinPluginWire {
    kind: BuiltinPluginKind,
    name: String,
    #[serde(default)]
    options: Option<serde_json::Value>,
}

fn decode_options<T, E>(name: &str, options: Option<serde_json::Value>) -> Result<T, E>
where
    T: for<'de> Deserialize<'de>,
    E: serde::de::Error,
{
    serde_json::from_value(options.unwrap_or_else(|| serde_json::json!({}))).map_err(|error| {
        E::custom(format!(
            "invalid options for built-in plugin `{name}`: {error}",
        ))
    })
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct GlyphPluginConfig {
    pub text: Option<String>,
    pub text_file: Option<String>,
    pub unicodes: Vec<u32>,
    pub unicode_ranges: Vec<UnicodeRange>,
    pub basic_text: Option<bool>,
    pub hinting: Option<bool>,
    pub trim: Option<bool>,
    pub keep_notdef: Option<bool>,
    pub keep_layout: Option<LayoutSubsetMode>,
    pub clone: Option<bool>,
    pub preserve_hinting: Option<bool>,
    pub missing_glyphs: Option<MissingGlyphPolicy>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UnicodeSlicesPluginConfig {
    pub slices: Vec<FontDeliverySlice>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Otf2TtfPluginConfig {
    pub clone: Option<bool>,
    pub preserve_hinting: Option<bool>,
    pub variation_coordinates: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Ttf2WoffPluginConfig {
    pub clone: Option<bool>,
    pub deflate: Option<bool>,
    pub compression_level: Option<u32>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Ttf2Woff2PluginConfig {
    pub clone: Option<bool>,
    pub quality: Option<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Ttf2EotPluginConfig {
    pub clone: Option<bool>,
    pub version: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Ttf2SvgPluginConfig {
    pub clone: Option<bool>,
    pub font_family: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Svg2TtfPluginConfig {
    pub clone: Option<bool>,
    pub hinting: Option<bool>,
    pub normalize: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Svgs2TtfPluginConfig {
    pub clone: Option<bool>,
    pub font_name: Option<String>,
    pub start_unicode: Option<u32>,
    pub ascent: Option<i16>,
    pub descent: Option<i16>,
    pub normalize: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct CssPluginConfig {
    pub font_path: Option<String>,
    pub base64: Option<bool>,
    pub glyph: Option<bool>,
    pub icon_prefix: Option<String>,
    pub font_family: Option<String>,
    pub as_file_name: Option<bool>,
    pub local: Option<bool>,
    pub font_display: Option<String>,
    pub target: Option<CssTarget>,
    pub unicode_ranges: Vec<UnicodeRange>,
}
