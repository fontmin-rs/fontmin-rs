use fontmin_config::{
    BuiltinPlugin, CssConfig, CssPluginConfig, CssTarget as ConfigCssTarget, DeliveryConfig,
    FontminConfig, GlyphPluginConfig, LayoutSubsetMode as ConfigLayoutSubsetMode,
    Otf2TtfPluginConfig, OutputConfig, PluginConfig, PluginEnforce, SubsetConfig,
    Svg2TtfPluginConfig, Svgs2TtfPluginConfig, Ttf2EotPluginConfig, Ttf2SvgPluginConfig,
    Ttf2Woff2PluginConfig, Ttf2WoffPluginConfig, UnicodeSlicesPluginConfig,
    VariationSpacePluginConfig,
};
use fontmin_core::{Asset, FontFormat, OutputFormat};
use fontmin_css::{CssOptions, CssTarget};
use fontmin_diagnostics::{FontminError, Result};
use fontmin_eot::EotOptions;
use fontmin_otf::Otf2TtfOptions;
use fontmin_plugin::{FontminPlugin, PluginOrder, async_trait};
use fontmin_plugins::{
    CssPlugin, GlyphPlugin, Otf2TtfPlugin, SlicePlugin, Svg2TtfPlugin, Svgs2TtfPlugin,
    Ttf2EotPlugin, Ttf2SvgPlugin, Ttf2Woff2Plugin, Ttf2WoffPlugin, VariationSpacePlugin,
};
use fontmin_subset::{LayoutSubsetMode, SubsetOptions};
use fontmin_svg::{Svg2TtfOptions, Svgs2TtfOptions, Ttf2SvgOptions};
use fontmin_woff::WoffOptions;
use fontmin_woff2::Woff2Options;

pub struct Engine {
    assets: Vec<Asset>,
    plugins: Vec<Box<dyn FontminPlugin>>,
    construction_error: Option<FontminError>,
}

impl Engine {
    #[must_use]
    pub fn new(config: FontminConfig) -> Self {
        match Self::try_new(config) {
            Ok(engine) => engine,
            Err(error) => Self {
                assets: Vec::new(),
                plugins: Vec::new(),
                construction_error: Some(error),
            },
        }
    }

    pub fn try_new(config: FontminConfig) -> Result<Self> {
        let has_explicit_plugins = !config.plugins.is_empty();
        let has_legacy_operations = config.subset.is_some()
            || config.delivery.is_some()
            || !config.outputs.is_empty()
            || config.css.is_some();
        let mut engine = Self {
            assets: Vec::new(),
            plugins: Vec::new(),
            construction_error: None,
        };

        engine.configure_explicit_plugins(&config.plugins)?;
        engine.configure_builtin_plugins(config, !has_explicit_plugins && has_legacy_operations);

        Ok(engine)
    }

    #[must_use]
    pub fn from_assets(assets: Vec<Asset>) -> Self {
        Self {
            assets,
            plugins: Vec::new(),
            construction_error: None,
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
        if let Some(error) = self.construction_error.take() {
            return Err(error);
        }

        let assets = std::mem::take(&mut self.assets);
        let mut started_plugin_count = 0;

        self.sort_plugins();
        for plugin in &self.plugins {
            started_plugin_count += 1;
            if let Err(error) = plugin.build_start().await {
                let _cleanup_result = self.run_build_end(started_plugin_count).await;

                return Err(error);
            }
        }

        let result = self.run_pipeline(assets).await;
        let cleanup_result = self.run_build_end(started_plugin_count).await;

        match (result, cleanup_result) {
            (Err(error), _) | (Ok(_), Err(error)) => Err(error),
            (Ok(assets), Ok(())) => Ok(assets),
        }
    }

    async fn run_pipeline(&self, mut assets: Vec<Asset>) -> Result<Vec<Asset>> {
        for plugin in &self.plugins {
            let mut next_assets = Vec::new();

            for asset in assets {
                next_assets.extend(plugin.transform(asset).await?);
            }

            assets = next_assets;
        }

        for plugin in &self.plugins {
            plugin.generate_bundle(&mut assets).await?;
        }

        Ok(assets)
    }

    async fn run_build_end(&self, started_plugin_count: usize) -> Result<()> {
        let mut first_error = None;

        for plugin in self.plugins.iter().take(started_plugin_count) {
            if let Err(error) = plugin.build_end().await {
                first_error.get_or_insert(error);
            }
        }

        first_error.map_or(Ok(()), Err)
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

    fn configure_builtin_plugins(&mut self, config: FontminConfig, add_implicit_otf: bool) {
        let FontminConfig {
            subset,
            delivery,
            outputs,
            css,
            otf,
            ..
        } = config;

        if add_implicit_otf {
            self.plugins.push(Box::new(Otf2TtfPlugin {
                options: Otf2TtfOptions {
                    preserve_hinting: otf.preserve_hinting,
                    variation_coordinates: otf.variation_coordinates,
                },
                clone: false,
            }));
        }

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

    async fn build_start(&self) -> Result<()> {
        self.inner.build_start().await
    }

    async fn transform(&self, asset: Asset) -> Result<Vec<Asset>> {
        self.inner.transform(asset).await
    }

    async fn generate_bundle(&self, assets: &mut Vec<Asset>) -> Result<()> {
        self.inner.generate_bundle(assets).await
    }

    async fn build_end(&self) -> Result<()> {
        self.inner.build_end().await
    }
}

fn configured_plugin(config: &PluginConfig) -> Result<Box<dyn FontminPlugin>> {
    let expected_name = config.native.public_name();

    if config.name != expected_name {
        return Err(FontminError::config(format!(
            "built-in plugin `{}` must use public name `{expected_name}`, got `{}`",
            config.native.name(),
            config.name,
        )));
    }

    match &config.native.plugin {
        BuiltinPlugin::Glyph(options) => glyph_plugin(options),
        BuiltinPlugin::UnicodeSlices(options) => slice_plugin(options),
        BuiltinPlugin::VariationSpace(options) => Ok(variation_space_plugin(options)),
        BuiltinPlugin::Otf2Ttf(options) => Ok(otf_plugin(options)),
        BuiltinPlugin::Ttf2Woff(options) => Ok(woff_plugin(options)),
        BuiltinPlugin::Ttf2Woff2(options) => Ok(woff2_plugin(options)),
        BuiltinPlugin::Ttf2Eot(options) => Ok(eot_plugin(options)),
        BuiltinPlugin::Ttf2Svg(options) => Ok(ttf_svg_plugin(options)),
        BuiltinPlugin::Svg2Ttf(options) => Ok(svg_ttf_plugin(options)),
        BuiltinPlugin::Svgs2Ttf(options) => Ok(svg_collection_plugin(options)),
        BuiltinPlugin::Css(options) => Ok(css_plugin(options)),
    }
}

fn glyph_plugin(options: &GlyphPluginConfig) -> Result<Box<dyn FontminPlugin>> {
    if options.text_file.is_some() {
        return Err(unsupported_plugin_option("glyph", "textFile"));
    }

    let mut subset = SubsetOptions::default();
    subset.text.clone_from(&options.text);
    subset.unicodes.clone_from(&options.unicodes);
    subset.gids.clone_from(&options.gids);
    subset.glyph_names.clone_from(&options.glyph_names);
    subset.unicode_ranges.clone_from(&options.unicode_ranges);
    subset.basic_text = options.basic_text.unwrap_or(subset.basic_text);
    subset.preserve_hinting = options
        .preserve_hinting
        .or(options.hinting)
        .unwrap_or(subset.preserve_hinting);
    subset.trim = options.trim.unwrap_or(subset.trim);
    subset.keep_notdef = options.keep_notdef.unwrap_or(subset.keep_notdef);
    subset.retain_gids = options.retain_gids.unwrap_or(subset.retain_gids);
    subset.retain_glyph_names = options
        .retain_glyph_names
        .unwrap_or(subset.retain_glyph_names);
    subset.retain_legacy_cmap = options
        .retain_legacy_cmap
        .unwrap_or(subset.retain_legacy_cmap);
    subset.retain_symbol_cmap = options
        .retain_symbol_cmap
        .unwrap_or(subset.retain_symbol_cmap);
    if let Some(layout) = options.keep_layout {
        subset.layout = layout_subset_mode_from_config(layout);
    }
    subset.layout_features.clone_from(&options.layout_features);
    subset.layout_scripts.clone_from(&options.layout_scripts);
    subset
        .layout_languages
        .clone_from(&options.layout_languages);
    subset.name_ids.clone_from(&options.name_ids);
    subset.name_languages.clone_from(&options.name_languages);
    subset.drop_tables.clone_from(&options.drop_tables);
    subset
        .pass_through_tables
        .clone_from(&options.pass_through_tables);
    subset.missing_glyphs = options.missing_glyphs.unwrap_or_default();

    Ok(Box::new(GlyphPlugin {
        options: subset,
        clone: options.clone.unwrap_or(false),
    }))
}

fn slice_plugin(options: &UnicodeSlicesPluginConfig) -> Result<Box<dyn FontminPlugin>> {
    if options.slices.is_empty() {
        return Err(FontminError::config(
            "unicode delivery slices must not be empty",
        ));
    }

    Ok(Box::new(SlicePlugin {
        slices: options.slices.clone(),
    }))
}

fn variation_space_plugin(options: &VariationSpacePluginConfig) -> Box<dyn FontminPlugin> {
    Box::new(VariationSpacePlugin {
        options: fontmin_subset::VariationSpaceOptions {
            axes: options.axes.clone(),
            downgrade_cff2: options.downgrade_cff2,
        },
        clone: options.clone.unwrap_or(false),
    })
}

fn otf_plugin(options: &Otf2TtfPluginConfig) -> Box<dyn FontminPlugin> {
    let mut plugin = Otf2TtfPlugin::default();
    plugin.clone = options.clone.unwrap_or(plugin.clone);
    plugin.options.preserve_hinting = options.preserve_hinting.unwrap_or(false);
    plugin
        .options
        .variation_coordinates
        .clone_from(&options.variation_coordinates);

    Box::new(plugin)
}

fn woff_plugin(options: &Ttf2WoffPluginConfig) -> Box<dyn FontminPlugin> {
    let mut plugin = Ttf2WoffPlugin::default();
    let mut woff = WoffOptions::default();
    plugin.clone = options.clone.unwrap_or(plugin.clone);
    woff.deflate = options.deflate.unwrap_or(woff.deflate);
    woff.compression_level = options.compression_level;
    woff.metadata.clone_from(&options.metadata);
    plugin.options = woff;

    Box::new(plugin)
}

fn woff2_plugin(options: &Ttf2Woff2PluginConfig) -> Box<dyn FontminPlugin> {
    let mut plugin = Ttf2Woff2Plugin::default();
    plugin.clone = options.clone.unwrap_or(plugin.clone);
    plugin.options = Woff2Options {
        quality: options.quality,
    };

    Box::new(plugin)
}

fn eot_plugin(options: &Ttf2EotPluginConfig) -> Box<dyn FontminPlugin> {
    let mut plugin = Ttf2EotPlugin::default();
    plugin.clone = options.clone.unwrap_or(plugin.clone);
    plugin.options = EotOptions {
        version: options.version,
    };

    Box::new(plugin)
}

fn ttf_svg_plugin(options: &Ttf2SvgPluginConfig) -> Box<dyn FontminPlugin> {
    let mut plugin = Ttf2SvgPlugin::default();
    plugin.clone = options.clone.unwrap_or(plugin.clone);
    plugin.options = Ttf2SvgOptions {
        font_family: options.font_family.clone(),
    };

    Box::new(plugin)
}

fn svg_ttf_plugin(options: &Svg2TtfPluginConfig) -> Box<dyn FontminPlugin> {
    let mut plugin = Svg2TtfPlugin::default();
    plugin.clone = options.clone.unwrap_or(plugin.clone);
    plugin.options = Svg2TtfOptions {
        hinting: options.hinting.unwrap_or(plugin.options.hinting),
        normalize: options.normalize.unwrap_or(plugin.options.normalize),
    };

    Box::new(plugin)
}

fn svg_collection_plugin(options: &Svgs2TtfPluginConfig) -> Box<dyn FontminPlugin> {
    let mut svg = Svgs2TtfOptions::default();
    svg.font_name = options
        .font_name
        .clone()
        .unwrap_or_else(|| svg.font_name.clone());
    svg.start_unicode = options.start_unicode.unwrap_or(svg.start_unicode);
    svg.ascent = options.ascent.unwrap_or(svg.ascent);
    svg.descent = options.descent.unwrap_or(svg.descent);
    svg.normalize = options.normalize.unwrap_or(svg.normalize);

    Box::new(Svgs2TtfPlugin {
        options: svg,
        clone: options.clone.unwrap_or(false),
    })
}

fn css_plugin(options: &CssPluginConfig) -> Box<dyn FontminPlugin> {
    let mut css = CssOptions::default();
    css.font_path = options
        .font_path
        .clone()
        .unwrap_or_else(|| css.font_path.clone());
    css.base64 = options.base64.unwrap_or(css.base64);
    css.glyph = options.glyph.unwrap_or(css.glyph);
    css.icon_prefix = options
        .icon_prefix
        .clone()
        .unwrap_or_else(|| css.icon_prefix.clone());
    css.font_family = options
        .font_family
        .clone()
        .unwrap_or_else(|| css.font_family.clone());
    css.as_file_name = options.as_file_name.unwrap_or(css.as_file_name);
    css.local = options.local.unwrap_or(css.local);
    css.font_display = options
        .font_display
        .clone()
        .unwrap_or_else(|| css.font_display.clone());
    if let Some(target) = options.target {
        css.target = css_target_from_config(target);
    }
    css.unicode_ranges.clone_from(&options.unicode_ranges);

    Box::new(CssPlugin { options: css })
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

    async fn generate_bundle(&self, assets: &mut Vec<Asset>) -> Result<()> {
        for asset in assets {
            let Some(format) = output_format_from_asset(asset) else {
                continue;
            };
            let Some(rule) = self.rules.iter().find(|rule| rule.format == format) else {
                continue;
            };

            rule.apply(asset)?;
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

    fn apply(&self, asset: &mut Asset) -> Result<()> {
        if let Some(file_name) = &self.file_name {
            asset.path = file_name.into();
        } else if let Some(ext) = &self.ext {
            asset.rename_ext(ext)?;
        }

        Ok(())
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

    async fn generate_bundle(&self, assets: &mut Vec<Asset>) -> Result<()> {
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
        gids: config.gids,
        glyph_names: config.glyph_names,
        basic_text: config.basic_text,
        preserve_hinting: config.preserve_hinting,
        trim: config.trim,
        keep_notdef: config.keep_notdef,
        retain_gids: config.retain_gids,
        retain_glyph_names: config.retain_glyph_names,
        retain_legacy_cmap: config.retain_legacy_cmap,
        retain_symbol_cmap: config.retain_symbol_cmap,
        layout: layout_subset_mode_from_config(config.keep_layout),
        layout_features: config.layout_features,
        layout_scripts: config.layout_scripts,
        layout_languages: config.layout_languages,
        name_ids: config.name_ids,
        name_languages: config.name_languages,
        drop_tables: config.drop_tables,
        pass_through_tables: config.pass_through_tables,
        missing_glyphs: config.missing_glyphs,
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
