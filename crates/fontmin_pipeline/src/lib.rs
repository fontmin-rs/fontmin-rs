use std::{collections::BTreeMap, path::Path};

use fontmin_config::{
    CssConfig, CssTarget as ConfigCssTarget, DeliveryConfig, FontminConfig,
    LayoutSubsetMode as ConfigLayoutSubsetMode, OutputConfig, PluginConfig, PluginEnforce,
    SubsetConfig,
};
use fontmin_core::{Asset, FontDeliverySlice, FontFormat, OutputFormat, UnicodeRange};
use fontmin_css::{CssOptions, CssTarget};
use fontmin_diagnostics::{FontminError, Result};
use fontmin_otf::Otf2TtfOptions;
use fontmin_plugin::{FontminPlugin, PluginContext, PluginKind, PluginOrder, async_trait};
use fontmin_plugins::{
    CssPlugin, GlyphPlugin, Otf2TtfPlugin, SlicePlugin, Svg2TtfPlugin, Svgs2TtfPlugin,
    Ttf2EotPlugin, Ttf2SvgPlugin, Ttf2Woff2Plugin, Ttf2WoffPlugin,
};
use fontmin_subset::{LayoutSubsetMode, SubsetOptions};
use serde::{Deserialize, de::DeserializeOwned};

pub struct Engine {
    assets: Vec<Asset>,
    plugins: Vec<Box<dyn FontminPlugin>>,
}

impl Engine {
    #[must_use]
    pub fn new(config: FontminConfig) -> Self {
        Self::try_new(config).expect("invalid fontmin configuration")
    }

    pub fn try_new(config: FontminConfig) -> Result<Self> {
        let mut engine = Self {
            assets: Vec::new(),
            plugins: Vec::new(),
        };

        engine.configure_explicit_plugins(&config)?;
        engine.configure_builtin_plugins(config);

        Ok(engine)
    }

    #[must_use]
    pub fn from_assets(assets: Vec<Asset>) -> Self {
        Self {
            assets,
            plugins: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_assets(mut self, assets: Vec<Asset>) -> Self {
        self.assets = assets;

        self
    }

    #[must_use]
    pub fn plugin(mut self, plugin: impl FontminPlugin + 'static) -> Self {
        self.plugins.push(Box::new(plugin));

        self
    }

    pub async fn run(mut self) -> Result<Vec<Asset>> {
        let mut ctx = PluginContext::new();
        let mut assets = std::mem::take(&mut self.assets);

        self.sort_plugins();
        for plugin in &self.plugins {
            plugin.build_start(&mut ctx).await?;
        }

        for plugin in &self.plugins {
            let mut next_assets = Vec::new();

            for asset in assets {
                next_assets.extend(plugin.transform(&mut ctx, asset).await?);
            }

            assets = next_assets;
        }

        for plugin in &self.plugins {
            plugin.generate_bundle(&mut ctx, &mut assets).await?;
        }

        for plugin in &self.plugins {
            plugin.build_end(&mut ctx).await?;
        }

        Ok(assets)
    }

    fn sort_plugins(&mut self) {
        self.plugins.sort_by_key(|plugin| plugin.order());
    }

    fn configure_explicit_plugins(&mut self, config: &FontminConfig) -> Result<()> {
        for plugin in &config.plugins {
            let inner = plugin_from_config(plugin, config.cwd.as_deref())?;
            let order = match plugin.enforce {
                Some(PluginEnforce::Pre) => PluginOrder::Pre,
                Some(PluginEnforce::Post) => PluginOrder::Post,
                None => inner.order(),
            };

            self.plugins.push(Box::new(OrderedPlugin { inner, order }));
        }

        Ok(())
    }

    fn configure_builtin_plugins(&mut self, config: FontminConfig) {
        let FontminConfig {
            subset,
            delivery,
            outputs,
            css,
            otf,
            ..
        } = config;

        self.plugins.push(Box::new(Otf2TtfPlugin {
            options: Otf2TtfOptions {
                preserve_hinting: otf.preserve_hinting,
                variation_coordinates: otf.variation_coordinates,
            },
            clone: false,
        }));

        if let Some(subset) = subset {
            self.plugins.push(Box::new(GlyphPlugin {
                options: subset_options_from_config(subset),
                clone: false,
            }));
        }

        if let Some(DeliveryConfig { slices }) = delivery {
            self.plugins.push(Box::new(SlicePlugin { slices }));
        }

        let mut requested_outputs = Vec::with_capacity(outputs.len());
        let mut output_path_rules = Vec::new();
        let mut css_requested = false;

        for output in outputs {
            requested_outputs.push(output.format);
            if output.file_name.is_some() || output.ext.is_some() {
                output_path_rules.push(OutputPathRule::from_config(&output));
            }

            match output.format {
                OutputFormat::Ttf => {}
                OutputFormat::Eot => self.plugins.push(Box::new(Ttf2EotPlugin {
                    clone: output.clone,
                    ..Ttf2EotPlugin::default()
                })),
                OutputFormat::Woff => self.plugins.push(Box::new(Ttf2WoffPlugin {
                    clone: output.clone,
                    ..Ttf2WoffPlugin::default()
                })),
                OutputFormat::Woff2 => self.plugins.push(Box::new(Ttf2Woff2Plugin {
                    clone: output.clone,
                    ..Ttf2Woff2Plugin::default()
                })),
                OutputFormat::Svg => self.plugins.push(Box::new(Ttf2SvgPlugin {
                    clone: output.clone,
                    ..Ttf2SvgPlugin::default()
                })),
                OutputFormat::Css => css_requested = true,
            }
        }

        let font_outputs = requested_outputs
            .iter()
            .copied()
            .filter(|format| *format != OutputFormat::Css)
            .collect::<Vec<_>>();

        if !font_outputs.is_empty() {
            self.plugins.push(Box::new(OutputFilterPlugin {
                formats: font_outputs,
                order: PluginOrder::Normal,
            }));
        }

        let font_output_path_rules = output_path_rules
            .iter()
            .filter(|rule| rule.format != OutputFormat::Css)
            .cloned()
            .collect::<Vec<_>>();

        if !font_output_path_rules.is_empty() {
            self.plugins.push(Box::new(OutputPathPlugin {
                rules: font_output_path_rules,
                order: PluginOrder::Normal,
            }));
        }

        if css_requested {
            self.plugins.push(Box::new(CssPlugin {
                options: css_options_from_config(css),
            }));

            let css_output_path_rules = output_path_rules
                .into_iter()
                .filter(|rule| rule.format == OutputFormat::Css)
                .collect::<Vec<_>>();

            if !css_output_path_rules.is_empty() {
                self.plugins.push(Box::new(OutputPathPlugin {
                    rules: css_output_path_rules,
                    order: PluginOrder::Post,
                }));
            }

            self.plugins.push(Box::new(OutputFilterPlugin {
                formats: requested_outputs,
                order: PluginOrder::Post,
            }));
        }
    }
}

struct OrderedPlugin {
    inner: Box<dyn FontminPlugin>,
    order: PluginOrder,
}

#[async_trait]
impl FontminPlugin for OrderedPlugin {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn order(&self) -> PluginOrder {
        self.order
    }

    fn kind(&self) -> PluginKind {
        self.inner.kind()
    }

    async fn build_start(&self, ctx: &mut PluginContext) -> Result<()> {
        self.inner.build_start(ctx).await
    }

    async fn transform(&self, ctx: &mut PluginContext, asset: Asset) -> Result<Vec<Asset>> {
        self.inner.transform(ctx, asset).await
    }

    async fn generate_bundle(
        &self,
        ctx: &mut PluginContext,
        assets: &mut Vec<Asset>,
    ) -> Result<()> {
        self.inner.generate_bundle(ctx, assets).await
    }

    async fn build_end(&self, ctx: &mut PluginContext) -> Result<()> {
        self.inner.build_end(ctx).await
    }
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct GlyphPluginOptions {
    text: Option<String>,
    text_file: Option<String>,
    unicodes: Vec<u32>,
    unicode_ranges: Vec<UnicodeRange>,
    basic_text: Option<bool>,
    hinting: Option<bool>,
    trim: Option<bool>,
    keep_notdef: Option<bool>,
    keep_layout: Option<ConfigLayoutSubsetMode>,
    clone: Option<bool>,
    preserve_hinting: Option<bool>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SlicePluginOptions {
    slices: Vec<FontDeliverySlice>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct OtfPluginOptions {
    clone: Option<bool>,
    preserve_hinting: Option<bool>,
    variation_coordinates: BTreeMap<String, f32>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct WoffPluginOptions {
    clone: Option<bool>,
    deflate: Option<bool>,
    compression_level: Option<u32>,
    metadata: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Woff2PluginOptions {
    clone: Option<bool>,
    quality: Option<u8>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct EotPluginOptions {
    clone: Option<bool>,
    version: Option<u32>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct TtfSvgPluginOptions {
    clone: Option<bool>,
    font_family: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SvgTtfPluginOptions {
    clone: Option<bool>,
    hinting: Option<bool>,
    normalize: Option<bool>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct SvgCollectionPluginOptions {
    clone: Option<bool>,
    font_name: Option<String>,
    start_unicode: Option<u32>,
    ascent: Option<i16>,
    descent: Option<i16>,
    normalize: Option<bool>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct CssPluginOptions {
    font_path: Option<String>,
    base64: Option<bool>,
    glyph: Option<bool>,
    icon_prefix: Option<String>,
    font_family: Option<String>,
    as_file_name: Option<bool>,
    local: Option<bool>,
    font_display: Option<String>,
    target: Option<ConfigCssTarget>,
    unicode_ranges: Vec<UnicodeRange>,
}

fn plugin_from_config(config: &PluginConfig, cwd: Option<&str>) -> Result<Box<dyn FontminPlugin>> {
    let operation = config.native.name.as_str();
    let expected_name = match operation {
        "unicodeSlices" => "fontmin:unicode-slices".to_owned(),
        "glyph" | "otf2ttf" | "ttf2woff" | "ttf2woff2" | "ttf2eot" | "ttf2svg" | "svg2ttf"
        | "svgs2ttf" | "css" => format!("fontmin:{operation}"),
        _ => {
            return Err(FontminError::config(format!(
                "unsupported built-in plugin `{operation}`"
            )));
        }
    };

    if config.name != expected_name {
        return Err(FontminError::config(format!(
            "built-in plugin `{operation}` must use name `{expected_name}`"
        )));
    }

    match operation {
        "glyph" => glyph_plugin(config, cwd),
        "unicodeSlices" => {
            let options = plugin_options::<SlicePluginOptions>(config)?;
            Ok(Box::new(SlicePlugin {
                slices: options.slices,
            }))
        }
        "otf2ttf" => {
            let options = plugin_options::<OtfPluginOptions>(config)?;
            Ok(Box::new(Otf2TtfPlugin {
                options: Otf2TtfOptions {
                    preserve_hinting: options.preserve_hinting.unwrap_or(false),
                    variation_coordinates: options.variation_coordinates,
                },
                clone: options.clone.unwrap_or(true),
            }))
        }
        "ttf2woff" => {
            let options = plugin_options::<WoffPluginOptions>(config)?;
            let mut plugin = Ttf2WoffPlugin {
                clone: options.clone.unwrap_or(true),
                ..Ttf2WoffPlugin::default()
            };
            plugin.options.deflate = options.deflate.unwrap_or(plugin.options.deflate);
            plugin.options.compression_level = options.compression_level;
            plugin.options.metadata = options.metadata;
            Ok(Box::new(plugin))
        }
        "ttf2woff2" => {
            let options = plugin_options::<Woff2PluginOptions>(config)?;
            let mut plugin = Ttf2Woff2Plugin {
                clone: options.clone.unwrap_or(true),
                ..Ttf2Woff2Plugin::default()
            };
            plugin.options.quality = options.quality;
            Ok(Box::new(plugin))
        }
        "ttf2eot" => {
            let options = plugin_options::<EotPluginOptions>(config)?;
            let mut plugin = Ttf2EotPlugin {
                clone: options.clone.unwrap_or(true),
                ..Ttf2EotPlugin::default()
            };
            plugin.options.version = options.version;
            Ok(Box::new(plugin))
        }
        "ttf2svg" => {
            let options = plugin_options::<TtfSvgPluginOptions>(config)?;
            let mut plugin = Ttf2SvgPlugin {
                clone: options.clone.unwrap_or(true),
                ..Ttf2SvgPlugin::default()
            };
            plugin.options.font_family = options.font_family;
            Ok(Box::new(plugin))
        }
        "svg2ttf" => {
            let options = plugin_options::<SvgTtfPluginOptions>(config)?;
            let mut plugin = Svg2TtfPlugin {
                clone: options.clone.unwrap_or(true),
                ..Svg2TtfPlugin::default()
            };
            plugin.options.hinting = options.hinting.unwrap_or(plugin.options.hinting);
            plugin.options.normalize = options.normalize.unwrap_or(plugin.options.normalize);
            Ok(Box::new(plugin))
        }
        "svgs2ttf" => {
            let options = plugin_options::<SvgCollectionPluginOptions>(config)?;
            let mut plugin = Svgs2TtfPlugin {
                clone: options.clone.unwrap_or(false),
                ..Svgs2TtfPlugin::default()
            };
            if let Some(font_name) = options.font_name {
                plugin.options.font_name = font_name;
            }
            if let Some(start_unicode) = options.start_unicode {
                plugin.options.start_unicode = start_unicode;
            }
            if let Some(ascent) = options.ascent {
                plugin.options.ascent = ascent;
            }
            if let Some(descent) = options.descent {
                plugin.options.descent = descent;
            }
            plugin.options.normalize = options.normalize.unwrap_or(plugin.options.normalize);
            Ok(Box::new(plugin))
        }
        "css" => css_plugin(config),
        _ => unreachable!("supported operations are matched above"),
    }
}

fn glyph_plugin(config: &PluginConfig, cwd: Option<&str>) -> Result<Box<dyn FontminPlugin>> {
    let mut options = plugin_options::<GlyphPluginOptions>(config)?;

    if let Some(text_file) = options.text_file.take() {
        let path = Path::new(cwd.unwrap_or(".")).join(text_file);
        let file_text = std::fs::read_to_string(&path).map_err(|error| {
            FontminError::config(format!(
                "failed to read glyph text file {}: {error}",
                path.display()
            ))
        })?;
        options.text = Some(
            options
                .text
                .map_or(file_text.clone(), |text| text + &file_text),
        );
    }

    Ok(Box::new(GlyphPlugin {
        options: SubsetOptions {
            text: options.text,
            unicodes: options.unicodes,
            unicode_ranges: options.unicode_ranges,
            basic_text: options.basic_text.unwrap_or(false),
            preserve_hinting: options
                .preserve_hinting
                .or(options.hinting)
                .unwrap_or(false),
            trim: options.trim.unwrap_or(true),
            keep_notdef: options.keep_notdef.unwrap_or(true),
            layout: options.keep_layout.map_or(
                LayoutSubsetMode::Conservative,
                layout_subset_mode_from_config,
            ),
        },
        clone: options.clone.unwrap_or(false),
    }))
}

fn css_plugin(config: &PluginConfig) -> Result<Box<dyn FontminPlugin>> {
    let options = plugin_options::<CssPluginOptions>(config)?;
    let mut css = CssConfig::default();

    if let Some(value) = options.font_path {
        css.font_path = value;
    }
    if let Some(value) = options.base64 {
        css.base64 = value;
    }
    if let Some(value) = options.glyph {
        css.glyph = value;
    }
    if let Some(value) = options.icon_prefix {
        css.icon_prefix = value;
    }
    css.font_family = options.font_family;
    css.as_file_name = options.as_file_name;
    if let Some(value) = options.local {
        css.local = value;
    }
    if let Some(value) = options.font_display {
        css.font_display = value;
    }
    if let Some(value) = options.target {
        css.target = value;
    }
    css.unicode_ranges = options.unicode_ranges;

    Ok(Box::new(CssPlugin {
        options: css_options_from_config(Some(css)),
    }))
}

fn plugin_options<T: DeserializeOwned>(config: &PluginConfig) -> Result<T> {
    let value = if config.native.options.is_null() {
        serde_json::json!({})
    } else {
        config.native.options.clone()
    };

    serde_json::from_value(value).map_err(|error| {
        FontminError::config(format!(
            "invalid options for built-in plugin `{}`: {error}",
            config.native.name
        ))
    })
}

struct OutputPathPlugin {
    rules: Vec<OutputPathRule>,
    order: PluginOrder,
}

#[async_trait]
impl FontminPlugin for OutputPathPlugin {
    fn name(&self) -> &'static str {
        "fontmin:output-path"
    }

    fn order(&self) -> PluginOrder {
        self.order
    }

    async fn generate_bundle(
        &self,
        _ctx: &mut PluginContext,
        assets: &mut Vec<Asset>,
    ) -> Result<()> {
        for asset in assets {
            let Some(format) = output_format_from_asset(asset) else {
                continue;
            };
            let Some(rule) = self.rules.iter().find(|rule| rule.format == format) else {
                continue;
            };

            rule.apply(asset);
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct OutputPathRule {
    format: OutputFormat,
    file_name: Option<String>,
    ext: Option<String>,
}

impl OutputPathRule {
    fn from_config(config: &OutputConfig) -> Self {
        Self {
            format: config.format,
            file_name: config.file_name.clone(),
            ext: config.ext.clone(),
        }
    }

    fn apply(&self, asset: &mut Asset) {
        if let Some(file_name) = &self.file_name {
            asset.path = file_name.into();
        } else if let Some(ext) = &self.ext {
            asset.rename_ext(ext);
        }
    }
}

struct OutputFilterPlugin {
    formats: Vec<OutputFormat>,
    order: PluginOrder,
}

#[async_trait]
impl FontminPlugin for OutputFilterPlugin {
    fn name(&self) -> &'static str {
        "fontmin:output-filter"
    }

    fn order(&self) -> PluginOrder {
        self.order
    }

    async fn generate_bundle(
        &self,
        _ctx: &mut PluginContext,
        assets: &mut Vec<Asset>,
    ) -> Result<()> {
        assets.retain(|asset| {
            output_format_from_asset(asset).is_some_and(|format| self.formats.contains(&format))
        });

        Ok(())
    }
}

fn subset_options_from_config(config: SubsetConfig) -> SubsetOptions {
    SubsetOptions {
        text: config.text,
        unicodes: config.unicodes,
        unicode_ranges: Vec::new(),
        basic_text: config.basic_text,
        preserve_hinting: config.preserve_hinting,
        trim: config.trim,
        keep_notdef: config.keep_notdef,
        layout: layout_subset_mode_from_config(config.keep_layout),
    }
}

fn layout_subset_mode_from_config(mode: ConfigLayoutSubsetMode) -> LayoutSubsetMode {
    match mode {
        ConfigLayoutSubsetMode::Drop => LayoutSubsetMode::Drop,
        ConfigLayoutSubsetMode::Conservative => LayoutSubsetMode::Conservative,
        ConfigLayoutSubsetMode::Preserve => LayoutSubsetMode::Preserve,
    }
}

fn css_options_from_config(config: Option<CssConfig>) -> CssOptions {
    let config = config.unwrap_or_default();

    CssOptions {
        font_family: config
            .font_family
            .unwrap_or_else(|| CssOptions::default().font_family),
        font_path: config.font_path,
        base64: config.base64,
        glyph: config.glyph,
        icon_prefix: config.icon_prefix,
        as_file_name: config
            .as_file_name
            .unwrap_or_else(|| CssOptions::default().as_file_name),
        local: config.local,
        font_display: config.font_display,
        target: css_target_from_config(config.target),
        unicode_ranges: config.unicode_ranges,
    }
}

fn css_target_from_config(target: ConfigCssTarget) -> CssTarget {
    match target {
        ConfigCssTarget::Css => CssTarget::Css,
        ConfigCssTarget::Scss => CssTarget::Scss,
        ConfigCssTarget::Less => CssTarget::Less,
    }
}

fn output_format_from_asset(asset: &Asset) -> Option<OutputFormat> {
    match asset.format {
        FontFormat::Ttf => Some(OutputFormat::Ttf),
        FontFormat::Woff => Some(OutputFormat::Woff),
        FontFormat::Woff2 => Some(OutputFormat::Woff2),
        FontFormat::Eot => Some(OutputFormat::Eot),
        FontFormat::Svg => Some(OutputFormat::Svg),
        FontFormat::Css => Some(OutputFormat::Css),
        FontFormat::Otf | FontFormat::Unknown => None,
    }
}
