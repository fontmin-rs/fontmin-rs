use std::collections::BTreeMap;

use fontmin_config::{
    CssConfig, CssTarget as ConfigCssTarget, DeliveryConfig, FontminConfig,
    LayoutSubsetMode as ConfigLayoutSubsetMode, OutputConfig, PluginConfig, PluginEnforce,
    SubsetConfig,
};
use fontmin_core::{Asset, FontDeliverySlice, FontFormat, OutputFormat, UnicodeRange};
use fontmin_css::{CssOptions, CssTarget};
use fontmin_diagnostics::{FontminError, Result};
use fontmin_eot::EotOptions;
use fontmin_otf::Otf2TtfOptions;
use fontmin_plugin::{FontminPlugin, PluginContext, PluginKind, PluginOrder, async_trait};
use fontmin_plugins::{
    CssPlugin, GlyphPlugin, Otf2TtfPlugin, SlicePlugin, Svg2TtfPlugin, Svgs2TtfPlugin,
    Ttf2EotPlugin, Ttf2SvgPlugin, Ttf2Woff2Plugin, Ttf2WoffPlugin,
};
use fontmin_subset::{LayoutSubsetMode, SubsetOptions};
use fontmin_svg::{Svg2TtfOptions, Svgs2TtfOptions, Ttf2SvgOptions};
use fontmin_woff::WoffOptions;
use fontmin_woff2::Woff2Options;
use serde::Deserialize;

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

        engine.configure_explicit_plugins(&config.plugins)?;
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

    fn configure_explicit_plugins(&mut self, configs: &[PluginConfig]) -> Result<()> {
        for config in configs {
            let plugin = configured_plugin(config)?;
            let order = match config.enforce {
                Some(PluginEnforce::Pre) => PluginOrder::Pre,
                Some(PluginEnforce::Post) => PluginOrder::Post,
                None => plugin.order(),
            };

            self.plugins.push(Box::new(OrderedPlugin {
                inner: plugin,
                order,
            }));
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

fn configured_plugin(config: &PluginConfig) -> Result<Box<dyn FontminPlugin>> {
    let expected_name = if config.native.name == "unicodeSlices" {
        "fontmin:unicode-slices".to_string()
    } else {
        format!("fontmin:{}", config.native.name)
    };

    if config.name != expected_name {
        return Err(FontminError::config(format!(
            "built-in plugin `{}` must use public name `{expected_name}`, got `{}`",
            config.native.name, config.name,
        )));
    }

    match config.native.name.as_str() {
        "glyph" => glyph_plugin(config),
        "unicodeSlices" => slice_plugin(config),
        "otf2ttf" => otf_plugin(config),
        "ttf2woff" => woff_plugin(config),
        "ttf2woff2" => woff2_plugin(config),
        "ttf2eot" => eot_plugin(config),
        "ttf2svg" => ttf_svg_plugin(config),
        "svg2ttf" => svg_ttf_plugin(config),
        "svgs2ttf" => svg_collection_plugin(config),
        "css" => css_plugin(config),
        name => Err(FontminError::config(format!(
            "unsupported built-in plugin `{name}`",
        ))),
    }
}

fn plugin_options<T>(config: &PluginConfig) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let options = if config.native.options.is_null() {
        serde_json::json!({})
    } else {
        config.native.options.clone()
    };

    serde_json::from_value(options).map_err(|error| {
        FontminError::config(format!(
            "invalid options for built-in plugin `{}`: {error}",
            config.native.name,
        ))
    })
}

fn glyph_plugin(config: &PluginConfig) -> Result<Box<dyn FontminPlugin>> {
    let options: GlyphPluginOptions = plugin_options(config)?;
    if options.text_file.is_some() {
        return Err(unsupported_plugin_option("glyph", "textFile"));
    }

    let mut subset = SubsetOptions::default();
    subset.text = options.text;
    subset.unicodes = options.unicodes;
    subset.unicode_ranges = options.unicode_ranges;
    subset.basic_text = options.basic_text.unwrap_or(subset.basic_text);
    subset.preserve_hinting = options
        .preserve_hinting
        .or(options.hinting)
        .unwrap_or(subset.preserve_hinting);
    subset.trim = options.trim.unwrap_or(subset.trim);
    subset.keep_notdef = options.keep_notdef.unwrap_or(subset.keep_notdef);
    if let Some(layout) = options.keep_layout {
        subset.layout = layout_subset_mode_from_config(layout);
    }

    Ok(Box::new(GlyphPlugin {
        options: subset,
        clone: options.clone.unwrap_or(false),
    }))
}

fn slice_plugin(config: &PluginConfig) -> Result<Box<dyn FontminPlugin>> {
    let options: SlicePluginOptions = plugin_options(config)?;
    if options.slices.is_empty() {
        return Err(FontminError::config(
            "unicode delivery slices must not be empty",
        ));
    }

    Ok(Box::new(SlicePlugin {
        slices: options.slices,
    }))
}

fn otf_plugin(config: &PluginConfig) -> Result<Box<dyn FontminPlugin>> {
    let options: OtfPluginOptions = plugin_options(config)?;
    let mut plugin = Otf2TtfPlugin::default();
    plugin.clone = options.clone.unwrap_or(plugin.clone);
    plugin.options.preserve_hinting = options.preserve_hinting.unwrap_or(false);
    plugin.options.variation_coordinates = options.variation_coordinates;

    Ok(Box::new(plugin))
}

fn woff_plugin(config: &PluginConfig) -> Result<Box<dyn FontminPlugin>> {
    let options: WoffPluginOptions = plugin_options(config)?;
    let mut plugin = Ttf2WoffPlugin::default();
    let mut woff = WoffOptions::default();
    plugin.clone = options.clone.unwrap_or(plugin.clone);
    woff.deflate = options.deflate.unwrap_or(woff.deflate);
    woff.compression_level = options.compression_level;
    woff.metadata = options.metadata;
    plugin.options = woff;

    Ok(Box::new(plugin))
}

fn woff2_plugin(config: &PluginConfig) -> Result<Box<dyn FontminPlugin>> {
    let options: Woff2PluginOptions = plugin_options(config)?;
    let mut plugin = Ttf2Woff2Plugin::default();
    plugin.clone = options.clone.unwrap_or(plugin.clone);
    plugin.options = Woff2Options {
        quality: options.quality,
    };

    Ok(Box::new(plugin))
}

fn eot_plugin(config: &PluginConfig) -> Result<Box<dyn FontminPlugin>> {
    let options: EotPluginOptions = plugin_options(config)?;
    let mut plugin = Ttf2EotPlugin::default();
    plugin.clone = options.clone.unwrap_or(plugin.clone);
    plugin.options = EotOptions {
        version: options.version,
    };

    Ok(Box::new(plugin))
}

fn ttf_svg_plugin(config: &PluginConfig) -> Result<Box<dyn FontminPlugin>> {
    let options: TtfSvgPluginOptions = plugin_options(config)?;
    let mut plugin = Ttf2SvgPlugin::default();
    plugin.clone = options.clone.unwrap_or(plugin.clone);
    plugin.options = Ttf2SvgOptions {
        font_family: options.font_family,
    };

    Ok(Box::new(plugin))
}

fn svg_ttf_plugin(config: &PluginConfig) -> Result<Box<dyn FontminPlugin>> {
    let options: SvgTtfPluginOptions = plugin_options(config)?;
    let mut plugin = Svg2TtfPlugin::default();
    plugin.clone = options.clone.unwrap_or(plugin.clone);
    plugin.options = Svg2TtfOptions {
        hinting: options.hinting.unwrap_or(plugin.options.hinting),
        normalize: options.normalize.unwrap_or(plugin.options.normalize),
    };

    Ok(Box::new(plugin))
}

fn svg_collection_plugin(config: &PluginConfig) -> Result<Box<dyn FontminPlugin>> {
    let options: SvgCollectionPluginOptions = plugin_options(config)?;
    let derive_font_name_from_first_svg = options.font_name.is_none();
    let mut svg = Svgs2TtfOptions::default();
    svg.font_name = options.font_name.unwrap_or(svg.font_name);
    svg.start_unicode = options.start_unicode.unwrap_or(svg.start_unicode);
    svg.ascent = options.ascent.unwrap_or(svg.ascent);
    svg.descent = options.descent.unwrap_or(svg.descent);
    svg.normalize = options.normalize.unwrap_or(svg.normalize);

    Ok(Box::new(Svgs2TtfPlugin {
        options: svg,
        clone: options.clone.unwrap_or(false),
        derive_font_name_from_first_svg,
    }))
}

fn css_plugin(config: &PluginConfig) -> Result<Box<dyn FontminPlugin>> {
    let options: CssPluginOptions = plugin_options(config)?;
    let mut css = CssOptions::default();
    css.font_path = options.font_path.unwrap_or(css.font_path);
    css.base64 = options.base64.unwrap_or(css.base64);
    css.glyph = options.glyph.unwrap_or(css.glyph);
    css.icon_prefix = options.icon_prefix.unwrap_or(css.icon_prefix);
    css.font_family = options.font_family.unwrap_or(css.font_family);
    css.as_file_name = options.as_file_name.unwrap_or(css.as_file_name);
    css.local = options.local.unwrap_or(css.local);
    css.font_display = options.font_display.unwrap_or(css.font_display);
    if let Some(target) = options.target {
        css.target = css_target_from_config(target);
    }
    css.unicode_ranges = options.unicode_ranges;

    Ok(Box::new(CssPlugin { options: css }))
}

fn unsupported_plugin_option(plugin: &str, option: &str) -> FontminError {
    FontminError::config(format!(
        "built-in plugin `{plugin}` option `{option}` is not supported by the Rust pipeline",
    ))
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
