use std::path::{Path, PathBuf};

use fontmin_config::FontminConfig;
use fontmin_fs::resolve_path;
use jsonc_parser::ParseOptions;
use miette::{Context, IntoDiagnostic, Result, miette};
use tokio::{io::AsyncReadExt, process::Command};

const DEFAULT_CONFIG_FILES: &[&str] = &[
    "fontmin.config.ts",
    "fontmin.config.mts",
    "fontmin.config.mjs",
    "fontmin.config.cjs",
    "fontmin.config.json",
    "fontmin.config.jsonc",
];
const STDERR_LIMIT: usize = 64 * 1024;
const MODULE_CONFIG_NODE_ERROR: &str =
    "module config requires Node.js 22 or newer; install Node.js or use JSON/JSONC";
const MODULE_CONFIG_BRIDGE: &str = r"
import { Console } from 'node:console'
import { pathToFileURL } from 'node:url'

const nodeMajor = Number.parseInt(process.versions.node.split('.')[0], 10)
if (nodeMajor < 22) {
  throw new Error('module config requires Node.js 22 or newer')
}

globalThis.console = new Console({
  stdout: process.stderr,
  stderr: process.stderr,
  colorMode: false,
})

const pluginNames = new Map([
  ['glyph', 'fontmin:glyph'],
  ['unicodeSlices', 'fontmin:unicode-slices'],
  ['otf2ttf', 'fontmin:otf2ttf'],
  ['ttf2woff', 'fontmin:ttf2woff'],
  ['ttf2woff2', 'fontmin:ttf2woff2'],
  ['ttf2eot', 'fontmin:ttf2eot'],
  ['ttf2svg', 'fontmin:ttf2svg'],
  ['svg2ttf', 'fontmin:svg2ttf'],
  ['svgs2ttf', 'fontmin:svgs2ttf'],
  ['css', 'fontmin:css'],
])

const optionKeys = new Map([
  ['glyph', new Set(['text', 'textFile', 'unicodes', 'unicodeRanges', 'basicText', 'hinting', 'trim', 'keepNotdef', 'keepLayout', 'clone', 'preserveHinting'])],
  ['unicodeSlices', new Set(['slices'])],
  ['otf2ttf', new Set(['clone', 'preserveHinting', 'variationCoordinates'])],
  ['ttf2woff', new Set(['clone', 'deflate', 'compressionLevel', 'metadata'])],
  ['ttf2woff2', new Set(['clone', 'quality'])],
  ['ttf2eot', new Set(['clone', 'version'])],
  ['ttf2svg', new Set(['clone', 'fontFamily'])],
  ['svg2ttf', new Set(['clone', 'hinting', 'normalize'])],
  ['svgs2ttf', new Set(['clone', 'fontName', 'startUnicode', 'ascent', 'descent', 'normalize'])],
  ['css', new Set(['fontPath', 'base64', 'glyph', 'iconPrefix', 'fontFamily', 'asFileName', 'local', 'fontDisplay', 'target', 'unicodeRanges'])],
])

function fieldPath(parent, key, arrayIndex = false) {
  if (arrayIndex) return `${parent}[${key}]`
  return parent === '' ? key : `${parent}.${key}`
}

function normalize(value, path, seen, inArray = false) {
  if (value === null || typeof value === 'string' || typeof value === 'boolean') return value
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) throw new Error(`${path || 'config'} must contain finite numbers`)
    return value
  }
  if (value === undefined) {
    if (inArray) throw new Error(`${path} must not be undefined`)
    return undefined
  }
  if (typeof value === 'function' || typeof value === 'symbol' || typeof value === 'bigint') {
    throw new Error(`${path || 'config'} is not serializable (${typeof value})`)
  }
  if (typeof value !== 'object') throw new Error(`${path || 'config'} is not serializable`)
  if (seen.has(value)) throw new Error(`${path || 'config'} contains a cycle`)
  seen.add(value)
  try {
    if (Array.isArray(value)) {
      return value.map((entry, index) => normalize(entry, fieldPath(path, index, true), seen, true))
    }
    const prototype = Object.getPrototypeOf(value)
    if (prototype !== Object.prototype && prototype !== null) {
      throw new Error(`${path || 'config'} must be a plain object`)
    }
    const result = {}
    for (const [key, entry] of Object.entries(value)) {
      const childPath = fieldPath(path, key)
      const normalized = normalize(entry, childPath, seen)
      if (normalized !== undefined) result[key] = normalized
    }
    return result
  } finally {
    seen.delete(value)
  }
}

function validatePlugins(config) {
  if (config.css && typeof config.css.fontFamily === 'function') {
    throw new Error('css.fontFamily is not serializable (function)')
  }
  for (const [index, plugin] of (config.plugins ?? []).entries()) {
    const path = `plugins[${index}]`
    if (!plugin.native || plugin.native.kind !== 'builtin') {
      throw new Error(`${path} must be a serializable built-in plugin`)
    }
    const expectedName = pluginNames.get(plugin.native.name)
    if (expectedName === undefined || plugin.name !== expectedName) {
      throw new Error(`${path} is an unknown built-in plugin`)
    }
    const allowed = optionKeys.get(plugin.native.name)
    for (const key of Object.keys(plugin.native.options ?? {})) {
      if (!allowed.has(key)) throw new Error(`${path}.native.options.${key} is unsupported by the Rust CLI`)
    }
  }
}

const configPath = process.argv[1]
const module = await import(pathToFileURL(configPath).href)
const exported = module.default ?? module.config
if (exported === undefined) throw new Error('does not export default or config')
const config = typeof exported === 'function' ? await exported() : exported
const normalized = normalize(config, '', new WeakSet())
validatePlugins(normalized)
if (normalized.plugins !== undefined) {
  if (normalized.outputs === undefined) normalized.outputs = []
  if (normalized.css === undefined) normalized.css = null
}
process.stdout.write(JSON.stringify(normalized))
";

pub async fn find_config(cwd: &Path) -> Result<Option<PathBuf>> {
    for file_name in DEFAULT_CONFIG_FILES {
        let config_path = cwd.join(file_name);

        if is_file(&config_path).await {
            return Ok(Some(config_path));
        }
    }

    Ok(None)
}

pub async fn load_config(path: &Path) -> Result<FontminConfig> {
    let result = match path.extension().and_then(|extension| extension.to_str()) {
        Some("json" | "jsonc") => load_json_config(path).await,
        Some("ts" | "mts" | "mjs" | "cjs") => load_module_config(path).await,
        Some(extension) => Err(miette!("unsupported config extension `.{extension}`")),
        None => Err(miette!("config file requires an extension")),
    };

    result.map_err(|error| miette!("failed to parse {}: {error}", path.display()))
}

async fn load_json_config(path: &Path) -> Result<FontminConfig> {
    let contents = tokio::fs::read_to_string(path)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", path.display()))?;

    match path.extension().and_then(|extension| extension.to_str()) {
        Some("json") => serde_json::from_str(&contents).into_diagnostic(),
        Some("jsonc") => jsonc_parser::parse_to_serde_value(&contents, &ParseOptions::default())
            .into_diagnostic(),
        _ => unreachable!("JSON loader called only for JSON or JSONC"),
    }
}

async fn load_module_config(path: &Path) -> Result<FontminConfig> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().into_diagnostic()?.join(path)
    };
    let mut child = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(MODULE_CONFIG_BRIDGE)
        .arg(&path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|_| miette!(MODULE_CONFIG_NODE_ERROR))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| miette!("failed to capture module config bridge stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| miette!("failed to capture module config bridge stderr"))?;
    let mut stdout_bytes = Vec::new();
    let (status, stdout_result, stderr_result) = tokio::join!(
        child.wait(),
        stdout.read_to_end(&mut stdout_bytes),
        read_bounded_stderr(stderr),
    );
    let status = status.into_diagnostic()?;
    stdout_result.into_diagnostic()?;
    let stderr_bytes = stderr_result?;
    let stderr = String::from_utf8_lossy(&stderr_bytes);

    if !status.success() {
        let details = stderr.trim();
        return if details.is_empty() {
            Err(miette!("module config bridge exited with {status}"))
        } else {
            Err(miette!(
                "module config bridge exited with {status}: {details}"
            ))
        };
    }
    if stdout_bytes.is_empty() {
        return Err(miette!("module config bridge returned empty stdout"));
    }

    serde_json::from_slice(&stdout_bytes)
        .into_diagnostic()
        .wrap_err("module config bridge returned invalid JSON")
}

async fn read_bounded_stderr(mut stderr: tokio::process::ChildStderr) -> Result<Vec<u8>> {
    let mut retained = Vec::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = stderr.read(&mut buffer).await.into_diagnostic()?;
        if read == 0 {
            break;
        }
        let remaining = STDERR_LIMIT.saturating_sub(retained.len());
        retained.extend_from_slice(&buffer[..read.min(remaining)]);
    }

    Ok(retained)
}

pub async fn resolve_plugin_text_files(config: &mut FontminConfig, cwd: &Path) -> Result<()> {
    for (index, plugin) in config.plugins.iter_mut().enumerate() {
        if plugin.native.name != "glyph" {
            continue;
        }

        let Some(options) = plugin.native.options.as_object_mut() else {
            continue;
        };
        let Some(text_file) = options.get("textFile").and_then(|value| value.as_str()) else {
            continue;
        };
        let text_path = resolve_path(cwd, text_file);
        let file_text = tokio::fs::read_to_string(&text_path)
            .await
            .into_diagnostic()
            .wrap_err_with(|| {
                format!(
                    "failed to read plugins[{index}].native.options.textFile {}",
                    text_path.display()
                )
            })?;

        options.remove("textFile");
        match options.get_mut("text") {
            Some(text) if text.is_string() => {
                let text = text.as_str().expect("checked as a string");
                *options.get_mut("text").expect("text option exists") =
                    serde_json::Value::String(format!("{text}{file_text}"));
            }
            Some(_) => {}
            None => {
                options.insert("text".into(), serde_json::Value::String(file_text));
            }
        }
    }

    Ok(())
}

async fn is_file(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .is_ok_and(|metadata| metadata.is_file())
}

#[cfg(test)]
mod tests {
    use std::{env, path::Path};

    use fontmin_pipeline::Engine;
    use serde_json::json;
    use tokio::sync::Mutex;

    use super::{find_config, load_config, resolve_plugin_text_files};

    static ENV_LOCK: Mutex<()> = Mutex::const_new(());

    async fn load(path: &Path) -> miette::Result<fontmin_config::FontminConfig> {
        let guard = ENV_LOCK.lock().await;
        let result = load_config(path).await;
        drop(guard);
        result
    }

    async fn write_module(source: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fontmin.config.mjs");
        tokio::fs::write(&path, source).await.unwrap();
        (dir, path)
    }

    #[tokio::test]
    async fn discovery_uses_exact_module_before_json_order() {
        let dir = tempfile::tempdir().unwrap();
        let names = [
            "fontmin.config.ts",
            "fontmin.config.mts",
            "fontmin.config.mjs",
            "fontmin.config.cjs",
            "fontmin.config.json",
            "fontmin.config.jsonc",
        ];

        for name in names.into_iter().rev() {
            tokio::fs::write(dir.path().join(name), "{}").await.unwrap();
            assert_eq!(
                find_config(dir.path()).await.unwrap(),
                Some(dir.path().join(name)),
            );
        }
    }

    #[tokio::test]
    async fn jsonc_loading_does_not_require_node() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fontmin.config.jsonc");
        tokio::fs::write(&path, "{ \"input\": [\"font.ttf\"] }")
            .await
            .unwrap();
        let guard = ENV_LOCK.lock().await;
        let old_path = env::var_os("PATH");
        unsafe { env::set_var("PATH", "") };

        let result = load_config(&path).await;

        match old_path {
            Some(value) => unsafe { env::set_var("PATH", value) },
            None => unsafe { env::remove_var("PATH") },
        }
        drop(guard);
        assert_eq!(result.unwrap().input, vec!["font.ttf"]);
    }

    #[tokio::test]
    async fn module_config_loads_default_object() {
        let (_dir, path) = write_module("export default { input: ['font.ttf'] }").await;
        assert_eq!(load(&path).await.unwrap().input, vec!["font.ttf"]);
    }

    #[tokio::test]
    async fn module_config_loads_named_config() {
        let (_dir, path) = write_module("export const config = { input: ['font.ttf'] }").await;
        assert_eq!(load(&path).await.unwrap().input, vec!["font.ttf"]);
    }

    #[tokio::test]
    async fn module_config_awaits_async_factory() {
        let (_dir, path) = write_module(
            "export default async () => ({ input: ['font.ttf'], outputs: [{ format: 'woff2' }] })",
        )
        .await;
        assert_eq!(load(&path).await.unwrap().input, vec!["font.ttf"]);
    }

    #[tokio::test]
    async fn module_plugins_preserve_explicit_outputs_and_css() {
        let (_dir, path) = write_module(
            "export default { plugins: [], outputs: [{ format: 'woff2', clone: false }], css: { fontFamily: 'Explicit Family' } }",
        )
        .await;

        let config = load(&path).await.unwrap();

        assert_eq!(config.outputs.len(), 1);
        assert_eq!(config.outputs[0].format, fontmin::OutputFormat::Woff2);
        assert!(!config.outputs[0].clone);
        assert_eq!(
            config.css.unwrap().font_family.as_deref(),
            Some("Explicit Family")
        );
    }

    #[tokio::test]
    async fn empty_module_plugins_preserve_absent_outputs_and_css() {
        let (_dir, path) = write_module("export default { plugins: [] }").await;

        let config = load(&path).await.unwrap();

        assert!(config.outputs.is_empty());
        assert!(config.css.is_none());
    }

    #[tokio::test]
    async fn module_without_plugins_retains_rust_defaults() {
        let (_dir, path) = write_module("export default { input: ['font.ttf'] }").await;

        let config = load(&path).await.unwrap();

        assert_eq!(config.outputs.len(), 5);
        assert!(config.css.is_some());
    }

    #[tokio::test]
    async fn json_formats_with_plugins_retain_rust_defaults() {
        for extension in ["json", "jsonc"] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(format!("fontmin.config.{extension}"));
            tokio::fs::write(&path, "{ \"plugins\": [] }")
                .await
                .unwrap();

            let config = load(&path).await.unwrap();

            assert_eq!(config.outputs.len(), 5, "failed extension: {extension}");
            assert!(config.css.is_some(), "failed extension: {extension}");
        }
    }

    #[tokio::test]
    async fn module_config_routes_console_output_away_from_json() {
        let (_dir, path) = write_module(
            "console.log('config log'); console.warn({ warning: true }); export default { input: ['font.ttf'] }",
        )
        .await;
        assert_eq!(load(&path).await.unwrap().input, vec!["font.ttf"]);
    }

    #[tokio::test]
    async fn module_config_routes_dir_and_table_away_from_json() {
        let (_dir, path) = write_module(
            "console.dir({ nested: { value: true } }); console.table([{ name: 'font.ttf' }]); export default { input: ['font.ttf'] }",
        )
        .await;
        assert_eq!(load(&path).await.unwrap().input, vec!["font.ttf"]);
    }

    #[tokio::test]
    async fn module_config_loads_commonjs_module_exports() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fontmin.config.cjs");
        tokio::fs::write(&path, "module.exports = { input: ['font.ttf'] }")
            .await
            .unwrap();

        assert_eq!(load(&path).await.unwrap().input, vec!["font.ttf"]);
    }

    #[tokio::test]
    async fn module_config_missing_node_has_dedicated_diagnostic() {
        let (_dir, path) = write_module("export default {}").await;
        let guard = ENV_LOCK.lock().await;
        let old_path = env::var_os("PATH");
        unsafe { env::set_var("PATH", "") };

        let result = load_config(&path).await;

        match old_path {
            Some(value) => unsafe { env::set_var("PATH", value) },
            None => unsafe { env::remove_var("PATH") },
        }
        drop(guard);
        let error = result.unwrap_err().to_string();
        assert!(error.contains(super::MODULE_CONFIG_NODE_ERROR), "{error}");
    }

    #[tokio::test]
    async fn module_config_nonzero_exit_includes_stderr() {
        let (_dir, path) = write_module("throw new Error('bridge exploded')").await;
        let error = load(&path).await.unwrap_err().to_string();
        assert!(error.contains("bridge exploded"), "{error}");
    }

    #[tokio::test]
    async fn module_config_rejects_empty_stdout() {
        let (_dir, path) = write_module("process.exit(0)").await;
        let error = load(&path).await.unwrap_err().to_string();
        assert!(error.contains("empty stdout"), "{error}");
    }

    #[tokio::test]
    async fn module_config_rejects_non_json_stdout() {
        let (_dir, path) = write_module("process.stdout.write('noise'); export default {}").await;
        let error = load(&path).await.unwrap_err().to_string();
        assert!(error.contains("invalid JSON"), "{error}");
    }

    #[tokio::test]
    async fn module_config_rejects_custom_plugin_function_with_path() {
        let (_dir, path) =
            write_module("export default { plugins: [{ name: 'custom', transform() {} }] }").await;
        let error = load(&path).await.unwrap_err().to_string();
        assert!(error.contains("plugins[0].transform"), "{error}");
    }

    #[tokio::test]
    async fn module_config_rejects_function_css_family_with_path() {
        let (_dir, path) = write_module("export default { css: { fontFamily() {} } }").await;
        let error = load(&path).await.unwrap_err().to_string();
        assert!(error.contains("css.fontFamily"), "{error}");
    }

    #[tokio::test]
    async fn module_config_rejects_bigint_with_path() {
        let (_dir, path) = write_module("export default { input: 1n }").await;
        let error = load(&path).await.unwrap_err().to_string();
        assert!(
            error.contains("input") && error.contains("bigint"),
            "{error}"
        );
    }

    #[tokio::test]
    async fn module_config_rejects_cycles_with_path() {
        let (_dir, path) = write_module(
            "const config = { input: [] }; config.self = config; export default config",
        )
        .await;
        let error = load(&path).await.unwrap_err().to_string();
        assert!(error.contains("self") && error.contains("cycle"), "{error}");
    }

    #[tokio::test]
    async fn module_config_rejects_unknown_plugin_with_path() {
        let (_dir, path) = write_module(
            "export default { plugins: [{ name: 'fontmin:unknown', native: { kind: 'builtin', name: 'unknown', options: {} } }] }",
        )
        .await;
        let error = load(&path).await.unwrap_err().to_string();
        assert!(
            error.contains("plugins[0]") && error.contains("unknown"),
            "{error}"
        );
    }

    fn glyph_plugin(options: &serde_json::Value) -> serde_json::Value {
        json!({
            "name": "fontmin:glyph",
            "native": { "kind": "builtin", "name": "glyph", "options": options }
        })
    }

    #[tokio::test]
    async fn glyph_text_file_is_relative_to_config_cwd_and_removed_before_engine() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("glyphs.txt"), "World")
            .await
            .unwrap();
        let mut config = serde_json::from_value(json!({
            "plugins": [glyph_plugin(&json!({ "textFile": "glyphs.txt" }))]
        }))
        .unwrap();

        resolve_plugin_text_files(&mut config, dir.path())
            .await
            .unwrap();

        assert_eq!(config.plugins[0].native.options, json!({ "text": "World" }));
        Engine::try_new(config).unwrap();
    }

    #[test]
    fn direct_engine_construction_still_rejects_glyph_text_file() {
        let config = serde_json::from_value(json!({
            "plugins": [glyph_plugin(&json!({ "textFile": "glyphs.txt" }))]
        }))
        .unwrap();

        let error = Engine::try_new(config)
            .err()
            .expect("textFile must remain invalid at the engine boundary")
            .to_string();
        assert!(error.contains("textFile"), "{error}");
    }

    #[tokio::test]
    async fn glyph_text_file_appends_to_existing_text() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("glyphs.txt"), "World")
            .await
            .unwrap();
        let mut config = serde_json::from_value(json!({
            "plugins": [glyph_plugin(&json!({ "text": "Hello ", "textFile": "glyphs.txt" }))]
        }))
        .unwrap();

        resolve_plugin_text_files(&mut config, dir.path())
            .await
            .unwrap();

        assert_eq!(config.plugins[0].native.options["text"], "Hello World");
    }

    #[tokio::test]
    async fn glyph_text_file_preprocesses_every_glyph_descriptor() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("one.txt"), "One")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("two.txt"), "Two")
            .await
            .unwrap();
        let mut config = serde_json::from_value(json!({
            "plugins": [
                glyph_plugin(&json!({ "textFile": "one.txt" })),
                glyph_plugin(&json!({ "text": "+", "textFile": "two.txt" }))
            ]
        }))
        .unwrap();

        resolve_plugin_text_files(&mut config, dir.path())
            .await
            .unwrap();

        assert_eq!(config.plugins[0].native.options, json!({ "text": "One" }));
        assert_eq!(config.plugins[1].native.options, json!({ "text": "+Two" }));
    }

    #[tokio::test]
    async fn glyph_text_file_read_error_has_plugin_path_and_file_context() {
        let mut config = serde_json::from_value(json!({
            "plugins": [glyph_plugin(&json!({ "textFile": "missing.txt" }))]
        }))
        .unwrap();

        let error = resolve_plugin_text_files(&mut config, Path::new("/missing-config-dir"))
            .await
            .unwrap_err()
            .to_string();

        assert!(
            error.contains("plugins[0].native.options.textFile"),
            "{error}"
        );
        assert!(error.contains("/missing-config-dir/missing.txt"), "{error}");
    }
}
