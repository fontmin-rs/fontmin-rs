# Rust CLI Module Configuration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the Rust CLI discover and execute TypeScript and JavaScript configuration modules through an on-demand Node bridge, then run their serializable built-ins through the Rust pipeline.

**Architecture:** Move CLI config discovery/loading into a focused module with separate pure-Rust JSON and Node-backed module paths. Extend the Rust config and pipeline layers to deserialize and construct the same built-in plugin descriptors emitted by the Node package, while rejecting functions, unknown plugins, unsupported options, and non-JSON values at the bridge boundary.

**Tech Stack:** Rust 2024, Tokio process APIs, Serde/serde_json, Node.js 22+ ESM and native TypeScript stripping, bpaf CLI, Cargo tests.

## Global Constraints

- Discovery order is `.ts`, `.mts`, `.mjs`, `.cjs`, `.json`, then `.jsonc`.
- JSON and JSONC never start Node and remain usable without Node on `PATH`.
- Module configs require Node.js 22 or newer and may export `default` or `config` objects or sync/async factories.
- The bridge accepts only JSON-compatible values and known serializable built-in plugin descriptors.
- Custom JS hooks, function-valued CSS families, symbols, bigint, cycles, unknown plugins, and unsupported options fail with field paths.
- Rust applies `cwd` defaults and CLI overrides after module evaluation.
- The Rust pipeline preserves plugin declaration order within pre/normal/post groups.
- `fontmin-rs init` continues to generate JSONC.
- Do not embed a JS engine, delegate the build command to the npm CLI, change versions, or publish packages.

---

### Task 1: Model Node Built-in Plugin Descriptors in Rust

**Files:**
- Modify: `crates/fontmin_config/src/config.rs`
- Modify: `crates/fontmin_config/src/lib.rs`
- Test: `crates/fontmin_config/src/config.rs`

**Interfaces:**
- Produces: `PluginConfig`, `PluginEnforce`, and `BuiltinPluginConfig` matching Node's serialized descriptor.
- Consumes: `serde_json::Value` for operation-specific options.

- [ ] **Step 1: Write failing deserialization tests**

Add tests to `crates/fontmin_config/src/config.rs`:

```rust
#[test]
fn deserializes_node_builtin_plugin_descriptors() {
    let config: FontminConfig = serde_json::from_str(
        r#"{
          "plugins": [
            {
              "name": "fontmin:glyph",
              "enforce": "pre",
              "native": {
                "kind": "builtin",
                "name": "glyph",
                "options": { "text": "Hello", "clone": false }
              }
            }
          ]
        }"#,
    )
    .unwrap();

    assert_eq!(config.plugins[0].name, "fontmin:glyph");
    assert_eq!(config.plugins[0].enforce, Some(PluginEnforce::Pre));
    assert_eq!(config.plugins[0].native.name, "glyph");
    assert_eq!(config.plugins[0].native.options["text"], "Hello");
}

#[test]
fn rejects_non_builtin_plugin_descriptors() {
    let error = serde_json::from_str::<FontminConfig>(
        r#"{"plugins":[{"name":"custom","native":{"kind":"custom","name":"custom","options":{}}}]}"#,
    )
    .unwrap_err();

    assert!(error.to_string().contains("unknown variant `custom`"));
}
```

- [ ] **Step 2: Run the config tests and verify RED**

Run: `rtk cargo test -p fontmin_config node_builtin_plugin`

Expected: FAIL because the current `PluginConfig` has only `name` and
`options` and `PluginEnforce` does not exist.

- [ ] **Step 3: Replace the unused plugin placeholder with the Node shape**

Implement:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfig {
    pub name: String,
    pub enforce: Option<PluginEnforce>,
    pub native: BuiltinPluginConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginEnforce {
    Pre,
    Post,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinPluginConfig {
    pub kind: BuiltinPluginKind,
    pub name: String,
    #[serde(default)]
    pub options: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuiltinPluginKind {
    Builtin,
}
```

Re-export the new types from `crates/fontmin_config/src/lib.rs`. Keep
`FontminConfig.plugins` defaulting to an empty vector.

- [ ] **Step 4: Run config tests and verify GREEN**

Run: `rtk cargo test -p fontmin_config`

Expected: PASS.

- [ ] **Step 5: Commit the descriptor schema**

```bash
rtk git add crates/fontmin_config/src/config.rs crates/fontmin_config/src/lib.rs
rtk git commit -m "feat: model builtin plugin configs"
```

### Task 2: Construct Ordered Rust Plugins From Node Descriptors

**Files:**
- Modify: `crates/fontmin_pipeline/src/lib.rs`
- Test: `crates/fontmin/tests/pipeline.rs`

**Interfaces:**
- Consumes: `PluginConfig` and operation option JSON from Task 1.
- Produces: `Engine::try_new(config) -> Result<Engine>` and ordered wrappers for supported built-ins.

- [ ] **Step 1: Write failing modern-web descriptor and rejection tests**

Add a `#[tokio::test]` that deserializes this configuration and runs it:

```rust
let config: FontminConfig = serde_json::from_value(serde_json::json!({
    "plugins": [
        { "name": "fontmin:glyph", "native": { "kind": "builtin", "name": "glyph", "options": { "text": "Hello", "clone": false } } },
        { "name": "fontmin:ttf2woff", "native": { "kind": "builtin", "name": "ttf2woff", "options": { "clone": true } } },
        { "name": "fontmin:ttf2woff2", "native": { "kind": "builtin", "name": "ttf2woff2", "options": { "clone": false } } },
        { "name": "fontmin:css", "native": { "kind": "builtin", "name": "css", "options": { "fontFamily": "Roboto Module", "local": false } } }
    ],
    "outputs": [],
    "css": null
}))
.unwrap();

let assets = Engine::try_new(config)
    .unwrap()
    .with_assets(vec![roboto_asset()])
    .run()
    .await
    .unwrap();

assert!(assets.iter().any(|asset| asset.format == FontFormat::Woff));
assert!(assets.iter().any(|asset| asset.format == FontFormat::Woff2));
assert!(assets.iter().any(|asset| asset.format == FontFormat::Css));
```

Add tests that `native.name = "unknown"` and an extra WOFF2 option both return
errors containing `unsupported built-in plugin` or `unknown field`.

- [ ] **Step 2: Run focused pipeline tests and verify RED**

Run: `rtk cargo test -p fontmin node_builtin_plugins`

Expected: FAIL because `Engine::try_new` does not exist and config plugins are
ignored.

- [ ] **Step 3: Add fallible engine construction and ordered delegation**

Keep `Engine::new(config)` for source compatibility by making it call
`Engine::try_new(config).expect("invalid fontmin configuration")`. Add:

```rust
pub fn try_new(config: FontminConfig) -> Result<Self> {
    let mut engine = Self {
        assets: Vec::new(),
        plugins: Vec::new(),
    };
    engine.configure_explicit_plugins(&config.plugins)?;
    engine.configure_builtin_plugins(config);
    Ok(engine)
}
```

Create `OrderedPlugin` containing `inner: Box<dyn FontminPlugin>` and
`order: PluginOrder`. Delegate all trait methods and return the configured
order. This allows `enforce` to override a built-in's default while Rust's
stable `sort_by_key` preserves declaration order within groups.

- [ ] **Step 4: Add strict option structs and factories for every supported built-in**

Define private `#[derive(Deserialize)]` option structs with
`#[serde(default, rename_all = "camelCase", deny_unknown_fields)]` for:

- glyph: all `SubsetOptions` fields plus `clone`, `hinting`, and `textFile`;
- unicode slices: `slices`;
- OTF to TTF: `clone`, `preserveHinting`, `variationCoordinates`;
- WOFF: `clone`, `deflate`, `compressionLevel`, and `metadata`; reject a
  provided `privateData` because the module bridge does not serialize typed
  arrays;
- WOFF2: `clone`, `quality` and a rejected Node-only `fallback` unless absent;
- EOT: `clone`, `version`;
- TTF/SVG and SVG/TTF: their documented options plus `clone`;
- SVG collection: `fontName`, `startUnicode`, `ascent`, `descent`, `normalize`, `clone`;
- CSS: the serializable `CssConfig` fields.

Use these concrete shapes (all omitted values remain `None` so each factory
can apply the corresponding Node plugin's default semantics):

```rust
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
struct SlicePluginOptions { slices: Vec<FontDeliverySlice> }

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
struct Woff2PluginOptions { clone: Option<bool>, quality: Option<u8> }

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct EotPluginOptions { clone: Option<bool>, version: Option<u32> }

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct TtfSvgPluginOptions { clone: Option<bool>, font_family: Option<String> }

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
```

Use `serde_json::from_value()` for each descriptor, convert to the existing
Rust option structs, and return the matching plugin. Verify `config.name`
matches `fontmin:${config.native.name}` except for `unicodeSlices`, whose
public name is `fontmin:unicode-slices`.

For options that Rust cannot represent, return a diagnostic naming the plugin
and option; do not discard the value. Preserve clone behavior exactly.

- [ ] **Step 5: Run focused and full pipeline tests to verify GREEN**

Run: `rtk cargo test -p fontmin node_builtin_plugins`

Expected: PASS.

Run: `rtk cargo test -p fontmin -p fontmin_pipeline`

Expected: PASS.

- [ ] **Step 6: Commit built-in plugin construction**

```bash
rtk git add crates/fontmin_pipeline/src/lib.rs crates/fontmin/tests/pipeline.rs
rtk git commit -m "feat: construct configured builtin plugins"
```

### Task 3: Extract Config Loading and Add the Embedded Node Bridge

**Files:**
- Create: `apps/fontmin/src/config.rs`
- Modify: `apps/fontmin/src/main.rs`
- Modify: `apps/fontmin/src/commands/build.rs`
- Modify: `apps/fontmin/Cargo.toml`
- Test: `apps/fontmin/src/config.rs`

**Interfaces:**
- Produces: `find_config(cwd) -> Result<Option<PathBuf>>` and `load_config(path) -> Result<FontminConfig>`.
- Consumes: Tokio filesystem/process, JSONC parser, Node dynamic import.

- [ ] **Step 1: Write failing unit tests for discovery and pure-Rust loading**

Add tests in the new module for the exact discovery order and for JSON loading
with an empty `PATH`:

```rust
#[tokio::test]
async fn discovery_prefers_typescript_before_jsonc() {
    let dir = tempfile::tempdir().unwrap();
    tokio::fs::write(dir.path().join("fontmin.config.jsonc"), "{}").await.unwrap();
    tokio::fs::write(dir.path().join("fontmin.config.ts"), "export default {}").await.unwrap();

    assert_eq!(
        find_config(dir.path()).await.unwrap(),
        Some(dir.path().join("fontmin.config.ts")),
    );
}

#[tokio::test]
async fn jsonc_loading_does_not_require_node() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fontmin.config.jsonc");
    tokio::fs::write(&path, "{ \"input\": [\"font.ttf\"] }").await.unwrap();

    let config = load_config(&path).await.unwrap();
    assert_eq!(config.input, vec!["font.ttf"]);
}
```

- [ ] **Step 2: Run module tests and verify RED**

Run: `rtk cargo test -p fontmin_app config::tests`

Expected: FAIL because `apps/fontmin/src/config.rs` is absent.

- [ ] **Step 3: Move JSON/JSONC loading and discovery into the focused module**

Use this constant:

```rust
const DEFAULT_CONFIG_FILES: &[&str] = &[
    "fontmin.config.ts",
    "fontmin.config.mts",
    "fontmin.config.mjs",
    "fontmin.config.cjs",
    "fontmin.config.json",
    "fontmin.config.jsonc",
];
```

Move the existing pure-Rust parse and file tests from `commands/build.rs`.
Expose the module from `main.rs` and import `find_config`/`load_config` into the
build command. Preserve config-path error context. Replace build-path calls to
`Engine::new(config)` with `Engine::try_new(config).into_diagnostic()?` so
invalid module plugin descriptors become CLI diagnostics rather than panics.

- [ ] **Step 4: Write failing module-evaluation and boundary tests**

Add tests using `.mjs` files for default object, named `config`, async factory,
console output, custom function, function-valued `css.fontFamily`, bigint,
cycle, and unknown plugin. The successful async test is:

```rust
tokio::fs::write(
    &path,
    "export default async () => ({ input: ['font.ttf'], outputs: [{ format: 'woff2' }] })",
)
.await
.unwrap();
let config = load_config(&path).await.unwrap();
assert_eq!(config.input, vec!["font.ttf"]);
```

Assert rejection messages contain paths such as `plugins[0].transform` and
`css.fontFamily`.

- [ ] **Step 5: Run module tests and verify RED**

Run: `rtk cargo test -p fontmin_app module_config`

Expected: FAIL with `unsupported config extension .mjs`.

- [ ] **Step 6: Implement the embedded ESM bridge and strict response parsing**

Add Tokio's `process` and `io-util` features in the workspace dependency and
invoke Node with `tokio::process::Command`. The embedded script must contain
this normalization and validation logic:

```js
import { inspect } from 'node:util'
import { pathToFileURL } from 'node:url'

const nodeMajor = Number.parseInt(process.versions.node.split('.')[0], 10)
if (nodeMajor < 22) {
  throw new Error('module config requires Node.js 22 or newer')
}

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
process.stdout.write(JSON.stringify(normalized))
```

The generic normalizer must run before `validatePlugins()` so absent
`undefined` properties emitted by built-in factories are omitted, while
undefined array entries are rejected. Function-valued `css.fontFamily` is
caught by the generic normalizer at `css.fontFamily`.

Rust treats spawn failure as:

```text
module config requires Node.js 22 or newer; install Node.js or use JSON/JSONC
```

Pipe stdout and stderr separately. Drain stderr to EOF while retaining only its
first 64 KiB, require a successful exit, reject empty stdout, and deserialize
the entire stdout buffer with `serde_json::from_slice`. Draining after the
retained prefix prevents a verbose config from blocking on a full child pipe
without allowing unbounded diagnostic memory.

- [ ] **Step 7: Run config tests and verify GREEN**

Run: `rtk cargo test -p fontmin_app config::tests`

Expected: PASS.

Run: `rtk cargo clippy -p fontmin_app --all-targets --all-features`

Expected: exit `0` without new warnings.

- [ ] **Step 8: Commit the config bridge**

```bash
rtk git add Cargo.toml Cargo.lock apps/fontmin/Cargo.toml apps/fontmin/src/main.rs apps/fontmin/src/config.rs apps/fontmin/src/commands/build.rs
rtk git commit -m "feat: load module configs in rust cli"
```

### Task 4: Verify Every Module Extension, Presets, Overrides, and Missing Node

**Files:**
- Modify: `apps/fontmin/tests/cli.rs`

**Interfaces:**
- Consumes: module loader from Task 3 and explicit-plugin engine from Task 2.
- Produces: end-to-end Rust CLI parity evidence.

- [ ] **Step 1: Add extension and export integration tests**

Use a table-driven test over `ts`, `mts`, `mjs`, and `cjs`. For ESM/TS write:

```ts
const family: string = 'Module Font'
export default async () => ({
  input: ['roboto.ttf'],
  outDir: 'module-output',
  outputs: [{ format: 'woff2', clone: false }],
  css: { fontFamily: family },
})
```

For CJS write the same object through `module.exports`. Copy the Roboto fixture
beside each config, run `fontmin-rs build --config <path>`, and assert a valid
`module-output/roboto.woff2` starts with `wOF2`.

- [ ] **Step 2: Run extension tests and verify their precondition**

Run: `rtk cargo test -p fontmin_app module_config_extensions -- --nocapture`

Expected after Tasks 1–3: PASS. If a Node version rejects erasable TypeScript,
the test must expose that environment failure rather than skipping `.ts`.

- [ ] **Step 3: Add a real `modernWeb()` import test**

Create the temporary config inside `packages/fontmin/` so Node package
self-resolution can import `fontmin-rs`:

```ts
import { defineConfig, modernWeb } from 'fontmin-rs'

export default defineConfig({
  input: ['roboto.ttf'],
  outDir: 'module-output',
  plugins: modernWeb({
    clone: false,
    fontFamily: 'Module Roboto',
    text: 'Hello',
  }),
})
```

Run the Rust CLI and assert WOFF, WOFF2, and CSS outputs and the CSS family.

- [ ] **Step 4: Add override, relative-path, discovery, and no-Node tests**

Add end-to-end tests proving:

- `--out-dir`, `--text`, `--formats`, cache flags, CSS flags, delivery slices,
  and variations override module values just as they override JSONC;
- relative input, `textFile`, output, and cache paths use the config directory;
- auto-discovery prefers `.ts` over `.jsonc`;
- an `.mjs` config with `PATH` cleared reports the dedicated Node requirement;
- a JSONC build with `PATH` cleared still emits its requested output.

- [ ] **Step 5: Run the complete Rust CLI suite**

Run: `rtk cargo test -p fontmin_app -- --nocapture`

Expected: PASS with all module and existing CLI tests.

- [ ] **Step 6: Commit CLI parity tests**

```bash
rtk git add apps/fontmin/tests/cli.rs
rtk git commit -m "test: cover rust module config parity"
```

### Task 5: Document the Shared Config Contract and Run Repository Verification

**Files:**
- Modify: `docs/architecture.md`
- Modify: `docs/guide/cli.md`
- Modify: `docs/guide/config.md`
- Modify: `docs/zh/architecture.md`
- Modify: `docs/zh/guide/cli.md`
- Modify: `docs/zh/guide/config.md`

**Interfaces:**
- Documents: discovery order, Node requirement, trusted-code model, built-in boundary, and JSONC fallback.

- [ ] **Step 1: Update English and Chinese documentation**

Document this supported async module example:

```ts
import { modernWeb } from 'fontmin-rs'

export default async () => ({
  input: ['fonts/*.ttf'],
  outDir: 'build',
  plugins: modernWeb({ text: 'Hello' }),
})
```

State the exact discovery order, Node 22+ requirement, executable trusted-code
model, supported built-ins/presets, rejected custom functions, and JSONC's
dependency-free behavior. Remove the old statement that the Rust CLI supports
only JSON/JSONC.

- [ ] **Step 2: Run documentation and Rust verification**

Run: `rtk cargo fmt --all --check`

Expected: exit `0`.

Run: `rtk cargo clippy --workspace --all-targets --all-features`

Expected: exit `0` without new warnings.

Run: `rtk cargo test --workspace`

Expected: PASS.

Run: `rtk pnpm run docs:check`

Expected: exit `0`.

- [ ] **Step 3: Commit module-config documentation**

```bash
rtk git add docs/architecture.md docs/guide/cli.md docs/guide/config.md docs/zh/architecture.md docs/zh/guide/cli.md docs/zh/guide/config.md
rtk git commit -m "docs: document rust module configs"
```

- [ ] **Step 4: Run the repository non-publishing gate**

Run: `rtk pnpm run check`

Expected: exit `0` with no test, type, lint, format, or documentation failures.
