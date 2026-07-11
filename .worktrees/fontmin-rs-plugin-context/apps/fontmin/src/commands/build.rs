use std::path::{Path, PathBuf};

use fontmin::{Asset, OutputFormat};
use fontmin_config::{CssConfig, FontminConfig, OutputConfig, SubsetConfig};
use fontmin_pipeline::Engine;
use jsonc_parser::ParseOptions;
use miette::{Context, IntoDiagnostic, Result, miette};

use super::format::parse_output_formats;

const DEFAULT_CONFIG_FILES: &[&str] = &["fontmin.config.json", "fontmin.config.jsonc"];

pub struct BuildOptions {
    pub inputs: Vec<PathBuf>,
    pub config: Option<PathBuf>,
    pub out_dir: Option<PathBuf>,
    pub text: Option<String>,
    pub basic_text: bool,
    pub formats: Option<String>,
    pub font_family: Option<String>,
    pub font_path: Option<String>,
}

pub async fn run(mut options: BuildOptions) -> Result<i32> {
    let config_path = match options.config.take() {
        Some(config_path) => Some(absolute_path(config_path)?),
        None if options.inputs.is_empty() => {
            find_config(&std::env::current_dir().into_diagnostic()?).await?
        }
        None => None,
    };

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

    Ok(0)
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

    let out_dir = resolve_path(&cwd, config.out_dir.as_deref().unwrap_or("build"));

    if config.clean {
        remove_dir_if_exists(&out_dir).await?;
    }

    tokio::fs::create_dir_all(&out_dir)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create {}", out_dir.display()))?;

    for input in expand_input_paths(&config.input, &cwd)? {
        build_input(&input, &out_dir, config.clone()).await?;
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
    let Some(formats) = options.formats else {
        return Err(miette!("build requires --formats"));
    };
    let formats = parse_output_formats(&formats)?;

    Ok(FontminConfig {
        input: paths_to_strings(options.inputs),
        out_dir: Some(path_to_string(&out_dir)),
        subset: (options.text.is_some() || options.basic_text).then_some(SubsetConfig {
            text: options.text,
            basic_text: options.basic_text,
            ..SubsetConfig::default()
        }),
        outputs: formats.iter().copied().map(OutputConfig::format).collect(),
        css: formats.contains(&OutputFormat::Css).then_some(CssConfig {
            font_family: options.font_family,
            font_path: options.font_path.unwrap_or_else(|| "./".into()),
            local: true,
            font_display: "swap".into(),
            ..CssConfig::default()
        }),
        ..FontminConfig::default()
    })
}

fn apply_cli_overrides(config: &mut FontminConfig, options: BuildOptions) -> Result<()> {
    if !options.inputs.is_empty() {
        config.input = paths_to_strings(options.inputs);
    }

    if let Some(out_dir) = options.out_dir {
        config.out_dir = Some(path_to_string(&out_dir));
    }

    if let Some(formats) = options.formats {
        config.outputs = parse_output_formats(&formats)?
            .into_iter()
            .map(OutputConfig::format)
            .collect();
    }

    if options.text.is_some() || options.basic_text {
        let subset = config.subset.get_or_insert_with(SubsetConfig::default);

        if let Some(text) = options.text {
            subset.text = Some(text);
        }
        if options.basic_text {
            subset.basic_text = true;
        }
    }

    if options.font_family.is_some() || options.font_path.is_some() {
        let css = config.css.get_or_insert_with(CssConfig::default);

        if let Some(font_family) = options.font_family {
            css.font_family = Some(font_family);
        }
        if let Some(font_path) = options.font_path {
            css.font_path = font_path;
        }
    }

    Ok(())
}

async fn build_input(input: &Path, out_dir: &Path, mut config: FontminConfig) -> Result<()> {
    let bytes = tokio::fs::read(input)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", input.display()))?;

    if let Some(css) = &mut config.css
        && css.font_family.is_none()
    {
        css.font_family = Some(file_stem(input)?);
    }

    let format = fontmin_detect::detect_format(&bytes);
    let asset = Asset::new(file_name(input)?.into(), bytes, format);
    let assets = Engine::new(config)
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

    for asset in assets {
        let output_path = out_dir.join(&asset.path);

        tokio::fs::write(&output_path, asset.contents)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to write {}", output_path.display()))?;
    }

    Ok(())
}

async fn load_config(path: &Path) -> Result<FontminConfig> {
    let contents = tokio::fs::read_to_string(path)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", path.display()))?;

    match path.extension().and_then(|extension| extension.to_str()) {
        Some("json") => serde_json::from_str(&contents).into_diagnostic(),
        Some("jsonc") => jsonc_parser::parse_to_serde_value(&contents, &ParseOptions::default())
            .into_diagnostic(),
        Some(extension) => Err(miette!("unsupported config extension `.{extension}`")),
        None => Err(miette!("config file requires an extension")),
    }
    .wrap_err_with(|| format!("failed to parse {}", path.display()))
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

fn expand_input_paths(inputs: &[String], cwd: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    for input in inputs {
        paths.extend(expand_input_path(input, cwd)?);
    }

    Ok(paths)
}

fn expand_input_path(input: &str, cwd: &Path) -> Result<Vec<PathBuf>> {
    if !is_glob_pattern(input) {
        return Ok(vec![resolve_path(cwd, input)]);
    }

    let pattern = path_to_string(&resolve_path(cwd, input));
    let mut paths = Vec::new();

    for entry in glob::glob(&pattern).into_diagnostic()? {
        let path = entry.into_diagnostic()?;

        if path.is_file() {
            paths.push(path);
        }
    }

    if paths.is_empty() {
        return Err(miette!("input glob matched no files: {input}"));
    }

    paths.sort();

    Ok(paths)
}

fn is_glob_pattern(path: &str) -> bool {
    path.chars()
        .any(|character| matches!(character, '*' | '?' | '[' | ']' | '{' | '}'))
}

async fn find_config(cwd: &Path) -> Result<Option<PathBuf>> {
    for file_name in DEFAULT_CONFIG_FILES {
        let config_path = cwd.join(file_name);

        if is_file(&config_path).await {
            return Ok(Some(config_path));
        }
    }

    Ok(None)
}

async fn is_file(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .is_ok_and(|metadata| metadata.is_file())
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

fn resolve_path(cwd: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);

    if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }
}

fn paths_to_strings(paths: Vec<PathBuf>) -> Vec<String> {
    paths
        .into_iter()
        .map(|path| path_to_string(&path))
        .collect()
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
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
