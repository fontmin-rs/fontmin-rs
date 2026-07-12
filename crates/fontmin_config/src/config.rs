use fontmin_core::{FontDeliverySlice, OutputFormat};
use fontmin_css::UnicodeRange;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FontminConfig {
    pub cwd: Option<String>,
    pub input: Vec<String>,
    pub out_dir: Option<String>,
    pub clean: bool,
    pub preserve_original: bool,
    pub parallel: ParallelConfig,
    pub cache: CacheConfig,
    pub otf: OtfConfig,
    pub subset: Option<SubsetConfig>,
    pub delivery: Option<DeliveryConfig>,
    pub outputs: Vec<OutputConfig>,
    pub css: Option<CssConfig>,
    pub plugins: Vec<PluginConfig>,
    pub diagnostics: DiagnosticsConfig,
}

impl Default for FontminConfig {
    fn default() -> Self {
        Self {
            cwd: None,
            input: Vec::new(),
            out_dir: Some("build".into()),
            clean: false,
            preserve_original: true,
            parallel: ParallelConfig::default(),
            cache: CacheConfig::default(),
            otf: OtfConfig::default(),
            subset: None,
            delivery: None,
            outputs: vec![
                OutputConfig::format(OutputFormat::Eot),
                OutputConfig::format(OutputFormat::Woff),
                OutputConfig::format(OutputFormat::Woff2),
                OutputConfig::format(OutputFormat::Svg),
                OutputConfig::format(OutputFormat::Css),
            ],
            css: Some(CssConfig::default()),
            plugins: Vec::new(),
            diagnostics: DiagnosticsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OtfConfig {
    pub preserve_hinting: bool,
    pub variation_coordinates: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DeliveryConfig {
    pub slices: Vec<FontDeliverySlice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ParallelConfig {
    pub threads: ThreadCount,
    pub per_file: bool,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            threads: ThreadCount::Auto,
            per_file: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThreadCount {
    #[default]
    Auto,
    Count(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CacheConfig {
    pub enabled: bool,
    pub dir: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            dir: "node_modules/.cache/fontmin-rs".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SubsetConfig {
    pub text: Option<String>,
    pub text_file: Option<String>,
    pub unicodes: Vec<u32>,
    pub basic_text: bool,
    pub preserve_hinting: bool,
    pub trim: bool,
    pub keep_notdef: bool,
    pub keep_layout: LayoutSubsetMode,
}

impl Default for SubsetConfig {
    fn default() -> Self {
        Self {
            text: None,
            text_file: None,
            unicodes: Vec::new(),
            basic_text: false,
            preserve_hinting: false,
            trim: true,
            keep_notdef: true,
            keep_layout: LayoutSubsetMode::Conservative,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutSubsetMode {
    Drop,
    Conservative,
    Preserve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputConfig {
    pub format: OutputFormat,
    #[serde(default = "default_true")]
    pub clone: bool,
    pub file_name: Option<String>,
    pub ext: Option<String>,
}

impl OutputConfig {
    pub fn format(format: OutputFormat) -> Self {
        Self {
            format,
            clone: true,
            file_name: None,
            ext: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CssConfig {
    pub font_path: String,
    pub base64: bool,
    pub glyph: bool,
    pub icon_prefix: String,
    pub font_family: Option<String>,
    pub as_file_name: Option<bool>,
    pub local: bool,
    pub font_display: String,
    pub target: CssTarget,
    pub unicode_ranges: Vec<UnicodeRange>,
}

impl Default for CssConfig {
    fn default() -> Self {
        Self {
            font_path: "./".into(),
            base64: false,
            glyph: false,
            icon_prefix: "icon".into(),
            font_family: None,
            as_file_name: None,
            local: true,
            font_display: "swap".into(),
            target: CssTarget::Css,
            unicode_ranges: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CssTarget {
    #[default]
    Css,
    Scss,
    Less,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfig {
    pub name: String,
    pub enforce: Option<PluginEnforce>,
    pub native: BuiltinPluginConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginEnforce {
    Pre,
    Post,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinPluginConfig {
    pub kind: BuiltinPluginKind,
    pub name: String,
    #[serde(default)]
    pub options: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuiltinPluginKind {
    Builtin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DiagnosticsConfig {
    pub level: DiagnosticLevel,
    pub pretty: bool,
    pub fail_on_warning: bool,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self {
            level: DiagnosticLevel::Warn,
            pretty: true,
            fail_on_warning: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticLevel {
    Error,
    Warn,
    Info,
    Silent,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use fontmin_core::OutputFormat;

    use super::{FontminConfig, OutputConfig, PluginEnforce};

    #[test]
    fn deserializes_node_builtin_plugin_descriptors() {
        let config: FontminConfig = serde_json::from_str(
            r#"{
              "plugins": [
                {
                  "name": "fontmin:glyph",
                  "enforce": "pre",
                  "native": {
                    "kind": "builtin",
                    "name": "glyph",
                    "options": { "text": "Hello", "clone": false }
                  }
                }
              ]
            }"#,
        )
        .unwrap();

        assert_eq!(config.plugins[0].name, "fontmin:glyph");
        assert_eq!(config.plugins[0].enforce, Some(PluginEnforce::Pre));
        assert_eq!(config.plugins[0].native.name, "glyph");
        assert_eq!(config.plugins[0].native.options["text"], "Hello");
    }

    #[test]
    fn rejects_non_builtin_plugin_descriptors() {
        let error = serde_json::from_str::<FontminConfig>(
            r#"{"plugins":[{"name":"custom","native":{"kind":"custom","name":"custom","options":{}}}]}"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("unknown variant `custom`"));
    }

    #[test]
    fn default_config_matches_fontmin_compat_outputs() {
        let config = FontminConfig::default();
        let formats: Vec<_> = config.outputs.iter().map(|output| output.format).collect();

        assert_eq!(
            formats,
            vec![
                OutputFormat::Eot,
                OutputFormat::Woff,
                OutputFormat::Woff2,
                OutputFormat::Svg,
                OutputFormat::Css,
            ],
        );
        assert_eq!(config.out_dir.as_deref(), Some("build"));
        assert!(config.preserve_original);
    }

    #[test]
    fn output_config_uses_clone_by_default() {
        let output = OutputConfig::format(OutputFormat::Woff2);

        assert_eq!(output.format, OutputFormat::Woff2);
        assert!(output.clone);
        assert!(output.file_name.is_none());
        assert!(output.ext.is_none());
    }

    #[test]
    fn deserializes_partial_user_config_with_defaults() {
        let config: FontminConfig = serde_json::from_str(
            r#"{
              "input": ["font.ttf"],
              "subset": { "text": "Hello" },
              "outputs": [{ "format": "woff2" }],
              "css": {
                "fontFamily": "Roboto",
                "unicodeRanges": ["U+0020-007E"]
              }
            }"#,
        )
        .unwrap();

        assert_eq!(config.out_dir.as_deref(), Some("build"));
        assert_eq!(config.subset.unwrap().text.as_deref(), Some("Hello"));
        assert_eq!(config.outputs[0].format, OutputFormat::Woff2);
        assert!(config.outputs[0].clone);
        let css = config.css.unwrap();
        assert_eq!(css.font_family.as_deref(), Some("Roboto"));
        assert_eq!(css.unicode_ranges[0].to_string(), "U+0020-007E");
    }

    #[test]
    fn deserializes_otf_variation_coordinates() {
        let config: FontminConfig = serde_json::from_str(
            r#"{
              "otf": {
                "variationCoordinates": { "wght": 700, "opsz": 14 }
              }
            }"#,
        )
        .unwrap();

        assert_eq!(config.otf.variation_coordinates.get("wght"), Some(&700.0));
        assert_eq!(config.otf.variation_coordinates.get("opsz"), Some(&14.0));
    }
}
