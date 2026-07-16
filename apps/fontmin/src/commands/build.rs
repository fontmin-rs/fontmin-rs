use std::{
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use fontmin::{
    Asset, CoverageOptions, CssOptions, CssPlugin, CssTarget, FontDeliverySlice, FontFormat,
    MissingGlyphPolicy, OutputFormat, Svgs2TtfOptions, Svgs2TtfPlugin, UnicodeRange,
    validate_delivery_slices,
};
use fontmin_config::{
    CssConfig, CssTarget as ConfigCssTarget, DeliveryConfig, DiagnosticLevel, DiagnosticsConfig,
    FontminConfig, OtfConfig, OutputConfig, SubsetConfig,
};
use fontmin_fs::{expand_input_paths, path_to_string, resolve_path};
use fontmin_pipeline::Engine;
use miette::{Context, IntoDiagnostic, Result, miette};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{
    convert::parse_variations,
    coverage::{handle_missing_glyphs, parse_missing_glyph_policy},
    format::parse_output_formats,
    unicode::parse_optional_unicodes,
};
use crate::config::{find_config, load_config, resolve_plugin_text_files};

const CACHE_SCHEMA_VERSION: &str = "v1";
const FONTMIN_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct BuildOptions {
    pub inputs: Vec<PathBuf>,
    pub config: Option<PathBuf>,
    pub out_dir: Option<PathBuf>,
    pub text: Option<String>,
    pub text_file: Option<PathBuf>,
    pub unicodes: Option<String>,
    pub basic_text: bool,
    pub missing_glyphs: Option<String>,
    pub reporting: BuildReporting,
    pub cache_override: Option<bool>,
    pub css_glyph: bool,
    pub css_unicode_ranges: Vec<String>,
    pub delivery_slices: Vec<String>,
    pub variations: Vec<String>,
    pub formats: Option<String>,
    pub preset: Option<String>,
    pub no_original: bool,
    pub font_family: Option<String>,
    pub font_path: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum BuildReporting {
    Default,
    Silent,
    Timed,
}

impl BuildReporting {
    pub fn from_flags(show_time: bool, silent: bool) -> Self {
        if silent {
            Self::Silent
        } else if show_time {
            Self::Timed
        } else {
            Self::Default
        }
    }
}

pub fn cache_override_from_flags(cache: bool, no_cache: bool) -> Result<Option<bool>> {
    if cache && no_cache {
        return Err(miette!("build accepts only one of --cache or --no-cache"));
    }

    Ok(cache.then_some(true).or_else(|| no_cache.then_some(false)))
}

pub async fn run(mut options: BuildOptions) -> Result<i32> {
    let started_at = matches!(options.reporting, BuildReporting::Timed).then(Instant::now);
    let iconfont_requested = options.preset.as_deref().is_some_and(is_iconfont_preset);
    let config_path = match options.config.take() {
        Some(config_path) => Some(absolute_path(config_path)?),
        None if options.inputs.is_empty() => {
            find_config(&std::env::current_dir().into_diagnostic()?).await?
        }
        None => None,
    };

    if iconfont_requested {
        let config = match config_path {
            Some(config_path) => {
                let mut config = load_config(&config_path).await?;

                if config.cwd.is_none() {
                    config.cwd = Some(path_to_string(
                        config_path.parent().unwrap_or(Path::new(".")),
                    ));
                }

                apply_cli_overrides(&mut config, options)?;
                config
            }
            None => iconfont_config_from_cli(options)?,
        };

        run_iconfont_config(config).await?;
        report_show_time(started_at);
        return Ok(0);
    }

    let config = match config_path {
        Some(config_path) => {
            let mut config = load_config(&config_path).await?;

            if config.cwd.is_none() {
                config.cwd = Some(path_to_string(
                    config_path.parent().unwrap_or(Path::new(".")),
                ));
            }

            apply_cli_overrides(&mut config, options)?;
            config
        }
        None => config_from_cli(options)?,
    };

    run_config(config).await?;
    report_show_time(started_at);

    Ok(0)
}

fn report_show_time(started_at: Option<Instant>) {
    if let Some(started_at) = started_at {
        println!(
            "fontmin-rs build completed in {} ms",
            started_at.elapsed().as_millis()
        );
    }
}

fn iconfont_config_from_cli(options: BuildOptions) -> Result<FontminConfig> {
    if options.inputs.is_empty() {
        return Err(miette!("build requires at least one input"));
    }
    if options.formats.is_some() {
        return Err(miette!("build accepts only one of --formats or --preset"));
    }

    let Some(out_dir) = options.out_dir else {
        return Err(miette!("build requires -o, --out-dir"));
    };
    let unicode_ranges = parse_css_unicode_ranges(&options.css_unicode_ranges)?;
    let delivery_slices = parse_delivery_slices(&options.delivery_slices)?;
    let variation_coordinates = parse_variations(&options.variations)?;

    Ok(FontminConfig {
        input: paths_to_strings(options.inputs),
        out_dir: Some(path_to_string(&out_dir)),
        outputs: vec![
            OutputConfig::format(OutputFormat::Ttf),
            OutputConfig::format(OutputFormat::Css),
        ],
        css: Some(CssConfig {
            font_family: options.font_family,
            font_path: options.font_path.unwrap_or_else(|| "./".into()),
            glyph: true,
            as_file_name: Some(true),
            unicode_ranges,
            ..CssConfig::default()
        }),
        delivery: (!delivery_slices.is_empty()).then_some(DeliveryConfig {
            slices: delivery_slices,
        }),
        otf: OtfConfig {
            preserve_hinting: false,
            variation_coordinates,
        },
        cache: fontmin_config::CacheConfig {
            enabled: options.cache_override.unwrap_or(false),
            ..fontmin_config::CacheConfig::default()
        },
        diagnostics: diagnostics_for_reporting(options.reporting),
        ..FontminConfig::default()
    })
}

async fn run_iconfont_config(mut config: FontminConfig) -> Result<()> {
    if config.input.is_empty() {
        return Err(miette!("build requires at least one input"));
    }

    let cwd = config
        .cwd
        .as_deref()
        .map_or_else(std::env::current_dir, |cwd| Ok(PathBuf::from(cwd)))
        .into_diagnostic()?;
    let input_paths = expand_input_paths(&config.input, &cwd)?;
    let out_dir = resolve_path(&cwd, config.out_dir.as_deref().unwrap_or("build"));
    let cache = BuildCache::from_config(&config, &cwd);
    let css_config = config.css.take().unwrap_or_default();
    let font_family = css_config
        .font_family
        .clone()
        .unwrap_or_else(|| "iconfont".into());
    let mut assets = Vec::with_capacity(input_paths.len());

    if config.clean {
        remove_dir_if_exists(&out_dir).await?;
    }

    for input in &input_paths {
        let bytes = tokio::fs::read(&input)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to read {}", input.display()))?;
        let format = fontmin_detect::detect_format(&bytes);

        if format != FontFormat::Svg {
            return Err(miette!(
                "iconfont preset requires SVG icon inputs, got {}",
                input.display()
            ));
        }

        assets.push(Asset::new(file_name(input)?.into(), bytes, format));
    }

    let cache_key = cache
        .enabled
        .then(|| {
            cache_key_for_iconfont_inputs(
                &input_paths,
                &assets,
                &config.outputs,
                &css_config,
                &font_family,
            )
        })
        .transpose()?;

    if let Some(cache_key) = &cache_key
        && let Some(outputs) = read_cached_outputs(&cache.dir, cache_key).await?
    {
        write_build_outputs(&out_dir, &outputs).await?;
        return Ok(());
    }

    let mut assets = Engine::from_assets(assets)
        .plugin(Svgs2TtfPlugin {
            options: Svgs2TtfOptions {
                font_name: font_family.clone(),
                ..Svgs2TtfOptions::default()
            },
            clone: false,
        })
        .run()
        .await
        .into_diagnostic()?;

    let Some(ttf) = assets
        .iter_mut()
        .find(|asset| asset.format == FontFormat::Ttf)
    else {
        return Err(miette!("iconfont preset did not produce a TTF asset"));
    };
    ttf.path = "iconfont.ttf".into();
    apply_output_path(ttf, &config.outputs, OutputFormat::Ttf);

    let mut assets = Engine::from_assets(assets)
        .plugin(CssPlugin {
            options: CssOptions {
                font_family: font_family.clone(),
                font_path: css_config.font_path,
                base64: css_config.base64,
                glyph: true,
                icon_prefix: css_config.icon_prefix,
                as_file_name: css_config.as_file_name.unwrap_or(true),
                local: css_config.local,
                font_display: css_config.font_display,
                target: css_target_from_config(css_config.target),
                unicode_ranges: css_config.unicode_ranges,
            },
        })
        .run()
        .await
        .into_diagnostic()?;

    if let Some(css) = assets
        .iter_mut()
        .find(|asset| asset.format == FontFormat::Css)
    {
        apply_output_path(css, &config.outputs, OutputFormat::Css);
    }

    tokio::fs::create_dir_all(&out_dir)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create {}", out_dir.display()))?;

    let outputs = assets
        .into_iter()
        .map(BuildOutput::from_asset)
        .collect::<Vec<_>>();

    write_build_outputs(&out_dir, &outputs).await?;

    if let Some(cache_key) = &cache_key {
        write_cached_outputs(&cache.dir, cache_key, &outputs).await?;
    }

    Ok(())
}

fn apply_output_path(asset: &mut Asset, outputs: &[OutputConfig], format: OutputFormat) {
    let Some(output) = outputs.iter().find(|output| output.format == format) else {
        return;
    };

    if let Some(file_name) = &output.file_name {
        asset.path = file_name.into();
    } else if let Some(ext) = &output.ext {
        asset.rename_ext(ext);
    }
}

async fn run_config(mut config: FontminConfig) -> Result<()> {
    if config.input.is_empty() {
        return Err(miette!("build requires at least one input"));
    }

    let cwd = config
        .cwd
        .as_deref()
        .map_or_else(std::env::current_dir, |cwd| Ok(PathBuf::from(cwd)))
        .into_diagnostic()?;
    resolve_subset_text_file(&mut config, &cwd).await?;
    resolve_plugin_text_files(&mut config, &cwd).await?;

    let out_dir = resolve_path(&cwd, config.out_dir.as_deref().unwrap_or("build"));

    if config.clean {
        remove_dir_if_exists(&out_dir).await?;
    }

    tokio::fs::create_dir_all(&out_dir)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create {}", out_dir.display()))?;

    for input in expand_input_paths(&config.input, &cwd)? {
        build_input(&input, &out_dir, &cwd, config.clone()).await?;
    }

    Ok(())
}

fn config_from_cli(options: BuildOptions) -> Result<FontminConfig> {
    if options.inputs.is_empty() {
        return Err(miette!("build requires at least one input"));
    }

    let Some(out_dir) = options.out_dir else {
        return Err(miette!("build requires -o, --out-dir"));
    };
    let formats = output_formats_from_cli(options.formats, options.preset, options.no_original)?
        .ok_or_else(|| miette!("build requires --formats or --preset"))?;
    ensure_output_formats(&formats)?;
    let unicodes = parse_optional_unicodes(options.unicodes.as_deref())?;
    let unicode_ranges = parse_css_unicode_ranges(&options.css_unicode_ranges)?;
    let delivery_slices = parse_delivery_slices(&options.delivery_slices)?;
    let variation_coordinates = parse_variations(&options.variations)?;
    let missing_glyphs =
        parse_missing_glyph_policy(options.missing_glyphs.as_deref())?.unwrap_or_default();

    Ok(FontminConfig {
        input: paths_to_strings(options.inputs),
        out_dir: Some(path_to_string(&out_dir)),
        subset: (options.text.is_some()
            || options.text_file.is_some()
            || !unicodes.is_empty()
            || options.basic_text)
            .then_some(SubsetConfig {
                text: options.text,
                text_file: options.text_file.as_ref().map(|path| path_to_string(path)),
                unicodes,
                basic_text: options.basic_text,
                missing_glyphs,
                ..SubsetConfig::default()
            }),
        outputs: formats.iter().copied().map(OutputConfig::format).collect(),
        css: formats.contains(&OutputFormat::Css).then_some(CssConfig {
            font_family: options.font_family,
            font_path: options.font_path.unwrap_or_else(|| "./".into()),
            glyph: options.css_glyph,
            local: true,
            font_display: "swap".into(),
            unicode_ranges,
            ..CssConfig::default()
        }),
        delivery: (!delivery_slices.is_empty()).then_some(DeliveryConfig {
            slices: delivery_slices,
        }),
        otf: OtfConfig {
            preserve_hinting: false,
            variation_coordinates,
        },
        preserve_original: !options.no_original,
        cache: fontmin_config::CacheConfig {
            enabled: options.cache_override.unwrap_or(false),
            ..fontmin_config::CacheConfig::default()
        },
        diagnostics: diagnostics_for_reporting(options.reporting),
        ..FontminConfig::default()
    })
}

fn apply_cli_overrides(config: &mut FontminConfig, options: BuildOptions) -> Result<()> {
    let unicodes = parse_optional_unicodes(options.unicodes.as_deref())?;
    let unicode_ranges = parse_css_unicode_ranges(&options.css_unicode_ranges)?;
    let delivery_slices = parse_delivery_slices(&options.delivery_slices)?;
    let variation_coordinates = parse_variations(&options.variations)?;
    let missing_glyphs = parse_missing_glyph_policy(options.missing_glyphs.as_deref())?;

    if matches!(options.reporting, BuildReporting::Silent) {
        config.diagnostics.level = DiagnosticLevel::Silent;
    }

    if !options.inputs.is_empty() {
        config.input = paths_to_strings(options.inputs);
    }

    if let Some(out_dir) = options.out_dir {
        config.out_dir = Some(path_to_string(&out_dir));
    }

    if options.formats.is_some() || options.preset.is_some() || options.no_original {
        let formats = if options.formats.is_none() && options.preset.is_none() {
            config
                .outputs
                .iter()
                .map(|output| output.format)
                .collect::<Vec<_>>()
        } else {
            output_formats_from_cli(options.formats, options.preset, false)?
                .ok_or_else(|| miette!("build requires --formats or --preset"))?
        };

        let formats = filter_original_output(formats, options.no_original);
        ensure_output_formats(&formats)?;

        config.outputs = output_configs_for_formats(formats, &config.outputs);
    }

    if options.no_original {
        config.preserve_original = false;
    }

    if !delivery_slices.is_empty() {
        config.delivery = Some(DeliveryConfig {
            slices: delivery_slices,
        });
    }

    config
        .otf
        .variation_coordinates
        .extend(variation_coordinates);

    if let Some(enabled) = options.cache_override {
        config.cache.enabled = enabled;
    }

    if options.text.is_some()
        || options.text_file.is_some()
        || !unicodes.is_empty()
        || options.basic_text
    {
        let subset = config.subset.get_or_insert_with(SubsetConfig::default);

        if let Some(text) = options.text {
            subset.text = Some(text);
        }
        if let Some(text_file) = options.text_file {
            subset.text_file = Some(path_to_string(&text_file));
        }
        if !unicodes.is_empty() {
            subset.unicodes = unicodes;
        }
        if options.basic_text {
            subset.basic_text = true;
        }
    }

    if let Some(missing_glyphs) = missing_glyphs {
        config
            .subset
            .get_or_insert_with(SubsetConfig::default)
            .missing_glyphs = missing_glyphs;
    }

    if options.font_family.is_some()
        || options.font_path.is_some()
        || options.css_glyph
        || !unicode_ranges.is_empty()
    {
        let css = config.css.get_or_insert_with(CssConfig::default);

        if let Some(font_family) = options.font_family {
            css.font_family = Some(font_family);
        }
        if let Some(font_path) = options.font_path {
            css.font_path = font_path;
        }
        if options.css_glyph {
            css.glyph = true;
        }
        if !unicode_ranges.is_empty() {
            css.unicode_ranges = unicode_ranges;
        }
    }

    Ok(())
}

fn parse_css_unicode_ranges(values: &[String]) -> Result<Vec<UnicodeRange>> {
    values
        .iter()
        .map(|value| {
            value
                .parse::<UnicodeRange>()
                .map_err(|error| miette!(error))
        })
        .collect()
}

fn parse_delivery_slices(values: &[String]) -> Result<Vec<FontDeliverySlice>> {
    let mut slices = Vec::<FontDeliverySlice>::new();

    for value in values {
        let (name, ranges) = value
            .split_once(':')
            .ok_or_else(|| miette!("delivery slice must use NAME:RANGE[,RANGE...]: {value}"))?;
        let unicode_ranges = ranges
            .split(',')
            .map(|range| {
                range
                    .parse::<UnicodeRange>()
                    .map_err(|error| miette!(error))
            })
            .collect::<Result<Vec<_>>>()?;

        if let Some(slice) = slices.iter_mut().find(|slice| slice.name == name) {
            slice.unicode_ranges.extend(unicode_ranges);
        } else {
            slices.push(FontDeliverySlice {
                name: name.into(),
                unicode_ranges,
            });
        }
    }

    validate_delivery_slices(&slices).map_err(|error| miette!(error))?;

    Ok(slices)
}

fn output_formats_from_cli(
    formats: Option<String>,
    preset: Option<String>,
    no_original: bool,
) -> Result<Option<Vec<OutputFormat>>> {
    let formats = match (formats, preset) {
        (Some(_), Some(_)) => Err(miette!("build accepts only one of --formats or --preset")),
        (Some(formats), None) => parse_output_formats(&formats).map(Some),
        (None, Some(preset)) => preset_output_formats(&preset).map(Some),
        (None, None) => Ok(None),
    }?;

    Ok(formats.map(|formats| filter_original_output(formats, no_original)))
}

fn filter_original_output(mut formats: Vec<OutputFormat>, no_original: bool) -> Vec<OutputFormat> {
    if no_original {
        formats.retain(|format| *format != OutputFormat::Ttf);
    }

    formats
}

fn output_configs_for_formats(
    formats: Vec<OutputFormat>,
    outputs: &[OutputConfig],
) -> Vec<OutputConfig> {
    formats
        .into_iter()
        .map(|format| {
            outputs
                .iter()
                .find(|output| output.format == format)
                .cloned()
                .unwrap_or_else(|| OutputConfig::format(format))
        })
        .collect()
}

fn ensure_output_formats(formats: &[OutputFormat]) -> Result<()> {
    if formats.is_empty() {
        return Err(miette!(
            "build requires at least one non-original output format"
        ));
    }

    Ok(())
}

fn preset_output_formats(preset: &str) -> Result<Vec<OutputFormat>> {
    match preset.trim().to_ascii_lowercase().as_str() {
        "compat" => Ok(vec![
            OutputFormat::Eot,
            OutputFormat::Svg,
            OutputFormat::Woff,
            OutputFormat::Woff2,
            OutputFormat::Css,
        ]),
        "modern-web" => Ok(vec![
            OutputFormat::Woff2,
            OutputFormat::Woff,
            OutputFormat::Css,
        ]),
        "iconfont" => Ok(vec![OutputFormat::Ttf, OutputFormat::Css]),
        preset => Err(miette!("unsupported preset `{preset}`")),
    }
}

fn is_iconfont_preset(preset: &str) -> bool {
    preset.trim().eq_ignore_ascii_case("iconfont")
}

fn css_target_from_config(target: ConfigCssTarget) -> CssTarget {
    match target {
        ConfigCssTarget::Css => CssTarget::Css,
        ConfigCssTarget::Scss => CssTarget::Scss,
        ConfigCssTarget::Less => CssTarget::Less,
    }
}

async fn build_input(
    input: &Path,
    out_dir: &Path,
    cwd: &Path,
    mut config: FontminConfig,
) -> Result<()> {
    let bytes = tokio::fs::read(input)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", input.display()))?;

    check_configured_coverage(&bytes, &config)?;

    if let Some(css) = &mut config.css
        && css.font_family.is_none()
    {
        css.font_family = Some(file_stem(input)?);
    }

    let cache = BuildCache::from_config(&config, cwd);
    let cache_key = cache
        .enabled
        .then(|| cache_key_for_input(input, &bytes, &config))
        .transpose()?;

    if let Some(cache_key) = &cache_key
        && let Some(outputs) = read_cached_outputs(&cache.dir, cache_key).await?
    {
        write_build_outputs(out_dir, &outputs).await?;
        return Ok(());
    }

    let format = fontmin_detect::detect_format(&bytes);
    let asset = Asset::new(file_name(input)?.into(), bytes, format);
    let assets = Engine::try_new(config)
        .into_diagnostic()?
        .with_assets(vec![asset])
        .run()
        .await
        .into_diagnostic()?;

    if assets.is_empty() {
        return Err(miette!(
            "build did not produce assets for {}",
            input.display()
        ));
    }

    let outputs = assets
        .into_iter()
        .map(BuildOutput::from_asset)
        .collect::<Vec<_>>();

    write_build_outputs(out_dir, &outputs).await?;

    if let Some(cache_key) = &cache_key {
        write_cached_outputs(&cache.dir, cache_key, &outputs).await?;
    }

    Ok(())
}

fn check_configured_coverage(bytes: &[u8], config: &FontminConfig) -> Result<()> {
    let Some(subset) = config.subset.as_ref() else {
        return Ok(());
    };

    if subset.missing_glyphs == MissingGlyphPolicy::Ignore {
        return Ok(());
    }

    let report = fontmin::analyze_coverage(
        bytes,
        CoverageOptions {
            text: subset.text.clone(),
            unicodes: subset.unicodes.clone(),
            basic_text: subset.basic_text,
            ..CoverageOptions::default()
        },
    )
    .into_diagnostic()?;
    let emit_warning = matches!(
        config.diagnostics.level,
        DiagnosticLevel::Warn | DiagnosticLevel::Info
    );

    handle_missing_glyphs(
        &report,
        subset.missing_glyphs,
        emit_warning,
        config.diagnostics.fail_on_warning,
    )
}

fn diagnostics_for_reporting(reporting: BuildReporting) -> DiagnosticsConfig {
    DiagnosticsConfig {
        level: if matches!(reporting, BuildReporting::Silent) {
            DiagnosticLevel::Silent
        } else {
            DiagnosticLevel::Warn
        },
        ..DiagnosticsConfig::default()
    }
}

struct BuildCache {
    enabled: bool,
    dir: PathBuf,
}

impl BuildCache {
    fn from_config(config: &FontminConfig, cwd: &Path) -> Self {
        Self {
            enabled: config.cache.enabled,
            dir: resolve_path(cwd, &config.cache.dir),
        }
    }
}

struct BuildOutput {
    file_name: PathBuf,
    contents: Vec<u8>,
}

impl BuildOutput {
    fn from_asset(asset: Asset) -> Self {
        Self {
            file_name: asset.path,
            contents: asset.contents,
        }
    }
}

async fn write_build_outputs(out_dir: &Path, outputs: &[BuildOutput]) -> Result<()> {
    tokio::fs::create_dir_all(out_dir)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create {}", out_dir.display()))?;

    for output in outputs {
        let output_path = out_dir.join(&output.file_name);

        tokio::fs::write(&output_path, &output.contents)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to write {}", output_path.display()))?;
    }

    Ok(())
}

async fn read_cached_outputs(cache_dir: &Path, key: &str) -> Result<Option<Vec<BuildOutput>>> {
    let manifest_path = cache_manifest_path(cache_dir, key);
    let manifest = match tokio::fs::read_to_string(&manifest_path).await {
        Ok(manifest) => manifest,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to read {}", manifest_path.display()));
        }
    };
    let Ok(manifest) = serde_json::from_str::<Value>(&manifest) else {
        return Ok(None);
    };

    if manifest["version"] != CACHE_SCHEMA_VERSION || manifest["key"] != key {
        return Ok(None);
    }

    let Some(records) = manifest["outputs"].as_array() else {
        return Ok(None);
    };
    let entry_dir = cache_entry_dir(cache_dir, key);
    let mut outputs = Vec::with_capacity(records.len());

    for record in records {
        let (Some(file_name), Some(cache_file_name)) = (
            record["fileName"].as_str(),
            record["cacheFileName"].as_str(),
        ) else {
            return Ok(None);
        };
        let cache_file = entry_dir.join(cache_file_name);
        let contents = match tokio::fs::read(&cache_file).await {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error)
                    .into_diagnostic()
                    .wrap_err_with(|| format!("failed to read {}", cache_file.display()));
            }
        };

        outputs.push(BuildOutput {
            file_name: PathBuf::from(file_name),
            contents,
        });
    }

    Ok(Some(outputs))
}

async fn write_cached_outputs(cache_dir: &Path, key: &str, outputs: &[BuildOutput]) -> Result<()> {
    let entry_dir = cache_entry_dir(cache_dir, key);
    let mut records = Vec::with_capacity(outputs.len());

    tokio::fs::create_dir_all(&entry_dir)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create {}", entry_dir.display()))?;

    for (index, output) in outputs.iter().enumerate() {
        let extension = output
            .file_name
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("bin");
        let cache_file_name = format!("{index:03}.{extension}");
        let cache_file = entry_dir.join(&cache_file_name);

        tokio::fs::write(&cache_file, &output.contents)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to write {}", cache_file.display()))?;
        records.push(json!({
            "cacheFileName": cache_file_name,
            "fileName": path_to_string(&output.file_name),
        }));
    }

    let manifest = json!({
        "key": key,
        "outputs": records,
        "version": CACHE_SCHEMA_VERSION,
    });
    let manifest_path = cache_manifest_path(cache_dir, key);

    tokio::fs::write(
        &manifest_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&manifest).into_diagnostic()?
        ),
    )
    .await
    .into_diagnostic()
    .wrap_err_with(|| format!("failed to write {}", manifest_path.display()))?;
    update_cache_index(cache_dir, key, outputs).await
}

async fn update_cache_index(cache_dir: &Path, key: &str, outputs: &[BuildOutput]) -> Result<()> {
    let index_path = cache_index_path(cache_dir);
    let mut index = match tokio::fs::read_to_string(&index_path).await {
        Ok(index) => serde_json::from_str::<Value>(&index).unwrap_or_else(|_| empty_cache_index()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => empty_cache_index(),
        Err(error) => {
            return Err(error)
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to read {}", index_path.display()));
        }
    };

    if index["version"] != CACHE_SCHEMA_VERSION || !index["entries"].is_object() {
        index = empty_cache_index();
    }

    if let Some(entries) = index["entries"].as_object_mut() {
        entries.insert(
            key.into(),
            json!({
                "outputs": outputs
                    .iter()
                    .map(|output| path_to_string(&output.file_name))
                    .collect::<Vec<_>>(),
                "updatedAt": cache_timestamp(),
            }),
        );
    }

    let Some(root) = index_path.parent() else {
        return Err(miette!(
            "failed to determine cache root for {}",
            index_path.display()
        ));
    };

    tokio::fs::create_dir_all(root)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create {}", root.display()))?;
    tokio::fs::write(
        &index_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&index).into_diagnostic()?
        ),
    )
    .await
    .into_diagnostic()
    .wrap_err_with(|| format!("failed to write {}", index_path.display()))
}

fn empty_cache_index() -> Value {
    json!({
        "entries": {},
        "version": CACHE_SCHEMA_VERSION,
    })
}

fn cache_key_for_input(input: &Path, contents: &[u8], config: &FontminConfig) -> Result<String> {
    let key = json!({
        "config": {
            "css": config.css,
            "outputs": config.outputs,
            "preserveOriginal": config.preserve_original,
            "subset": config.subset,
        },
        "fontminVersion": FONTMIN_VERSION,
        "input": {
            "hash": sha256(contents),
            "path": path_to_string(input),
        },
        "schema": CACHE_SCHEMA_VERSION,
    });
    let key = serde_json::to_vec(&key).into_diagnostic()?;

    Ok(sha256(&key))
}

fn cache_key_for_iconfont_inputs(
    input_paths: &[PathBuf],
    assets: &[Asset],
    outputs: &[OutputConfig],
    css: &CssConfig,
    font_family: &str,
) -> Result<String> {
    let key = json!({
        "fontminVersion": FONTMIN_VERSION,
        "icons": assets
            .iter()
            .zip(input_paths)
            .map(|(asset, input)| {
                json!({
                    "hash": sha256(&asset.contents),
                    "input": path_to_string(input),
                    "path": path_to_string(&asset.path),
                })
            })
            .collect::<Vec<_>>(),
        "kind": "iconfont",
        "options": {
            "css": css,
            "fontFamily": font_family,
            "outputs": outputs,
        },
        "schema": CACHE_SCHEMA_VERSION,
    });
    let key = serde_json::to_vec(&key).into_diagnostic()?;

    Ok(sha256(&key))
}

fn cache_root(cache_dir: &Path) -> PathBuf {
    cache_dir.join(CACHE_SCHEMA_VERSION)
}

fn cache_index_path(cache_dir: &Path) -> PathBuf {
    cache_root(cache_dir).join("index.json")
}

fn cache_entry_dir(cache_dir: &Path, key: &str) -> PathBuf {
    cache_root(cache_dir).join(key)
}

fn cache_manifest_path(cache_dir: &Path, key: &str) -> PathBuf {
    cache_entry_dir(cache_dir, key).join("index.json")
}

fn cache_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
        .to_string()
}

fn sha256(input: impl AsRef<[u8]>) -> String {
    let digest = Sha256::digest(input);
    let mut hash = String::with_capacity(digest.len() * 2);

    for byte in digest {
        use std::fmt::Write as _;

        write!(&mut hash, "{byte:02x}").expect("writing a hash to a string cannot fail");
    }

    hash
}

async fn resolve_subset_text_file(config: &mut FontminConfig, cwd: &Path) -> Result<()> {
    let Some(subset) = &mut config.subset else {
        return Ok(());
    };
    let Some(text_file) = subset.text_file.as_deref() else {
        return Ok(());
    };

    let text_path = resolve_path(cwd, text_file);
    let file_text = tokio::fs::read_to_string(&text_path)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", text_path.display()))?;

    subset.text = Some(match subset.text.take() {
        Some(text) => format!("{text}{file_text}"),
        None => file_text,
    });

    Ok(())
}

async fn remove_dir_if_exists(path: &Path) -> Result<()> {
    match tokio::fs::remove_dir_all(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to clean {}", path.display())),
    }
}

fn absolute_path(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }

    Ok(std::env::current_dir().into_diagnostic()?.join(path))
}

fn paths_to_strings(paths: Vec<PathBuf>) -> Vec<String> {
    paths
        .into_iter()
        .map(|path| path_to_string(&path))
        .collect()
}

fn file_name(path: &Path) -> Result<String> {
    let file_name = path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .ok_or_else(|| miette!("failed to determine file name for {}", path.display()))?;

    Ok(file_name.into())
}

fn file_stem(path: &Path) -> Result<String> {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| miette!("failed to determine file name for {}", path.display()))?;

    Ok(stem.into())
}
