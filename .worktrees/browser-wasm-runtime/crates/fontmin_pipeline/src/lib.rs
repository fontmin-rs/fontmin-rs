use fontmin_config::{
    CssConfig, CssTarget as ConfigCssTarget, FontminConfig,
    LayoutSubsetMode as ConfigLayoutSubsetMode, OutputConfig, SubsetConfig,
};
use fontmin_core::{Asset, FontFormat, OutputFormat};
use fontmin_css::{CssOptions, CssTarget};
use fontmin_diagnostics::Result;
use fontmin_plugin::{FontminPlugin, PluginContext, PluginOrder, async_trait};
use fontmin_plugins::{
    CssPlugin, GlyphPlugin, Ttf2EotPlugin, Ttf2SvgPlugin, Ttf2Woff2Plugin, Ttf2WoffPlugin,
};
use fontmin_subset::{LayoutSubsetMode, SubsetOptions};

pub struct Engine {
    assets: Vec<Asset>,
    plugins: Vec<Box<dyn FontminPlugin>>,
}

impl Engine {
    #[must_use]
    pub fn new(config: FontminConfig) -> Self {
        let mut engine = Self {
            assets: Vec::new(),
            plugins: Vec::new(),
        };

        engine.configure_builtin_plugins(config);

        engine
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

    fn configure_builtin_plugins(&mut self, config: FontminConfig) {
        let FontminConfig {
            subset,
            outputs,
            css,
            ..
        } = config;

        if let Some(subset) = subset {
            self.plugins.push(Box::new(GlyphPlugin {
                options: subset_options_from_config(subset),
                clone: false,
            }));
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
