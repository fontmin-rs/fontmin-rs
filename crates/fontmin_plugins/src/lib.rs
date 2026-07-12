use std::collections::BTreeSet;

use fontmin_core::{
    Asset, FontDeliverySlice, FontFormat, OutputFormat, UnicodeRange, validate_delivery_slices,
};
use fontmin_css::{CssFontSource, CssGlyph, CssOptions};
use fontmin_diagnostics::{FontminError, Result};
use fontmin_eot::EotOptions;
use fontmin_otf::Otf2TtfOptions;
use fontmin_plugin::{FontminPlugin, PluginContext, PluginKind, PluginOrder, async_trait};
use fontmin_subset::SubsetOptions;
use fontmin_svg::{Svg2TtfOptions, SvgIcon, Svgs2TtfOptions, Ttf2SvgOptions};
use fontmin_woff::WoffOptions;
use fontmin_woff2::Woff2Options;

const CSS_GLYPHS_META_KEY: &str = "cssGlyphs";
const CSS_UNICODE_RANGES_META_KEY: &str = "cssUnicodeRanges";

#[derive(Debug, Clone, Default)]
pub struct GlyphPlugin {
    pub options: SubsetOptions,
    pub clone: bool,
}

#[async_trait]
impl FontminPlugin for GlyphPlugin {
    fn name(&self) -> &'static str {
        "fontmin:glyph"
    }

    async fn transform(&self, _ctx: &mut PluginContext, asset: Asset) -> Result<Vec<Asset>> {
        if asset.format != FontFormat::Ttf {
            return Ok(vec![asset]);
        }

        let mut subset = asset.clone();
        subset.contents = fontmin_subset::subset_ttf(&asset.contents, self.options.clone())?;
        subset.format = FontFormat::Ttf;
        set_css_glyphs(
            &mut subset,
            &fontmin_css::css_glyphs_from_text(
                self.options.text.as_deref().unwrap_or_default(),
                &self.options.unicodes,
            ),
        );
        subset.meta.generated_by.push(self.name().into());

        if self.clone {
            Ok(vec![asset, subset])
        } else {
            Ok(vec![subset])
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SlicePlugin {
    pub slices: Vec<FontDeliverySlice>,
}

#[async_trait]
impl FontminPlugin for SlicePlugin {
    fn name(&self) -> &'static str {
        "fontmin:unicode-slices"
    }

    async fn transform(&self, _ctx: &mut PluginContext, asset: Asset) -> Result<Vec<Asset>> {
        if asset.format != FontFormat::Ttf || self.slices.is_empty() {
            return Ok(vec![asset]);
        }

        validate_delivery_slices(&self.slices)?;

        self.slices
            .iter()
            .map(|slice| sliced_asset(&asset, slice, self.name()))
            .collect()
    }
}

fn sliced_asset(asset: &Asset, slice: &FontDeliverySlice, generated_by: &str) -> Result<Asset> {
    let mut subset = asset.clone();
    subset.contents = fontmin_subset::subset_ttf(
        &asset.contents,
        SubsetOptions {
            unicode_ranges: slice.unicode_ranges.clone(),
            ..SubsetOptions::default()
        },
    )?;
    let stem = asset
        .path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("fontmin");
    let extension = asset
        .path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("ttf");
    subset
        .path
        .set_file_name(format!("{stem}-{}.{}", slice.name, extension));
    subset.meta.custom.insert(
        CSS_UNICODE_RANGES_META_KEY.into(),
        serde_json::json!(slice.unicode_ranges),
    );
    subset.meta.generated_by.push(generated_by.into());

    Ok(subset)
}

#[derive(Debug, Clone, Default)]
pub struct Svgs2TtfPlugin {
    pub options: Svgs2TtfOptions,
    pub clone: bool,
}

#[async_trait]
impl FontminPlugin for Svgs2TtfPlugin {
    fn name(&self) -> &'static str {
        "fontmin:svgs2ttf"
    }

    fn kind(&self) -> PluginKind {
        PluginKind::Generator
    }

    async fn generate_bundle(
        &self,
        _ctx: &mut PluginContext,
        assets: &mut Vec<Asset>,
    ) -> Result<()> {
        let svg_assets = assets
            .iter()
            .enumerate()
            .filter(|(_, asset)| asset.format == FontFormat::Svg)
            .collect::<Vec<_>>();

        if svg_assets.is_empty() {
            return Ok(());
        }

        let first_svg = svg_assets[0].1;
        let icons = svg_assets
            .iter()
            .enumerate()
            .map(|(index, (_, asset))| svg_icon_from_asset(asset, index))
            .collect::<Result<Vec<_>>>()?;
        let css_glyphs = css_glyphs_from_svg_icons(&icons, self.options.start_unicode);
        let mut ttf = Asset::new(
            format!("{}.ttf", self.options.font_name).into(),
            fontmin_svg::svgs_to_ttf(icons, &self.options)?,
            FontFormat::Ttf,
        );

        ttf.source_format = first_svg.source_format;
        set_css_glyphs(&mut ttf, &css_glyphs);
        ttf.meta.generated_by.push(self.name().into());

        if self.clone {
            assets.push(ttf);
        } else {
            let mut next_assets = Vec::with_capacity(assets.len() - svg_assets.len() + 1);

            next_assets.extend(
                assets
                    .drain(..)
                    .filter(|asset| asset.format != FontFormat::Svg),
            );
            next_assets.push(ttf);
            *assets = next_assets;
        }

        Ok(())
    }
}

fn svg_icon_from_asset(asset: &Asset, index: usize) -> Result<SvgIcon> {
    let contents = std::str::from_utf8(&asset.contents)
        .map_err(|error| FontminError::invalid_font(format!("invalid SVG UTF-8: {error}")))?
        .to_string();
    let name = asset
        .path
        .file_stem()
        .filter(|stem| !stem.is_empty())
        .map_or_else(
            || format!("glyph-{}", index + 1),
            |stem| stem.to_string_lossy().into_owned(),
        );
    let unicode = asset.meta.custom.get("unicode").and_then(unicode_from_json);

    Ok(SvgIcon {
        name,
        contents,
        unicode,
    })
}

fn unicode_from_json(value: &serde_json::Value) -> Option<u32> {
    value.as_u64().and_then(|value| u32::try_from(value).ok())
}

fn css_glyphs_from_svg_icons(icons: &[SvgIcon], start_unicode: u32) -> Vec<CssGlyph> {
    let mut next_unicode = start_unicode;
    let mut used = BTreeSet::new();
    let mut glyphs = Vec::with_capacity(icons.len());

    for icon in icons {
        let unicode = if let Some(unicode) = icon.unicode {
            unicode
        } else {
            while used.contains(&next_unicode) {
                let Some(next) = next_unicode.checked_add(1) else {
                    break;
                };
                next_unicode = next;
            }
            let unicode = next_unicode;

            if let Some(next) = next_unicode.checked_add(1) {
                next_unicode = next;
            }

            unicode
        };

        used.insert(unicode);
        glyphs.push(CssGlyph::new(Some(icon.name.clone()), unicode));
    }

    glyphs
}

#[derive(Debug, Clone, Default)]
pub struct CssPlugin {
    pub options: CssOptions,
}

#[async_trait]
impl FontminPlugin for CssPlugin {
    fn name(&self) -> &'static str {
        "fontmin:css"
    }

    fn kind(&self) -> PluginKind {
        PluginKind::Generator
    }

    async fn generate_bundle(
        &self,
        _ctx: &mut PluginContext,
        assets: &mut Vec<Asset>,
    ) -> Result<()> {
        let Some(first_source) = assets
            .iter()
            .find(|asset| css_output_format(asset.format).is_some())
        else {
            return Ok(());
        };
        let output_path = first_source.path.clone();
        let source_format = first_source.source_format;
        let sources = assets
            .iter()
            .filter_map(css_source_from_asset)
            .collect::<Vec<_>>();
        let mut css = Asset::new(
            output_path,
            fontmin_css::generate_font_face_css(&sources, &self.options)?.into_bytes(),
            FontFormat::Css,
        );

        css.rename_ext(self.options.target.extension());
        css.source_format = source_format;
        css.meta.generated_by.push(self.name().into());
        assets.push(css);

        Ok(())
    }
}

fn css_source_from_asset(asset: &Asset) -> Option<CssFontSource> {
    let format = css_output_format(asset.format)?;

    let unicode_ranges = asset
        .meta
        .custom
        .get(CSS_UNICODE_RANGES_META_KEY)
        .and_then(|value| serde_json::from_value::<Vec<UnicodeRange>>(value.clone()).ok())
        .unwrap_or_default();

    Some(
        CssFontSource::new(asset.path.to_string_lossy().into_owned(), format)
            .with_contents(asset.contents.clone())
            .with_glyphs(css_glyphs_from_asset(asset))
            .with_unicode_ranges(unicode_ranges),
    )
}

fn css_output_format(format: FontFormat) -> Option<OutputFormat> {
    match format {
        FontFormat::Ttf => Some(OutputFormat::Ttf),
        FontFormat::Woff => Some(OutputFormat::Woff),
        FontFormat::Woff2 => Some(OutputFormat::Woff2),
        FontFormat::Eot => Some(OutputFormat::Eot),
        FontFormat::Svg => Some(OutputFormat::Svg),
        FontFormat::Otf | FontFormat::Css | FontFormat::Unknown => None,
    }
}

fn css_glyphs_from_asset(asset: &Asset) -> Vec<CssGlyph> {
    asset
        .meta
        .custom
        .get(CSS_GLYPHS_META_KEY)
        .map_or_else(Vec::new, |value| {
            serde_json::from_value(value.clone()).unwrap_or_default()
        })
}

fn set_css_glyphs(asset: &mut Asset, glyphs: &[CssGlyph]) {
    if glyphs.is_empty() {
        return;
    }

    asset
        .meta
        .custom
        .insert(CSS_GLYPHS_META_KEY.into(), serde_json::json!(glyphs));
}

#[derive(Debug, Clone)]
pub struct Otf2TtfPlugin {
    pub options: Otf2TtfOptions,
    pub clone: bool,
}

impl Default for Otf2TtfPlugin {
    fn default() -> Self {
        Self {
            options: Otf2TtfOptions::default(),
            clone: true,
        }
    }
}

#[async_trait]
impl FontminPlugin for Otf2TtfPlugin {
    fn name(&self) -> &'static str {
        "fontmin:otf2ttf"
    }

    fn order(&self) -> PluginOrder {
        PluginOrder::Pre
    }

    async fn transform(&self, _ctx: &mut PluginContext, asset: Asset) -> Result<Vec<Asset>> {
        if asset.format != FontFormat::Otf {
            return Ok(vec![asset]);
        }

        let mut ttf = asset.clone();

        ttf.contents = fontmin_otf::otf_to_ttf(&asset.contents, &self.options)?;
        ttf.format = FontFormat::Ttf;
        ttf.rename_ext("ttf");
        ttf.meta.generated_by.push(self.name().into());

        if self.clone {
            Ok(vec![asset, ttf])
        } else {
            Ok(vec![ttf])
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ttf2EotPlugin {
    pub options: EotOptions,
    pub clone: bool,
}

impl Default for Ttf2EotPlugin {
    fn default() -> Self {
        Self {
            options: EotOptions::default(),
            clone: true,
        }
    }
}

#[async_trait]
impl FontminPlugin for Ttf2EotPlugin {
    fn name(&self) -> &'static str {
        "fontmin:ttf2eot"
    }

    async fn transform(&self, _ctx: &mut PluginContext, asset: Asset) -> Result<Vec<Asset>> {
        if asset.format != FontFormat::Ttf {
            return Ok(vec![asset]);
        }

        let mut eot = asset.clone();
        eot.contents = fontmin_eot::encode_ttf_to_eot(&asset.contents, &self.options)?;
        eot.format = FontFormat::Eot;
        eot.rename_ext("eot");
        eot.meta.generated_by.push(self.name().into());

        if self.clone {
            Ok(vec![asset, eot])
        } else {
            Ok(vec![eot])
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ttf2SvgPlugin {
    pub options: Ttf2SvgOptions,
    pub clone: bool,
}

impl Default for Ttf2SvgPlugin {
    fn default() -> Self {
        Self {
            options: Ttf2SvgOptions::default(),
            clone: true,
        }
    }
}

#[async_trait]
impl FontminPlugin for Ttf2SvgPlugin {
    fn name(&self) -> &'static str {
        "fontmin:ttf2svg"
    }

    async fn transform(&self, _ctx: &mut PluginContext, asset: Asset) -> Result<Vec<Asset>> {
        if asset.format != FontFormat::Ttf {
            return Ok(vec![asset]);
        }

        let mut svg = asset.clone();
        svg.contents = fontmin_svg::ttf_to_svg(&asset.contents, &self.options)?.into_bytes();
        svg.format = FontFormat::Svg;
        svg.rename_ext("svg");
        svg.meta.generated_by.push(self.name().into());

        if self.clone {
            Ok(vec![asset, svg])
        } else {
            Ok(vec![svg])
        }
    }
}

#[derive(Debug, Clone)]
pub struct Svg2TtfPlugin {
    pub options: Svg2TtfOptions,
    pub clone: bool,
}

impl Default for Svg2TtfPlugin {
    fn default() -> Self {
        Self {
            options: Svg2TtfOptions::default(),
            clone: true,
        }
    }
}

#[async_trait]
impl FontminPlugin for Svg2TtfPlugin {
    fn name(&self) -> &'static str {
        "fontmin:svg2ttf"
    }

    async fn transform(&self, _ctx: &mut PluginContext, asset: Asset) -> Result<Vec<Asset>> {
        if asset.format != FontFormat::Svg {
            return Ok(vec![asset]);
        }

        let svg_text = std::str::from_utf8(&asset.contents)
            .map_err(|error| FontminError::invalid_font(format!("invalid SVG UTF-8: {error}")))?;
        let mut ttf = asset.clone();
        ttf.contents = fontmin_svg::svg_font_to_ttf(svg_text, &self.options)?;
        ttf.format = FontFormat::Ttf;
        ttf.rename_ext("ttf");
        ttf.meta.generated_by.push(self.name().into());

        if self.clone {
            Ok(vec![asset, ttf])
        } else {
            Ok(vec![ttf])
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ttf2WoffPlugin {
    pub options: WoffOptions,
    pub clone: bool,
}

impl Default for Ttf2WoffPlugin {
    fn default() -> Self {
        Self {
            options: WoffOptions::default(),
            clone: true,
        }
    }
}

#[async_trait]
impl FontminPlugin for Ttf2WoffPlugin {
    fn name(&self) -> &'static str {
        "fontmin:ttf2woff"
    }

    async fn transform(&self, _ctx: &mut PluginContext, asset: Asset) -> Result<Vec<Asset>> {
        if asset.format != FontFormat::Ttf {
            return Ok(vec![asset]);
        }

        let mut woff = asset.clone();
        woff.contents = fontmin_woff::encode_ttf_to_woff(&asset.contents, &self.options)?;
        woff.format = FontFormat::Woff;
        woff.rename_ext("woff");
        woff.meta.generated_by.push(self.name().into());

        if self.clone {
            Ok(vec![asset, woff])
        } else {
            Ok(vec![woff])
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ttf2Woff2Plugin {
    pub options: Woff2Options,
    pub clone: bool,
}

impl Default for Ttf2Woff2Plugin {
    fn default() -> Self {
        Self {
            options: Woff2Options::default(),
            clone: true,
        }
    }
}

#[async_trait]
impl FontminPlugin for Ttf2Woff2Plugin {
    fn name(&self) -> &'static str {
        "fontmin:ttf2woff2"
    }

    async fn transform(&self, _ctx: &mut PluginContext, asset: Asset) -> Result<Vec<Asset>> {
        if asset.format != FontFormat::Ttf {
            return Ok(vec![asset]);
        }

        let mut woff2 = asset.clone();
        woff2.contents = fontmin_woff2::encode_ttf_to_woff2(&asset.contents, &self.options)?;
        woff2.format = FontFormat::Woff2;
        woff2.rename_ext("woff2");
        woff2.meta.generated_by.push(self.name().into());

        if self.clone {
            Ok(vec![asset, woff2])
        } else {
            Ok(vec![woff2])
        }
    }
}
