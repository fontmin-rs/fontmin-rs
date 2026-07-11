# fontmin-rs 项目目录与代码设计方案

> 目标：参考 Rolldown 与 OXC 的 monorepo 分层方式，为 `fontmin-rs` 设计一个基于 Rust + napi-rs 的现代字体裁剪、转换、压缩与插件化工具链。
>
> 当前日期：2026-07-09

---

## 1. 设计背景与定位

`fontmin-rs` 不建议做成“简单把 fontmin 全部翻译成 Rust”的项目，而应定位为：

> 一个 Rust 实现的字体处理核心引擎，提供 Node.js napi-rs 绑定、CLI 应用、JS/TS 插件 API，并兼容 Fontmin 的主流使用体验。

当前 Fontmin 的主要价值在于：

- TTF 字体按文字裁剪，即 `glyph`；
- TTF 转 EOT / WOFF / WOFF2 / SVG；
- SVG font 转 TTF；
- 多个 SVG icon 合并成 TTF；
- OTF 转 TTF；
- 生成 `@font-face` CSS 与 icon class CSS；
- 提供 stream/vinyl 插件式管线。

`fontmin-rs` 应把真正耗时、复杂、适合 Rust 的部分下沉到 Rust crates：字体二进制解析、subset、表重建、压缩、格式转换、并行处理、缓存。JS/TS 层则保留配置加载、用户插件、CLI 包装、npm 发布和类型友好的 API。

---

## 2. 参考项目结构提炼

### 2.1 Rolldown 的启发

Rolldown 根 `Cargo.toml` 使用 workspace，并把主要 Rust crates 放在 `crates/*`，任务脚本放在 `tasks/*`。它还在 workspace dependencies 中集中声明大量内部 crate，如 `rolldown`、`rolldown_common`、`rolldown_plugin`、`rolldown_plugin_*` 等。

对 `fontmin-rs` 的启发：

- Rust 核心应拆成多个小 crate，而不是一个巨大的 `fontmin_core`；
- 插件系统可单独有 `fontmin_plugin` crate；
- 内置插件可拆成 `fontmin_plugin_*` 或集中到 `fontmin_plugins`；
- workspace dependencies 统一版本与 path；
- 严格 clippy/lints 对长期维护很重要。

Rolldown 的 npm 包 `packages/rolldown` 里也通过 `napi` 字段声明 binary name、package name 和多平台 targets，这适合 `fontmin-rs` 的 Node 原生发布模型。

### 2.2 OXC 的启发

OXC 根 `Cargo.toml` 明确使用：

```toml
members = ["apps/*", "crates/*", "napi/*", "tasks/*"]
```

对 `fontmin-rs` 的启发更直接：

- `apps/fontmin` 放 Rust CLI 应用；
- `crates/*` 放可复用 Rust 核心库；
- `napi/fontmin` 放 Node.js napi-rs binding；
- `tasks/*` 放开发、生成、基准、fixtures 管理脚本；
- 未来可扩展 `wasm/*` 支持浏览器端字体处理。

OXC 的 napi package 也展示了 `napi build --esm --platform --js bindings.js --dts index.d.ts`、`wasm32-wasip1-threads`、`src-js` wrapper 等组织方式。`fontmin-rs` 可以采用类似形式。

---

## 3. 总体目录结构

建议采用 `apps + crates + napi + packages + npm + tasks + fixtures` 的混合结构。

```txt
fontmin-rs/
  Cargo.toml
  package.json
  pnpm-workspace.yaml
  rust-toolchain.toml
  taplo.toml
  .editorconfig
  .gitignore
  README.md
  LICENSE

  apps/
    fontmin/
      Cargo.toml
      src/
        main.rs
        cli.rs
        commands/
          mod.rs
          build.rs
          subset.rs
          convert.rs
          inspect.rs
          init.rs
          doctor.rs
          bench.rs
        output.rs
        exit_code.rs
      tests/
        cli.rs

  crates/
    fontmin/
      Cargo.toml
      src/
        lib.rs
        optimize.rs
        convert.rs
        subset.rs
        preset.rs

    fontmin_core/
      Cargo.toml
      src/
        lib.rs
        asset.rs
        format.rs
        metadata.rs
        text.rs
        unicode.rs
        path.rs
        hash.rs
        result.rs

    fontmin_config/
      Cargo.toml
      src/
        lib.rs
        config.rs
        normalize.rs
        schema.rs
        defaults.rs
        js_compat.rs

    fontmin_diagnostics/
      Cargo.toml
      src/
        lib.rs
        diagnostic.rs
        error.rs
        warning.rs
        reporter.rs

    fontmin_fs/
      Cargo.toml
      src/
        lib.rs
        glob.rs
        input.rs
        output.rs
        cache.rs

    fontmin_plugin/
      Cargo.toml
      src/
        lib.rs
        context.rs
        hook.rs
        order.rs
        registry.rs
        external.rs

    fontmin_pipeline/
      Cargo.toml
      src/
        lib.rs
        engine.rs
        graph.rs
        stage.rs
        emit.rs
        cache.rs
        parallel.rs

    fontmin_detect/
      Cargo.toml
      src/
        lib.rs
        magic.rs
        mime.rs

    fontmin_ttf/
      Cargo.toml
      src/
        lib.rs
        read.rs
        write.rs
        tables/
          mod.rs
          cmap.rs
          glyf.rs
          loca.rs
          head.rs
          hhea.rs
          hmtx.rs
          maxp.rs
          name.rs
          os2.rs
          post.rs
        checksum.rs
        validate.rs

    fontmin_subset/
      Cargo.toml
      src/
        lib.rs
        plan.rs
        subset.rs
        glyph_set.rs
        cmap.rs
        layout.rs
        hinting.rs
        prune.rs

    fontmin_woff/
      Cargo.toml
      src/
        lib.rs
        encode.rs
        decode.rs
        table.rs
        compress.rs

    fontmin_woff2/
      Cargo.toml
      src/
        lib.rs
        encode.rs
        decode.rs
        brotli.rs
        google_woff2.rs
        fallback.rs

    fontmin_eot/
      Cargo.toml
      src/
        lib.rs
        encode.rs
        decode.rs
        header.rs

    fontmin_otf/
      Cargo.toml
      src/
        lib.rs
        read.rs
        cff.rs
        convert.rs
        otf2ttf.rs

    fontmin_svg/
      Cargo.toml
      src/
        lib.rs
        ttf2svg.rs
        svg2ttf.rs
        svgs2ttf.rs
        path.rs
        glyph.rs
        parser.rs
        writer.rs

    fontmin_css/
      Cargo.toml
      src/
        lib.rs
        generator.rs
        template.rs
        base64.rs
        glyph_class.rs

    fontmin_plugins/
      Cargo.toml
      src/
        lib.rs
        glyph.rs
        ttf2eot.rs
        ttf2woff.rs
        ttf2woff2.rs
        ttf2svg.rs
        svg2ttf.rs
        svgs2ttf.rs
        otf2ttf.rs
        css.rs
        preset.rs

    fontmin_napi_shared/
      Cargo.toml
      src/
        lib.rs
        buffer.rs
        error.rs
        options.rs
        js_plugin.rs

    fontmin_testing/
      Cargo.toml
      src/
        lib.rs
        fixture.rs
        snapshot.rs
        compare.rs

  napi/
    fontmin/
      Cargo.toml
      package.json
      build.rs
      src/
        lib.rs
        binding.rs
        options.rs
        task.rs
        plugin_bridge.rs
      src-js/
        index.ts
        index.d.ts
        bindings.js
        fallback.ts
        plugins.ts
        config.ts
      test/
        api.test.ts
        plugin.test.ts
        compat-fontmin.test.ts
      bench/
        subset.bench.ts
        convert.bench.ts

  packages/
    fontmin/
      package.json
      tsconfig.json
      src/
        index.ts
        cli.ts
        config.ts
        plugins/
          index.ts
          glyph.ts
          ttf2woff.ts
          ttf2woff2.ts
          css.ts
        compat/
          fontmin-class.ts
          vinyl.ts
        types.ts
      bin/
        fontmin-rs.mjs
      test/
        config.test.ts
        compat.test.ts

  npm/
    fontmin-rs-darwin-arm64/
    fontmin-rs-darwin-x64/
    fontmin-rs-linux-x64-gnu/
    fontmin-rs-linux-x64-musl/
    fontmin-rs-linux-arm64-gnu/
    fontmin-rs-win32-x64-msvc/

  wasm/
    fontmin/
      package.json
      Cargo.toml
      src/
        lib.rs
      src-js/
        index.ts

  tasks/
    xtask/
      Cargo.toml
      src/
        main.rs
        generate_schema.rs
        update_fixtures.rs
        check_artifacts.rs
    release/
      package.ts
      prepublish.ts
      targets.ts
    benchmark/
      collect.ts
      report.ts

  fixtures/
    fonts/
      ttf/
      otf/
      woff/
      woff2/
      svg-font/
      icon-svg/
    expected/
      subset/
      convert/
      css/
    corpus/
      malformed/
      large-cjk/

  examples/
    basic-node/
    cli/
    custom-plugin/
    vite-plugin/
    font-subset-service/

  docs/
    architecture.md
    plugin-api.md
    config.md
    napi.md
    compatibility.md
    benchmarks.md
```

### 为什么不把所有 JS 都放在 `napi/fontmin`？

可以，但不建议。推荐拆成：

- `napi/fontmin`：只负责 native binding、生成的 `.node` 加载、低层 JS wrapper；
- `packages/fontmin`：对用户暴露稳定 API、兼容 Fontmin class、CLI、插件工厂、配置加载。

这样以后可以替换 native binding、添加 wasm fallback、增加插件生态，而不破坏主包 API。

---

## 4. 根配置设计

### 4.1 `Cargo.toml`

```toml
[workspace]
resolver = "3"
members = ["apps/*", "crates/*", "napi/*", "tasks/*"]
exclude = ["wasm/*"]

[workspace.package]
edition = "2024"
rust-version = "1.88.0"
homepage = "https://github.com/ntnyq/fontmin-rs"
license = "MIT"
repository = "https://github.com/ntnyq/fontmin-rs"
description = "Fast font subsetter and converter written in Rust with Node.js bindings."

[workspace.lints.rust]
unsafe_op_in_unsafe_fn = "warn"
unused_unsafe = "warn"
non_ascii_idents = "warn"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
missing_errors_doc = "allow"
missing_panics_doc = "allow"
module_name_repetitions = "allow"
too_many_lines = "allow"

[workspace.dependencies]
fontmin = { path = "crates/fontmin" }
fontmin_core = { path = "crates/fontmin_core" }
fontmin_config = { path = "crates/fontmin_config" }
fontmin_diagnostics = { path = "crates/fontmin_diagnostics" }
fontmin_fs = { path = "crates/fontmin_fs" }
fontmin_plugin = { path = "crates/fontmin_plugin" }
fontmin_pipeline = { path = "crates/fontmin_pipeline" }
fontmin_detect = { path = "crates/fontmin_detect" }
fontmin_ttf = { path = "crates/fontmin_ttf" }
fontmin_subset = { path = "crates/fontmin_subset" }
fontmin_woff = { path = "crates/fontmin_woff" }
fontmin_woff2 = { path = "crates/fontmin_woff2" }
fontmin_eot = { path = "crates/fontmin_eot" }
fontmin_otf = { path = "crates/fontmin_otf" }
fontmin_svg = { path = "crates/fontmin_svg" }
fontmin_css = { path = "crates/fontmin_css" }
fontmin_plugins = { path = "crates/fontmin_plugins" }
fontmin_napi_shared = { path = "crates/fontmin_napi_shared" }
fontmin_testing = { path = "crates/fontmin_testing" }

# napi-rs
napi = { version = "3", features = ["tokio_rt"] }
napi-derive = "3"
napi-build = "2"

# serialization / config
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
schemars = "1"

# error / diagnostic
thiserror = "2"
anyhow = "1"
miette = "7"
tracing = "0.1"

# fs / glob / parallel
ignore = "0.4"
globset = "0.4"
rayon = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "fs"] }

# font ecosystem candidates
font-types = "0.9"
read-fonts = "0.34"
write-fonts = "0.37"
skrifa = "0.40"

# binary / compression
bytemuck = "1"
byteorder = "1"
flate2 = "1"
brotli = "8"
base64 = "0.22"
memchr = "2"
rustc-hash = "2"
indexmap = "2"

# cli / testing
bpaf = { version = "0.9", features = ["derive", "bright-color", "autocomplete"] }
insta = "1"
tempfile = "3"
criterion2 = "3"
```

> 说明：`read-fonts` / `write-fonts` / `skrifa` 版本需以实际发布版本为准。首版也可以先只依赖 `read-fonts` / `write-fonts`，缺口由自研模块补齐。

### 4.2 `package.json`

```json
{
  "name": "fontmin-rs-monorepo",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "pnpm --workspace-concurrency=1 --filter './napi/*' --filter './packages/*' build",
    "build:debug": "pnpm --filter fontmin-rs build:debug",
    "build:release": "pnpm --filter fontmin-rs build:release",
    "test": "pnpm --workspace-concurrency=1 --filter './napi/*' --filter './packages/*' test && cargo test --workspace",
    "lint": "cargo clippy --workspace --all-targets --all-features && oxlint .",
    "fmt": "cargo fmt --all && oxfmt .",
    "bench": "cargo bench --workspace && pnpm --filter fontmin-rs bench"
  },
  "devDependencies": {
    "@napi-rs/cli": "^3.7.2",
    "@napi-rs/wasm-runtime": "^1.1.6",
    "@types/node": "^24.0.0",
    "cac": "^7.0.0",
    "consola": "^3.4.2",
    "pathe": "^2.0.3",
    "tinybench": "^6.0.0",
    "tsx": "^4.0.0",
    "typescript": "^6.0.0",
    "vitest": "^4.0.0"
  },
  "packageManager": "pnpm@11.9.0"
}
```

### 4.3 `pnpm-workspace.yaml`

```yaml
packages:
  - packages/*
  - napi/*
  - npm/*
  - wasm/*
  - tasks/*
  - examples/*

catalog:
  '@napi-rs/cli': ^3.7.2
  '@napi-rs/wasm-runtime': ^1.1.6
  '@types/node': ^24.0.0
  cac: ^7.0.0
  consola: ^3.4.2
  pathe: ^2.0.3
  tsdown: ^0.22.0
  typescript: ^6.0.0
  vitest: ^4.0.0

allowBuilds:
  '@napi-rs/cli': true

minimumReleaseAgeExclude:
  - '@napi-rs/*'
  - '@emnapi/*'
  - fontmin-rs
```

---

## 5. 当前 Fontmin 依赖到 Rust crate 的映射

| 当前依赖/功能                  | 当前用途                              | fontmin-rs 设计                                                                                                    |
| ------------------------------ | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `fonteditor-core`              | TTF/WOFF/EOT/SVG/OTF 解析、转换、写入 | 拆成 `fontmin_ttf`、`fontmin_subset`、`fontmin_woff`、`fontmin_woff2`、`fontmin_eot`、`fontmin_otf`、`fontmin_svg` |
| `ttf2woff2`                    | TTF 转 WOFF2                          | `fontmin_woff2`，首版可封装 Google woff2 或保留 JS fallback                                                        |
| `pako`                         | WOFF deflate                          | `fontmin_woff::compress`，使用 `flate2` / `zlib-rs`                                                                |
| `code-points`                  | 文本转 unicode code points            | `fontmin_core::text` / `fontmin_core::unicode`                                                                     |
| `b3b`                          | Buffer / ArrayBuffer 转换             | napi 侧直接处理 `Buffer` / `Uint8Array`，Rust 用 `Vec<u8>` 或 borrowed slice                                       |
| `is-ttf` / `is-otf` / `is-svg` | 文件类型判断                          | `fontmin_detect`                                                                                                   |
| `through2` / `stream-combiner` | Node stream 插件管线                  | JS 兼容层保留，核心 pipeline 转为 Rust `Asset` 管线                                                                |
| `vinyl-fs` / `buffer-to-vinyl` | 文件输入输出                          | `fontmin_fs` + JS compat vinyl adapter                                                                             |
| `replace-ext`                  | 替换扩展名                            | `fontmin_core::path`                                                                                               |
| `lodash`                       | option merge / util                   | Rust/TS 原生替代                                                                                                   |
| `meow` / `get-stdin`           | CLI 参数和 stdin                      | `apps/fontmin` 使用 `bpaf`；npm bin 用 JS wrapper 调 native CLI/API                                                |
| `concat-stream`                | 收集输出                              | Rust/JS API 直接返回 `Vec<Asset>` / `Promise<OutputFile[]>`                                                        |

---

## 6. Rust crates 详细设计

### 6.1 `crates/fontmin`

顶层 Rust API，面向 CLI、napi、测试和其他 Rust 用户。

职责：

- 暴露 `optimize`、`subset_ttf`、`convert`、`inspect`；
- 组合 config、fs、pipeline、plugins；
- 提供预设：`Preset::FontminCompat`、`Preset::ModernWeb`、`Preset::IconFont`。

```rust
// crates/fontmin/src/lib.rs
pub use fontmin_config::{FontminConfig, OutputFormat};
pub use fontmin_core::{Asset, FontFormat};
pub use fontmin_diagnostics::{Diagnostic, FontminError, Result};

pub async fn optimize(config: FontminConfig) -> Result<Vec<Asset>> {
    fontmin_pipeline::Engine::new(config)
        .with_builtin_plugins()
        .run()
        .await
}

pub fn subset_ttf(input: &[u8], options: fontmin_subset::SubsetOptions) -> Result<Vec<u8>> {
    fontmin_subset::subset_ttf(input, options)
}

pub fn convert(input: &[u8], target: OutputFormat) -> Result<Vec<u8>> {
    match target {
        OutputFormat::Ttf => Ok(input.to_vec()),
        OutputFormat::Woff => fontmin_woff::encode_ttf_to_woff(input, Default::default()),
        OutputFormat::Woff2 => fontmin_woff2::encode_ttf_to_woff2(input, Default::default()),
        OutputFormat::Eot => fontmin_eot::encode_ttf_to_eot(input, Default::default()),
        OutputFormat::Svg => fontmin_svg::ttf_to_svg(input, Default::default()).map(|s| s.into_bytes()),
        OutputFormat::Otf => Err(FontminError::Unsupported("ttf to otf is not supported".into())),
    }
}
```

---

### 6.2 `crates/fontmin_core`

核心数据模型，不依赖 napi，不依赖 CLI。

```rust
// crates/fontmin_core/src/format.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontFormat {
    Ttf,
    Otf,
    Woff,
    Woff2,
    Eot,
    Svg,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Ttf,
    Woff,
    Woff2,
    Eot,
    Svg,
    Css,
}
```

```rust
// crates/fontmin_core/src/asset.rs
use std::path::PathBuf;
use indexmap::IndexMap;

#[derive(Debug, Clone)]
pub struct Asset {
    pub path: PathBuf,
    pub contents: Vec<u8>,
    pub format: FontFormat,
    pub source_format: FontFormat,
    pub meta: AssetMeta,
}

#[derive(Debug, Clone, Default)]
pub struct AssetMeta {
    pub font_family: Option<String>,
    pub glyph_count: Option<u32>,
    pub subset_count: Option<u32>,
    pub generated_by: Vec<String>,
    pub custom: IndexMap<String, serde_json::Value>,
}

impl Asset {
    pub fn rename_ext(&mut self, ext: &str) {
        self.path.set_extension(ext.trim_start_matches('.'));
    }
}
```

---

### 6.3 `crates/fontmin_config`

配置模型应同时服务：Rust、Node API、CLI、配置文件。

```rust
// crates/fontmin_config/src/config.rs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FontminConfig {
    pub cwd: Option<String>,
    pub input: Vec<String>,
    pub out_dir: Option<String>,
    pub clean: bool,
    pub preserve_original: bool,
    pub parallel: ParallelConfig,
    pub cache: CacheConfig,
    pub subset: Option<SubsetConfig>,
    pub outputs: Vec<OutputConfig>,
    pub css: Option<CssConfig>,
    pub plugins: Vec<PluginConfig>,
    pub diagnostics: DiagnosticsConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubsetConfig {
    pub text: Option<String>,
    pub text_file: Option<String>,
    pub unicodes: Vec<u32>,
    pub basic_text: bool,
    pub preserve_hinting: bool,
    pub trim: bool,
    pub keep_notdef: bool,
    pub keep_layout: LayoutSubsetMode,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OutputConfig {
    pub format: OutputFormat,
    pub clone: bool,
    pub file_name: Option<String>,
    pub ext: Option<String>,
}
```

默认配置：

```rust
impl Default for FontminConfig {
    fn default() -> Self {
        Self {
            cwd: None,
            input: vec![],
            out_dir: Some("build".into()),
            clean: false,
            preserve_original: true,
            parallel: ParallelConfig::default(),
            cache: CacheConfig::default(),
            subset: None,
            outputs: vec![
                OutputConfig::format(OutputFormat::Eot),
                OutputConfig::format(OutputFormat::Woff),
                OutputConfig::format(OutputFormat::Woff2),
                OutputConfig::format(OutputFormat::Svg),
                OutputConfig::format(OutputFormat::Css),
            ],
            css: Some(CssConfig::default()),
            plugins: vec![],
            diagnostics: DiagnosticsConfig::default(),
        }
    }
}
```

---

### 6.4 `crates/fontmin_detect`

替代 `is-ttf`、`is-otf`、`is-svg`、`is-woff`、`is-woff2`、`is-eot`。

```rust
pub fn detect_format(bytes: &[u8]) -> FontFormat {
    if bytes.starts_with(&[0x00, 0x01, 0x00, 0x00]) || bytes.starts_with(b"true") {
        return FontFormat::Ttf;
    }
    if bytes.starts_with(b"OTTO") {
        return FontFormat::Otf;
    }
    if bytes.starts_with(b"wOFF") {
        return FontFormat::Woff;
    }
    if bytes.starts_with(b"wOF2") {
        return FontFormat::Woff2;
    }
    if looks_like_eot(bytes) {
        return FontFormat::Eot;
    }
    if looks_like_svg_font(bytes) {
        return FontFormat::Svg;
    }
    FontFormat::Unknown
}
```

---

### 6.5 `crates/fontmin_ttf`

TTF 解析与写入的基础 crate。

建议分两层：

1. 高层优先使用 `read-fonts` / `write-fonts`；
2. 对 fontmin 兼容所需但现有 crate 不方便的部分，提供内部 table patcher。

职责：

- 读取 TTF 表目录；
- 读取 cmap/glyf/loca/name/head/hhea/hmtx/maxp/post/OS2；
- 计算 checksum；
- 写出新 TTF；
- 提供 `TtfFont` 结构给 subset、woff、svg 使用。

```rust
pub struct TtfFont<'a> {
    pub data: &'a [u8],
    pub tables: TableDirectory<'a>,
}

pub struct OwnedTtfFont {
    pub tables: Vec<OwnedTable>,
    pub metadata: FontMetadata,
}

pub fn read_ttf(data: &[u8]) -> Result<TtfFont<'_>>;
pub fn write_ttf(font: &OwnedTtfFont) -> Result<Vec<u8>>;
pub fn inspect_ttf(data: &[u8]) -> Result<FontMetadata>;
```

---

### 6.6 `crates/fontmin_subset`

对应当前 Fontmin 的 `glyph` 插件，是第一优先级。

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsetOptions {
    pub unicodes: Vec<u32>,
    pub text: Option<String>,
    pub basic_text: bool,
    pub preserve_hinting: bool,
    pub trim: bool,
    pub keep_notdef: bool,
    pub layout: LayoutSubsetMode,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutSubsetMode {
    Drop,
    Conservative,
    Preserve,
}

pub fn subset_ttf(input: &[u8], options: SubsetOptions) -> Result<Vec<u8>> {
    let plan = SubsetPlan::from_options(input, &options)?;
    let font = fontmin_ttf::read_ttf(input)?;
    let subset = build_subset(font, plan)?;
    fontmin_ttf::write_ttf(&subset)
}
```

Subset 流程：

```txt
input TTF
  -> detect format
  -> parse table directory
  -> parse cmap
  -> text/unicodes -> glyph ids
  -> add .notdef
  -> resolve composite glyph dependencies
  -> prune glyf + loca
  -> prune hmtx / vmtx
  -> rebuild cmap
  -> update maxp / head / hhea / OS/2
  -> optionally preserve/drop hinting tables
  -> recalc checksums
  -> output TTF
```

需要重点测试：

- 中文字体大量 code points；
- composite glyph；
- `.notdef` 保留；
- cmap format 4 / 12；
- hinting 表 `fpgm` / `prep` / `cvt `；
- Windows/macOS/browser 加载验证。

---

### 6.7 `crates/fontmin_woff`

对应 `ttf2woff` / `woff2ttf`。

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WoffOptions {
    pub deflate: bool,
    pub compression_level: Option<u32>,
    pub metadata: Option<String>,
    pub private_data: Option<Vec<u8>>,
}

pub fn encode_ttf_to_woff(ttf: &[u8], options: WoffOptions) -> Result<Vec<u8>>;
pub fn decode_woff_to_ttf(woff: &[u8]) -> Result<Vec<u8>>;
```

实现要点：

- 读取 sfnt table directory；
- 对每个 table 可选 zlib compress；
- 写 WOFF header；
- 写 table directory；
- 对齐 4 bytes；
- 保留 metadata/private data；
- 输出可被浏览器加载的 WOFF。

---

### 6.8 `crates/fontmin_woff2`

对应 `ttf2woff2`。

建议分层：

```txt
fontmin_woff2
  encode.rs          # 对外 API
  decode.rs          # WOFF2 -> TTF
  brotli.rs          # brotli adapter
  google_woff2.rs    # 可选 C++ FFI / feature gate
  fallback.rs        # wasm/js fallback 标记
```

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Woff2Options {
    pub quality: Option<u8>,
    pub allow_fallback: bool,
}

pub fn encode_ttf_to_woff2(ttf: &[u8], options: Woff2Options) -> Result<Vec<u8>>;
pub fn decode_woff2_to_ttf(woff2: &[u8]) -> Result<Vec<u8>>;
```

首版策略：

- 如果纯 Rust 实现不足够成熟，先做 feature gated 实现；
- Node 包里可以保留 JS `ttf2woff2` fallback；
- Browser/wasm 包可以走 wasm fallback；
- CI 里对输出做浏览器加载测试。

---

### 6.9 `crates/fontmin_eot`

EOT 已偏 legacy，但为了 Fontmin 兼容需要保留。

```rust
pub fn encode_ttf_to_eot(ttf: &[u8], options: EotOptions) -> Result<Vec<u8>>;
pub fn decode_eot_to_ttf(eot: &[u8]) -> Result<Vec<u8>>;
```

建议：

- 默认插件预设里可以包含 EOT 兼容；
- `modernWeb` preset 不生成 EOT；
- 文档中标注 EOT 主要面向旧 IE 兼容。

---

### 6.10 `crates/fontmin_otf`

对应 `otf2ttf`。

难点：OTF CFF/CFF2 到 glyf TrueType outlines 并非简单封装，曲线类型、hinting、CFF charstring 解析都复杂。

建议阶段化：

- v0.1：仅检测 OTF 并给出清晰 unsupported diagnostic；
- v0.2：使用现有 Rust crate 或 fontations 能力支持基础 OTF metadata inspect；
- v0.3：支持有限 OTF -> TTF；
- v1.0：再考虑高兼容转换。

```rust
pub fn otf_to_ttf(otf: &[u8], options: Otf2TtfOptions) -> Result<Vec<u8>>;
```

---

### 6.11 `crates/fontmin_svg`

对应 `ttf2svg`、`svg2ttf`、`svgs2ttf`。

```rust
pub fn ttf_to_svg(ttf: &[u8], options: Ttf2SvgOptions) -> Result<String>;
pub fn svg_font_to_ttf(svg: &str, options: Svg2TtfOptions) -> Result<Vec<u8>>;
pub fn svgs_to_ttf(inputs: Vec<SvgIcon>, options: Svgs2TtfOptions) -> Result<Vec<u8>>;
```

`svgs2ttf` 是 iconfont 核心能力，建议重视：

```rust
#[derive(Debug, Clone)]
pub struct SvgIcon {
    pub name: String,
    pub contents: String,
    pub unicode: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Svgs2TtfOptions {
    pub font_name: String,
    pub start_unicode: u32,
    pub ascent: i16,
    pub descent: i16,
    pub normalize: bool,
}
```

---

### 6.12 `crates/fontmin_css`

CSS 插件不一定需要 Rust，但放在 Rust 侧有两个好处：CLI 不依赖 JS，也可以通过 napi 返回 CSS asset。

```rust
pub fn generate_css(fonts: &[Asset], options: CssOptions) -> Result<Asset>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CssOptions {
    pub font_path: String,
    pub base64: bool,
    pub glyph: bool,
    pub icon_prefix: String,
    pub font_family: Option<String>,
    pub as_file_name: bool,
    pub local: bool,
    pub formats: Vec<OutputFormat>,
}
```

生成示例：

```css
@font-face {
  font-family: 'myfont';
  src:
    local('myfont'),
    url('./myfont.woff2') format('woff2'),
    url('./myfont.woff') format('woff');
  font-weight: normal;
  font-style: normal;
  font-display: swap;
}

.icon-home::before {
  font-family: 'myfont';
  content: '\e001';
}
```

---

## 7. 插件系统设计

### 7.1 设计目标

`fontmin-rs` 的插件系统要同时满足：

- Rust 内置插件高性能执行；
- Node.js 用户可以写 JS/TS 插件；
- 兼容 Fontmin `.use(plugin)` 的思路；
- 避免在字形级别频繁跨 N-API 边界；
- 支持 transform、emit、diagnostic、cache、parallel。

### 7.2 Rust 插件 Trait

```rust
// crates/fontmin_plugin/src/hook.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginOrder {
    Pre,
    Normal,
    Post,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginKind {
    Loader,
    Transform,
    Generator,
    Reporter,
}
```

```rust
// crates/fontmin_plugin/src/lib.rs
use async_trait::async_trait;
use fontmin_core::Asset;
use fontmin_diagnostics::Result;

#[async_trait]
pub trait FontminPlugin: Send + Sync {
    fn name(&self) -> &'static str;

    fn order(&self) -> PluginOrder {
        PluginOrder::Normal
    }

    async fn build_start(&self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    async fn transform(&self, _ctx: &mut PluginContext, asset: Asset) -> Result<Vec<Asset>> {
        Ok(vec![asset])
    }

    async fn generate_bundle(&self, _ctx: &mut PluginContext, _assets: &mut Vec<Asset>) -> Result<()> {
        Ok(())
    }

    async fn build_end(&self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }
}
```

### 7.3 Pipeline 执行模型

```txt
load inputs
  -> detect format
  -> normalize asset metadata
  -> plugin.build_start
  -> for each asset:
       plugin.transform(pre)
       plugin.transform(normal)
       plugin.transform(post)
  -> plugin.generate_bundle
  -> emit files
  -> plugin.build_end
```

### 7.4 插件返回多 Asset

转换插件一般会 clone 原始字体再输出目标格式，比如当前 Fontmin 的 `ttf2woff({ clone: true })` 会保留原文件并输出 woff。因此 Rust transform 应返回 `Vec<Asset>`。

```rust
pub struct Ttf2WoffPlugin {
    pub options: WoffOptions,
    pub clone: bool,
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

        let mut outputs = Vec::new();
        if self.clone {
            outputs.push(asset.clone());
        }

        let mut woff = asset.clone();
        woff.contents = fontmin_woff::encode_ttf_to_woff(&asset.contents, self.options.clone())?;
        woff.format = FontFormat::Woff;
        woff.rename_ext("woff");
        woff.meta.generated_by.push(self.name().into());
        outputs.push(woff);

        Ok(outputs)
    }
}
```

### 7.5 JS/TS 插件 API

JS 插件只建议工作在 file-level / asset-level，不做 glyph-level 热路径。

```ts
export interface FontminPlugin {
  name: string
  enforce?: 'pre' | 'post'
  buildStart?(ctx: PluginContext): MaybePromise<void>
  transform?(
    asset: FontAsset,
    ctx: PluginContext,
  ): MaybePromise<FontAsset | FontAsset[] | null | undefined>
  generateBundle?(assets: FontAsset[], ctx: PluginContext): MaybePromise<void>
  buildEnd?(ctx: PluginContext): MaybePromise<void>
}

export interface FontAsset {
  path: string
  contents: Uint8Array
  format: FontFormat
  sourceFormat: FontFormat
  meta: Record<string, unknown>
}
```

JS 插件示例：

```ts
import { definePlugin } from 'fontmin-rs'

export default definePlugin({
  name: 'rename-to-hash',
  transform(asset) {
    if (asset.format === 'woff2') {
      asset.path = asset.path.replace(
        /\.woff2$/,
        `.${hash(asset.contents)}.woff2`,
      )
    }
    return asset
  },
})
```

### 7.6 内置插件工厂

```ts
import { glyph, ttf2woff, ttf2woff2, ttf2svg, css } from 'fontmin-rs/plugins'

export default {
  input: ['fonts/*.ttf'],
  outDir: 'dist/fonts',
  plugins: [
    glyph({ text: '天地玄黄', preserveHinting: false }),
    ttf2woff({ clone: true, deflate: true }),
    ttf2woff2({ clone: true }),
    css({ fontPath: './', base64: false }),
  ],
}
```

---

## 8. 配置设计

### 8.1 配置文件

支持以下文件：

```txt
fontmin.config.ts
fontmin.config.mts
fontmin.config.mjs
fontmin.config.cjs
fontmin.config.json
fontmin.config.jsonc
```

### 8.2 完整配置示例

```ts
// fontmin.config.ts
import { defineConfig } from 'fontmin-rs'
import { glyph, ttf2woff, ttf2woff2, css } from 'fontmin-rs/plugins'

export default defineConfig({
  cwd: process.cwd(),
  input: ['assets/fonts/*.ttf'],
  outDir: 'dist/fonts',
  clean: true,
  preserveOriginal: true,
  parallel: {
    threads: 'auto',
    perFile: true,
  },
  cache: {
    enabled: true,
    dir: 'node_modules/.cache/fontmin-rs',
  },
  subset: {
    text: '天地玄黄 宇宙洪荒',
    basicText: false,
    preserveHinting: false,
    trim: true,
    keepLayout: 'conservative',
  },
  outputs: [
    { format: 'ttf', clone: true },
    { format: 'woff', clone: true },
    { format: 'woff2', clone: true },
    { format: 'css', clone: false },
  ],
  css: {
    fontPath: './',
    base64: false,
    glyph: true,
    iconPrefix: 'icon',
    fontFamily: 'myfont',
    local: true,
    fontDisplay: 'swap',
  },
  plugins: [glyph(), ttf2woff({ deflate: true }), ttf2woff2(), css()],
  diagnostics: {
    level: 'warn',
    pretty: true,
    failOnWarning: false,
  },
})
```

### 8.3 配置归一化优先级

```txt
默认值
  < 配置文件
  < CLI flags
  < JS API 直接传入 options
```

### 8.4 Fontmin 兼容 preset

当前 Fontmin CLI 默认顺序大致是：`otf2ttf -> glyph -> ttf2eot -> ttf2svg -> ttf2woff -> ttf2woff2 -> css`。`fontmin-rs` 可以提供：

```ts
import { fontminCompatPreset } from 'fontmin-rs/presets'

export default defineConfig({
  input: ['fonts/*'],
  outDir: 'build',
  presets: [
    fontminCompatPreset({
      text: '天地玄黄',
      deflateWoff: true,
      cssGlyph: true,
    }),
  ],
})
```

---

## 9. Node.js API 设计

### 9.1 主 API

```ts
export async function optimize(config: FontminConfig): Promise<FontAsset[]>
export function subsetTtf(
  input: Buffer | Uint8Array,
  options: SubsetOptions,
): Buffer
export function convert(
  input: Buffer | Uint8Array,
  options: ConvertOptions,
): Buffer
export function inspect(input: Buffer | Uint8Array): FontInfo
export function defineConfig(config: FontminConfig): FontminConfig
export function definePlugin(plugin: FontminPlugin): FontminPlugin
```

### 9.2 使用示例

```ts
import { optimize } from 'fontmin-rs'
import { glyph, ttf2woff2, css } from 'fontmin-rs/plugins'

const files = await optimize({
  input: ['fonts/*.ttf'],
  outDir: 'dist/fonts',
  plugins: [
    glyph({ text: '你好，世界' }),
    ttf2woff2(),
    css({ fontPath: './' }),
  ],
})

console.log(files.map(file => file.path))
```

### 9.3 低层 API

```ts
import { subsetTtf, ttfToWoff2 } from 'fontmin-rs/native'
import { readFile, writeFile } from 'node:fs/promises'

const input = await readFile('SourceHanSansCN.ttf')
const subset = subsetTtf(input, {
  text: '首页登录注册',
  preserveHinting: false,
})
const woff2 = ttfToWoff2(subset)
await writeFile('dist/app.woff2', woff2)
```

### 9.4 Fontmin class 兼容 API

```ts
import Fontmin from 'fontmin-rs/compat'

const fontmin = new Fontmin()
  .src('fonts/*.ttf')
  .use(Fontmin.glyph({ text: '天地玄黄' }))
  .use(Fontmin.ttf2woff2())
  .use(Fontmin.css({ fontPath: './' }))
  .dest('build')

await fontmin.runAsync()
```

兼容层不建议完全模拟 vinyl stream 的所有细节，只保证主流 `.src().use().dest().run()` 用法。对依赖 `through2` stream 自定义插件的历史项目，提供单独 `fontmin-rs/compat-stream`，并标记为 legacy。

---

## 10. napi-rs 绑定设计

### 10.1 `napi/fontmin/Cargo.toml`

```toml
[package]
name = "fontmin_napi"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true

[lib]
crate-type = ["cdylib"]
path = "src/lib.rs"

[dependencies]
fontmin = { workspace = true }
fontmin_config = { workspace = true }
fontmin_core = { workspace = true }
fontmin_napi_shared = { workspace = true }

napi = { workspace = true, features = ["async", "tokio_rt", "serde-json"] }
napi-derive = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }

[build-dependencies]
napi-build = { workspace = true }
```

### 10.2 `napi/fontmin/package.json`

```json
{
  "name": "fontmin-rs",
  "version": "0.1.0",
  "description": "Fast font subsetter and converter written in Rust.",
  "type": "module",
  "main": "src-js/index.js",
  "types": "src-js/index.d.ts",
  "bin": {
    "fontmin-rs": "src-js/cli.js"
  },
  "exports": {
    ".": "./src-js/index.js",
    "./native": "./src-js/native.js",
    "./plugins": "./src-js/plugins.js",
    "./compat": "./src-js/compat.js",
    "./package.json": "./package.json"
  },
  "files": ["src-js", "!src-js/*.node"],
  "scripts": {
    "build-dev": "napi build --esm --platform --js bindings.js --dts index.d.ts --output-dir src-js",
    "build": "pnpm run build-dev --release",
    "build-npm-dir": "napi create-npm-dirs --npm-dir ../../npm && napi artifacts --npm-dir ../../npm --output-dir src-js",
    "prepublishOnly": "napi pre-publish -t npm --no-gh-release",
    "test": "vitest --dir test",
    "bench": "vitest bench --run bench"
  },
  "napi": {
    "binaryName": "fontmin_rs",
    "packageName": "@fontmin-rs/binding",
    "targets": [
      "x86_64-apple-darwin",
      "aarch64-apple-darwin",
      "x86_64-pc-windows-msvc",
      "aarch64-pc-windows-msvc",
      "x86_64-unknown-linux-gnu",
      "x86_64-unknown-linux-musl",
      "aarch64-unknown-linux-gnu",
      "aarch64-unknown-linux-musl"
    ]
  },
  "engines": {
    "node": ">=20.19.0"
  }
}
```

### 10.3 Rust binding

```rust
// napi/fontmin/src/lib.rs
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi(object)]
pub struct JsSubsetOptions {
    pub text: Option<String>,
    pub unicodes: Option<Vec<u32>>,
    pub basic_text: Option<bool>,
    pub preserve_hinting: Option<bool>,
    pub trim: Option<bool>,
}

#[napi]
pub fn subset_ttf(input: Buffer, options: JsSubsetOptions) -> napi::Result<Buffer> {
    let options = fontmin_napi_shared::options::subset_options_from_js(options)?;
    let output = fontmin::subset_ttf(&input, options)
        .map_err(fontmin_napi_shared::error::to_napi_error)?;
    Ok(output.into())
}

#[napi]
pub fn ttf_to_woff(input: Buffer, options_json: Option<String>) -> napi::Result<Buffer> {
    let options = fontmin_napi_shared::options::parse_woff_options(options_json)?;
    let output = fontmin_woff::encode_ttf_to_woff(&input, options)
        .map_err(fontmin_napi_shared::error::to_napi_error)?;
    Ok(output.into())
}

#[napi]
pub async fn optimize(config_json: String) -> napi::Result<Vec<JsAsset>> {
    let config: fontmin_config::FontminConfig = serde_json::from_str(&config_json)
        .map_err(|err| napi::Error::from_reason(err.to_string()))?;

    let assets = fontmin::optimize(config)
        .await
        .map_err(fontmin_napi_shared::error::to_napi_error)?;

    Ok(assets.into_iter().map(JsAsset::from).collect())
}
```

---

## 11. CLI 设计

### 11.1 命令结构

```txt
fontmin-rs <input...> [outDir]
fontmin-rs build [input...] -o dist --text "你好"
fontmin-rs subset input.ttf -o output.ttf --text "你好"
fontmin-rs convert input.ttf -f woff2 -o output.woff2
fontmin-rs inspect input.ttf --json
fontmin-rs init
fontmin-rs doctor
fontmin-rs bench input.ttf --text-file chars.txt
```

### 11.2 兼容 Fontmin CLI

保留旧参数：

```txt
-t, --text              require glyphs by text
-b, --basic-text        require glyphs with base chars
-d, --deflate-woff      deflate woff
--font-family           font-family for @font-face CSS
--css-glyph             generate class for each glyph
-T, --show-time         show time fontmin cost
```

新增参数：

```txt
--formats ttf,woff2,css
--no-original
--cache
--no-cache
--threads auto|1|4
--config fontmin.config.ts
--preset compat|modern-web|iconfont
--text-file chars.txt
--json
--silent
```

### 11.3 `apps/fontmin/src/main.rs`

```rust
fn main() -> miette::Result<()> {
    fontmin_diagnostics::install_reporter();
    let cli = fontmin_app::cli::parse();
    let code = fontmin_app::commands::run(cli)?;
    std::process::exit(code as i32);
}
```

---

## 12. 功能设计

### 12.1 字体裁剪

能力：

- `text` 字符串；
- `textFile` 从文件读取字符；
- `unicodes` 指定 code points；
- `basicText` 加入常用英文、数字、标点；
- 自动去重；
- 复合字形依赖追踪；
- 可选保留 hinting；
- 可选保留 OpenType layout；
- 输出裁剪报告。

报告示例：

```json
{
  "inputGlyphs": 28000,
  "outputGlyphs": 356,
  "inputSize": 14231040,
  "outputSize": 128420,
  "reduction": "99.10%",
  "missing": ["𠮷"]
}
```

### 12.2 格式转换

| 转换              | 首版优先级 | 说明                   |
| ----------------- | ---------: | ---------------------- |
| TTF -> TTF subset |         P0 | 第一版必须完成         |
| TTF -> WOFF       |         P0 | Web 常用               |
| TTF -> WOFF2      |      P0/P1 | Web 最常用，但实现复杂 |
| TTF -> EOT        |         P2 | 兼容 legacy            |
| TTF -> SVG font   |         P2 | 使用率低               |
| WOFF -> TTF       |         P2 | inspect/兼容场景       |
| WOFF2 -> TTF      |         P2 | debug/迁移场景         |
| OTF -> TTF        |         P3 | 难度高                 |
| SVG font -> TTF   |         P3 | 兼容 Fontmin           |
| SVG icons -> TTF  |         P1 | iconfont 场景很有价值  |

### 12.3 CSS 生成

支持：

- `@font-face`；
- 多格式 src；
- base64 inline；
- `font-display`；
- icon glyph class；
- 自定义 class prefix；
- 自定义 template；
- 输出 `.css` / `.scss` / `.less`。

### 12.4 缓存

缓存 key：

```txt
hash(input bytes)
+ fontmin-rs version
+ plugin versions
+ subset text/unicodes
+ output formats
+ options hash
```

缓存目录：

```txt
node_modules/.cache/fontmin-rs/
  v1/
    ab/cd/<hash>.ttf
    ab/cd/<hash>.woff2
    index.json
```

### 12.5 并行

推荐并行粒度：

- 多文件并行：默认开启；
- 单字体内部 subset：先不并行，避免复杂度；
- WOFF/WOFF2 压缩：可并行；
- JS plugin：不跨线程执行，避免 N-API 复杂性。

### 12.6 Diagnostics

错误需要可读：

```txt
error[FMN1001]: unsupported font format
  input: fonts/foo.otf
  detected: otf
  plugin: fontmin:glyph
  help: enable otf2ttf plugin before glyph, or convert the font manually.
```

错误分类：

```rust
pub enum FontminErrorKind {
    Io,
    Config,
    UnsupportedFormat,
    InvalidFont,
    MissingGlyph,
    ConvertFailed,
    PluginFailed,
    NapiBridgeFailed,
}
```

---

## 13. JS facade 设计

### 13.1 `packages/fontmin/src/index.ts`

```ts
export { optimize, subsetTtf, convert, inspect } from './native'
export { defineConfig } from './config'
export { definePlugin } from './plugins'
export * from './types'
export { default } from './compat/fontmin-class'
```

### 13.2 Native loader

```ts
// packages/fontmin/src/native.ts
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)

let binding: typeof import('../napi')
try {
  binding = require('@fontmin-rs/binding')
} catch (error) {
  throw new Error(
    'Failed to load fontmin-rs native binding. Please reinstall fontmin-rs or use the wasm fallback.',
    { cause: error },
  )
}

export function subsetTtf(input: Uint8Array, options: SubsetOptions = {}) {
  return binding.subsetTtf(input, options)
}
```

### 13.3 插件工厂

```ts
export function glyph(options: GlyphOptions = {}): FontminPlugin {
  return {
    name: 'fontmin:glyph',
    native: {
      kind: 'builtin',
      name: 'glyph',
      options,
    },
  }
}

export function ttf2woff2(options: Ttf2Woff2Options = {}): FontminPlugin {
  return {
    name: 'fontmin:ttf2woff2',
    native: {
      kind: 'builtin',
      name: 'ttf2woff2',
      options,
    },
  }
}
```

---

## 14. 兼容 Fontmin 的插件设计

当前 Fontmin 使用：

```js
new Fontmin()
  .src('fonts/*.ttf')
  .use(Fontmin.glyph({ text: '天地玄黄' }))
  .use(Fontmin.ttf2woff2())
  .dest('build')
  .run(cb)
```

兼容类：

```ts
export default class FontminCompat {
  private input: Array<string | Buffer> = []
  private outputDir?: string
  private plugins: FontminPlugin[] = []

  src(file: string | string[] | Buffer, options?: SourceOptions) {
    this.input = Array.isArray(file) ? file : [file]
    return this
  }

  dest(dir: string) {
    this.outputDir = dir
    return this
  }

  use(plugin: FontminPlugin) {
    this.plugins.push(plugin)
    return this
  }

  async runAsync() {
    return optimize({
      input: this.input,
      outDir: this.outputDir,
      plugins: this.plugins,
      compat: true,
    })
  }

  run(cb: (err: Error | null, files?: FontAsset[]) => void) {
    this.runAsync().then(files => cb(null, files), cb)
  }

  static glyph = glyph
  static ttf2woff = ttf2woff
  static ttf2woff2 = ttf2woff2
  static ttf2eot = ttf2eot
  static ttf2svg = ttf2svg
  static svg2ttf = svg2ttf
  static svgs2ttf = svgs2ttf
  static otf2ttf = otf2ttf
  static css = css
}
```

---

## 15. 内置插件设计清单

### 15.1 `glyph`

```ts
interface GlyphOptions {
  text?: string
  textFile?: string
  unicodes?: number[]
  basicText?: boolean
  preserveHinting?: boolean
  hinting?: boolean // Fontmin alias
  trim?: boolean
  keepLayout?: 'drop' | 'conservative' | 'preserve'
  clone?: boolean
}
```

Fontmin 兼容：`hinting: false` 映射到 `preserveHinting: false`。

### 15.2 `ttf2woff`

```ts
interface Ttf2WoffOptions {
  clone?: boolean
  deflate?: boolean
  compressionLevel?: number
}
```

### 15.3 `ttf2woff2`

```ts
interface Ttf2Woff2Options {
  clone?: boolean
  quality?: number
  fallback?: 'native' | 'wasm' | 'js' | 'auto'
}
```

### 15.4 `ttf2eot`

```ts
interface Ttf2EotOptions {
  clone?: boolean
}
```

### 15.5 `ttf2svg`

```ts
interface Ttf2SvgOptions {
  clone?: boolean
  pretty?: boolean
  metadata?: boolean
}
```

### 15.6 `svg2ttf`

```ts
interface Svg2TtfOptions {
  clone?: boolean
  hinting?: boolean
  normalize?: boolean
}
```

### 15.7 `svgs2ttf`

```ts
interface Svgs2TtfOptions {
  fontName: string
  startUnicode?: number
  normalize?: boolean
  ascent?: number
  descent?: number
  fixedWidth?: boolean
}
```

### 15.8 `otf2ttf`

```ts
interface Otf2TtfOptions {
  clone?: boolean
  hinting?: boolean
  subset?: GlyphOptions
  fallback?: 'unsupported' | 'fonttools' | 'auto'
}
```

### 15.9 `css`

```ts
interface CssOptions {
  fontPath?: string
  base64?: boolean
  glyph?: boolean
  iconPrefix?: string
  fontFamily?: string | ((info: FontInfo) => string)
  asFileName?: boolean
  local?: boolean
  fontDisplay?: 'auto' | 'block' | 'swap' | 'fallback' | 'optional'
  target?: 'css' | 'scss' | 'less'
}
```

注意：`fontFamily` 如果是函数，只能在 JS 层执行，不能直接下沉到 Rust。处理方式：

```txt
Rust 生成 FontInfo
  -> JS 调用 fontFamily 函数
  -> 把结果传回 CSS generator
```

---

## 16. apps/fontmin 设计

`apps/fontmin` 是 Rust CLI，不直接作为 npm 包发布，但 npm bin 可以调用它对应的 native API 或直接打包 binary。

### 16.1 `apps/fontmin/Cargo.toml`

```toml
[package]
name = "fontmin"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
publish = false

[[bin]]
name = "fontmin-rs"
path = "src/main.rs"

[dependencies]
fontmin = { workspace = true }
fontmin_config = { workspace = true }
fontmin_diagnostics = { workspace = true }
fontmin_fs = { workspace = true }

bpaf = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
serde_json = { workspace = true }
miette = { workspace = true }
```

### 16.2 CLI 子命令代码草图

```rust
#[derive(Debug, Clone, bpaf::Bpaf)]
#[bpaf(options, version)]
pub enum Command {
    Build(BuildCommand),
    Subset(SubsetCommand),
    Convert(ConvertCommand),
    Inspect(InspectCommand),
    Init(InitCommand),
    Doctor(DoctorCommand),
}
```

```rust
pub async fn run(command: Command) -> miette::Result<i32> {
    match command {
        Command::Build(cmd) => commands::build::run(cmd).await,
        Command::Subset(cmd) => commands::subset::run(cmd).await,
        Command::Convert(cmd) => commands::convert::run(cmd).await,
        Command::Inspect(cmd) => commands::inspect::run(cmd).await,
        Command::Init(cmd) => commands::init::run(cmd).await,
        Command::Doctor(cmd) => commands::doctor::run(cmd).await,
    }
}
```

---

## 17. 测试设计

### 17.1 测试分层

```txt
Rust unit tests
  - table parser
  - subset planner
  - checksum
  - format detect

Rust integration tests
  - TTF subset output valid
  - TTF -> WOFF
  - TTF -> WOFF2
  - CSS generation

Node API tests
  - optimize(config)
  - subsetTtf(buffer)
  - plugins
  - compat Fontmin class

CLI tests
  - fontmin-rs subset
  - fontmin-rs convert
  - fontmin-rs inspect --json

Conformance tests
  - compare against Fontmin output where reasonable
  - browser load test
  - font metadata consistency

Fuzz tests
  - malformed font corpus
  - cmap/glyf/loca table parser
```

### 17.2 字体 fixtures

```txt
fixtures/fonts/
  ttf/
    roboto.ttf
    source-han-sans-subset.ttf
    iconfont.ttf
  otf/
    sample-cff.otf
  woff/
  woff2/
  svg-font/
  icon-svg/
```

### 17.3 输出校验

不要只比较字节相等，因为不同 writer 可能生成不同但合法的表顺序或 checksum。建议校验：

- 能被 parser 重新读取；
- cmap 包含目标字符；
- 不包含非目标字符；
- glyph count 符合预期；
- 浏览器能加载；
- WOFF/WOFF2 可 decode 回 TTF；
- CSS 中 URL 和 format 正确。

---

## 18. Benchmark 设计

### 18.1 对比对象

- `fontmin@2`；
- `fonteditor-core` 直接 API；
- `fontmin-rs` native；
- `fontmin-rs` wasm fallback；
- 可选 Python `fonttools subset` 作为参考。

### 18.2 benchmark 场景

```txt
1. 小英文字体 subset 100 chars
2. 中型 iconfont subset 500 icons
3. 大型中文字体 subset 300 chars
4. 大型中文字体 subset 3000 chars
5. TTF -> WOFF
6. TTF -> WOFF2
7. subset + woff2 + css 完整 pipeline
8. 100 个字体文件批量处理
```

### 18.3 指标

```txt
耗时 p50/p95
峰值内存
输出大小
安装体积
native binding 加载时间
首次执行时间
缓存命中时间
```

---

## 19. 发布与 CI 设计

### 19.1 GitHub Actions jobs

```yaml
jobs:
  rust-test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - cargo test --workspace --all-features
      - cargo clippy --workspace --all-targets --all-features

  node-test:
    strategy:
      matrix:
        node: [20, 22, 24]
    steps:
      - pnpm install --frozen-lockfile
      - pnpm build
      - pnpm test

  build-native:
    strategy:
      matrix:
        target:
          - x86_64-apple-darwin
          - aarch64-apple-darwin
          - x86_64-pc-windows-msvc
          - x86_64-unknown-linux-gnu
          - x86_64-unknown-linux-musl
          - aarch64-unknown-linux-gnu
    steps:
      - pnpm --filter fontmin-rs build --target ${{ matrix.target }} --release
      - pnpm --filter fontmin-rs artifacts

  release:
    needs: [rust-test, node-test, build-native]
    steps:
      - pnpm --filter fontmin-rs prepublishOnly
      - pnpm publish -r
```

### 19.2 npm 包结构

```txt
fontmin-rs
  depends on optional platform packages

@fontmin-rs/binding-darwin-arm64
@fontmin-rs/binding-darwin-x64
@fontmin-rs/binding-linux-x64-gnu
@fontmin-rs/binding-linux-x64-musl
@fontmin-rs/binding-linux-arm64-gnu
@fontmin-rs/binding-win32-x64-msvc
```

### 19.3 fallback 策略

```txt
native binding 加载成功 -> 使用 native
native binding 缺失且安装了 wasm -> 使用 wasm
native/wasm 都失败 -> 对低层 API 抛错；对 compat API 提示安装问题
```

---

## 20. 开发里程碑

### v0.1：最小可用 native subset

- monorepo 搭建；
- `subsetTtf(buffer, { text })`；
- `fontmin-rs subset input.ttf -t text -o output.ttf`；
- napi binding；
- 基础测试 fixtures。

### v0.2：Node API 与 glyph 插件

- `optimize(config)`；
- `glyph()` 插件；
- `FontminCompat`；
- 配置文件加载；
- benchmark 对比 Fontmin。

### v0.3：WOFF / CSS

- `ttf2woff`；
- `css`；
- `modernWeb` preset；
- cache。

### v0.4：WOFF2

- `ttf2woff2`；
- decode/validate；
- wasm fallback 初版。

### v0.5：iconfont

- `svgs2ttf`；
- glyph class CSS；
- SVG path normalize。

### v1.0：稳定版

- Fontmin 主流 API 兼容；
- 完整 CLI；
- 多平台 npm 发布；
- browser load tests；
- 文档和迁移指南。

---

## 21. 关键风险与建议

### 21.1 不要首版追求 100% Fontmin 兼容

当前 Fontmin 有 stream/vinyl 生态，历史插件可能依赖 Node stream 行为。`fontmin-rs` 应优先兼容主流用法，而不是兼容所有 stream 插件。

### 21.2 不要首版直接挑战 OTF/CFF 完整转换

OTF -> TTF 难度高。首版可以保留诊断或 fallback，而不是承诺完整支持。

### 21.3 WOFF2 要谨慎

WOFF2 涉及 Brotli 和 Google woff2 生态。首版可以先实现接口和 fallback，再逐步 native 化。

### 21.4 插件边界要清晰

JS 插件不应进入 glyph 级热路径。推荐只允许 JS 插件处理 asset-level transform。

### 21.5 输出正确性比性能更重要

字体转换容易出现“文件能生成，但浏览器/系统渲染异常”。必须建立 conformance、browser load 和 corpus tests。

---

## 22. 推荐的最小初始目录

如果不想一开始建太多 crate，可以先落地这个最小结构：

```txt
fontmin-rs/
  Cargo.toml
  package.json
  pnpm-workspace.yaml

  apps/fontmin/
    Cargo.toml
    src/main.rs

  crates/fontmin/
    Cargo.toml
    src/lib.rs

  crates/fontmin_core/
    Cargo.toml
    src/{lib.rs,asset.rs,format.rs,text.rs}

  crates/fontmin_subset/
    Cargo.toml
    src/{lib.rs,plan.rs,subset.rs}

  crates/fontmin_ttf/
    Cargo.toml
    src/{lib.rs,read.rs,write.rs,checksum.rs}

  crates/fontmin_detect/
    Cargo.toml
    src/lib.rs

  crates/fontmin_diagnostics/
    Cargo.toml
    src/lib.rs

  napi/fontmin/
    Cargo.toml
    package.json
    build.rs
    src/lib.rs
    src-js/index.ts

  packages/fontmin/
    package.json
    src/index.ts
    src/plugins.ts
    src/compat.ts
```

等 `glyph subset` 和 napi API 跑通之后，再拆 `woff`、`woff2`、`css`、`svg`、`otf`。

---

## 23. 参考资料

- Rolldown：`https://github.com/rolldown/rolldown`
- OXC：`https://github.com/oxc-project/oxc`
- Fontmin：`https://github.com/ecomfe/fontmin`
- fonteditor-core：`https://github.com/kekee000/fonteditor-core`
- napi-rs：`https://github.com/napi-rs/napi-rs`
- Google Fontations：`https://github.com/googlefonts/fontations`

---

## 24. 总结

`fontmin-rs` 最合理的工程路线是：

```txt
OXC 风格 workspace 分层
+ Rolldown 风格 napi 多平台发布
+ Fontmin 兼容 API
+ Rust 字体处理核心
+ JS/TS 插件和配置生态
```

首要目标不是“重写 Fontmin 的所有历史行为”，而是先做出一个可靠、快速、可发布的：

```txt
TTF subset + WOFF/WOFF2 输出 + CSS 生成 + Node API + CLI
```

之后再逐步扩展到 SVG iconfont、OTF、EOT、WASM/browser fallback 和完整插件生态。
