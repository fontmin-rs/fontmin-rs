use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use fontmin_config::FontminConfig;
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

const NODE_CONFIG_BRIDGE: &str = r"
import { inspect } from 'node:util'
import { pathToFileURL } from 'node:url'

const nodeMajor = Number.parseInt(process.versions.node.split('.')[0], 10)
if (nodeMajor < 22) throw new Error('module config requires Node.js 22 or newer')

for (const method of ['log', 'info', 'warn', 'error', 'debug']) {
  console[method] = (...values) => {
    process.stderr.write(`${values.map(value => inspect(value)).join(' ')}\n`)
  }
}

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
  if (config === null || typeof config !== 'object' || Array.isArray(config)) {
    throw new Error('config must be a plain object')
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
if (normalized.plugins !== undefined && normalized.outputs === undefined) {
  normalized.outputs = []
  if (normalized.css === undefined) normalized.css = null
}
process.stdout.write(JSON.stringify(normalized))
";

pub async fn load_config(path: &Path) -> Result<FontminConfig> {
    let extension = path.extension().and_then(|extension| extension.to_str());

    if matches!(extension, Some("ts" | "mts" | "mjs" | "cjs")) {
        return load_module_config(path).await;
    }

    let contents = tokio::fs::read_to_string(path)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", path.display()))?;

    match extension {
        Some("json") => serde_json::from_str(&contents).into_diagnostic(),
        Some("jsonc") => jsonc_parser::parse_to_serde_value(&contents, &ParseOptions::default())
            .into_diagnostic(),
        Some(extension) => Err(miette!("unsupported config extension `.{extension}`")),
        None => Err(miette!("config file requires an extension")),
    }
    .wrap_err_with(|| format!("failed to parse {}", path.display()))
}

async fn load_module_config(path: &Path) -> Result<FontminConfig> {
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().into_diagnostic()?.join(path)
    };
    let mut child = Command::new("node")
        .args(["--input-type=module", "--eval", NODE_CONFIG_BRIDGE])
        .arg(&absolute_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| {
            miette!("module config requires Node.js 22 or newer; install Node.js or use JSON/JSONC")
        })?;
    let mut stdout = child.stdout.take().expect("configured stdout pipe");
    let stderr = child.stderr.take().expect("configured stderr pipe");
    let stdout_future = async {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).await.map(|_| bytes)
    };
    let (status, stdout, stderr) = tokio::join!(
        child.wait(),
        stdout_future,
        read_prefix_to_end(stderr, STDERR_LIMIT),
    );
    let status = status.into_diagnostic()?;
    let stdout = stdout.into_diagnostic()?;
    let stderr = stderr.into_diagnostic()?;
    let stderr = String::from_utf8_lossy(&stderr).trim().to_owned();

    if !status.success() {
        let detail = if stderr.is_empty() {
            format!("Node exited with status {status}")
        } else {
            stderr
        };

        return Err(miette!(
            "failed to evaluate module config {}: {detail}",
            absolute_path.display()
        ));
    }
    if stdout.is_empty() {
        return Err(miette!(
            "module config {} returned an empty response",
            absolute_path.display()
        ));
    }

    serde_json::from_slice(&stdout)
        .into_diagnostic()
        .wrap_err_with(|| {
            format!(
                "module config {} returned invalid JSON",
                absolute_path.display()
            )
        })
}

async fn read_prefix_to_end(
    mut reader: impl tokio::io::AsyncRead + Unpin,
    limit: usize,
) -> std::io::Result<Vec<u8>> {
    let mut retained = Vec::new();
    let mut chunk = [0_u8; 8192];

    loop {
        let read = reader.read(&mut chunk).await?;

        if read == 0 {
            break;
        }

        let remaining = limit.saturating_sub(retained.len());
        retained.extend_from_slice(&chunk[..read.min(remaining)]);
    }

    Ok(retained)
}

pub async fn find_config(cwd: &Path) -> Result<Option<PathBuf>> {
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

#[cfg(test)]
mod tests {
    use super::{find_config, load_config};

    #[tokio::test]
    async fn discovery_prefers_typescript_before_jsonc() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("fontmin.config.jsonc"), "{}")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("fontmin.config.ts"), "export default {}")
            .await
            .unwrap();

        assert_eq!(
            find_config(dir.path()).await.unwrap(),
            Some(dir.path().join("fontmin.config.ts")),
        );
    }

    #[tokio::test]
    async fn jsonc_loading_does_not_require_node() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fontmin.config.jsonc");
        tokio::fs::write(&path, "{ \"input\": [\"font.ttf\"] }")
            .await
            .unwrap();

        let config = load_config(&path).await.unwrap();
        assert_eq!(config.input, vec!["font.ttf"]);
    }

    #[tokio::test]
    async fn module_config_loads_async_default_and_named_exports() {
        let dir = tempfile::tempdir().unwrap();
        let default_path = dir.path().join("fontmin.config.mjs");
        let named_path = dir.path().join("fontmin.named.mjs");
        tokio::fs::write(
            &default_path,
            "export default async () => ({ input: ['font.ttf'], outputs: [{ format: 'woff2' }] })",
        )
        .await
        .unwrap();
        tokio::fs::write(
            &named_path,
            "export const config = { input: ['named.ttf'], outputs: [] }",
        )
        .await
        .unwrap();

        let default_config = load_config(&default_path).await.unwrap();
        let named_config = load_config(&named_path).await.unwrap();

        assert_eq!(default_config.input, vec!["font.ttf"]);
        assert_eq!(named_config.input, vec!["named.ttf"]);
    }

    #[tokio::test]
    async fn module_config_reports_non_serializable_field_paths() {
        let dir = tempfile::tempdir().unwrap();

        for (name, source, expected) in [
            (
                "plugin",
                "export default { plugins: [{ name: 'custom', transform() {} }] }",
                "plugins[0].transform",
            ),
            (
                "family",
                "export default { css: { fontFamily() { return 'Font' } } }",
                "css.fontFamily",
            ),
            (
                "bigint",
                "export default { value: 1n }",
                "value is not serializable (bigint)",
            ),
            (
                "cycle",
                "const config = {}; config.self = config; export default config",
                "self contains a cycle",
            ),
        ] {
            let path = dir.path().join(format!("{name}.mjs"));
            tokio::fs::write(&path, source).await.unwrap();
            let error = load_config(&path).await.unwrap_err();

            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[tokio::test]
    async fn module_config_rejects_unknown_plugins_and_keeps_stdout_clean() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fontmin.config.mjs");
        tokio::fs::write(
            &path,
            "console.log('config log'); export default { plugins: [{ name: 'fontmin:nope', native: { kind: 'builtin', name: 'nope', options: {} } }] }",
        )
        .await
        .unwrap();
        let error = load_config(&path).await.unwrap_err();

        assert!(error.to_string().contains("plugins[0]"), "{error}");
    }
}
