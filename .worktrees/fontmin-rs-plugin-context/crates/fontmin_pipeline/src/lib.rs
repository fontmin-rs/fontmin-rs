use fontmin_config::{
    CssConfig, FontminConfig, LayoutSubsetMode as ConfigLayoutSubsetMode, SubsetConfig,
};
use fontmin_core::{Asset, FontFormat, OutputFormat};
use fontmin_css::CssOptions;
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
        let mut css_requested = false;

        for output in outputs {
            requested_outputs.push(output.format);

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

        if css_requested {
            self.plugins.push(Box::new(CssPlugin {
                options: css_options_from_config(css),
            }));
            self.plugins.push(Box::new(OutputFilterPlugin {
                formats: requested_outputs,
                order: PluginOrder::Post,
            }));
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
        local: config.local,
        font_display: config.font_display,
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
