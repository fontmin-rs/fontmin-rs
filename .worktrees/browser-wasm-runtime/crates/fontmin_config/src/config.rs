use fontmin_core::OutputFormat;
use serde::{Deserialize, Serialize};

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
    pub subset: Option<SubsetConfig>,
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
            subset: None,
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
    pub options: serde_json::Value,
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

    use super::{FontminConfig, OutputConfig};

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
              "css": { "fontFamily": "Roboto" }
            }"#,
        )
        .unwrap();

        assert_eq!(config.out_dir.as_deref(), Some("build"));
        assert_eq!(config.subset.unwrap().text.as_deref(), Some("Hello"));
        assert_eq!(config.outputs[0].format, OutputFormat::Woff2);
        assert!(config.outputs[0].clone);
        assert_eq!(config.css.unwrap().font_family.as_deref(), Some("Roboto"));
    }
}
