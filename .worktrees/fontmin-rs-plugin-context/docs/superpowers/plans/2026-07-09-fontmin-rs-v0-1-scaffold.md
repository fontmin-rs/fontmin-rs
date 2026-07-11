# Fontmin Rs V0.1 Scaffold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the design document's v0.1/minimum initial `fontmin-rs` implementation: Rust workspace, core model crates, real TTF subsetting, Rust CLI, napi binding, JS package facade, fixtures, tests, and CI gates.

**Architecture:** Use an OXC-style workspace split into `apps/*`, `crates/*`, `napi/*`, and `packages/*`. Rust owns font data models, diagnostics, format detection, and the native subset path; napi exposes the native functions; the JS package provides the public typed facade, plugin factories, config helpers, and Fontmin-compatible class shell.

**Tech Stack:** Rust 2024, Cargo workspace resolver 3, napi-rs 3, `font-subset` 0.1.0 for the first TTF subset engine, bpaf 0.9.26 for CLI parsing, TypeScript ESM, tsdown, Vitest, pnpm 11.10.0.

## Global Constraints

- Positioning from the spec: `fontmin-rs` is "一个 Rust 实现的字体处理核心引擎，提供 Node.js napi-rs 绑定、CLI 应用、JS/TS 插件 API，并兼容 Fontmin 的主流使用体验。"
- Root workspace must follow the spec shape: `members = ["apps/*", "crates/*", "napi/*", "tasks/*"]`; v0.1 omits `tasks/*` until a task crate exists and uses `members = ["apps/*", "crates/*", "napi/*"]` to keep Cargo green.
- Rust edition must be `2024`.
- Rust version floor must be `1.88.0`.
- Workspace resolver must be `"3"`.
- Root package remains managed by `pnpm@11.10.0`, matching the current repository.
- Use the design doc's minimum initial directory first: `apps/fontmin`, `crates/fontmin`, `crates/fontmin_core`, `crates/fontmin_subset`, `crates/fontmin_detect`, `crates/fontmin_diagnostics`, `napi/fontmin`, `packages/fontmin`.
- Scope v0.1 to the design doc milestone: monorepo setup, `subsetTtf(buffer, { text })`, `fontmin-rs subset input.ttf -t text -o output.ttf`, napi binding, and basic fixture tests.
- OTF conversion, WOFF, WOFF2, EOT, SVG, CSS generation, cache, full pipeline, custom JS plugin execution, and multi-platform npm publish artifacts are out of this v0.1 plan and require follow-up plans matching the design doc milestones v0.2 through v1.0.
- Shell commands must be run with `rtk` in this repository, except `pnpm typecheck`.
- External API references checked before writing this plan: `font-subset` docs at `https://docs.rs/font-subset`, `bpaf` docs at `https://docs.rs/bpaf`, and `napi` docs at `https://docs.rs/napi`.

---

## File Structure

- Create `Cargo.toml`: Rust workspace package metadata, lints, and shared dependencies.
- Create `rust-toolchain.toml`: stable channel with rustfmt and clippy components.
- Create `taplo.toml`: TOML formatting rules for workspace config files.
- Modify `package.json`: convert root package into a private monorepo coordinator while preserving the current pnpm/tooling style.
- Modify `pnpm-workspace.yaml`: include `packages/*`, `napi/*`, `npm/*`, `wasm/*`, `tasks/*`, and `examples/*`.
- Create `crates/fontmin_diagnostics`: shared `FontminError`, `FontminErrorKind`, and `Result<T>`.
- Create `crates/fontmin_core`: `FontFormat`, `OutputFormat`, `Asset`, `AssetMeta`, and text/codepoint helpers.
- Create `crates/fontmin_detect`: magic-byte based format detection.
- Create `crates/fontmin_config`: serializable config models and defaults used by Rust, CLI, and napi.
- Create `crates/fontmin_subset`: `SubsetOptions`, `LayoutSubsetMode`, and `subset_ttf`.
- Create `crates/fontmin`: public Rust facade re-exporting the first stable APIs.
- Create `apps/fontmin`: Rust CLI binary named `fontmin-rs` with `subset`, `inspect`, and `doctor`.
- Create `napi/fontmin`: napi-rs cdylib package named `@fontmin-rs/binding`.
- Create `packages/fontmin`: user-facing TypeScript package named `fontmin-rs`.
- Create `fixtures/fonts/ttf/roboto-regular.ttf`: reproducible Roboto test fixture downloaded with SHA-256 verification.
- Modify `.github/workflows/ci.yml`: add Rust format, clippy, and cargo tests to CI.
- Modify `README.md`: replace starter template copy with v0.1 usage and scope.
- Delete root `src/index.ts`, `tests/index.test.ts`, `tsdown.config.ts`, and `vitest.config.ts` after the package has moved to `packages/fontmin`.

---

### Task 1: Workspace Foundation

**Files:**

- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `taplo.toml`
- Modify: `package.json`
- Modify: `pnpm-workspace.yaml`

**Interfaces:**

- Consumes: current root Node starter package.
- Produces: a private JS/Rust monorepo root where later tasks can add Cargo crates and pnpm workspaces.

- [ ] **Step 1: Write the failing workspace smoke test**

Run:

```bash
rtk cargo metadata --format-version 1 --no-deps
```

Expected: FAIL with `could not find Cargo.toml`.

- [ ] **Step 2: Replace the root Cargo and toolchain configuration**

Create `Cargo.toml` with:

```toml
[workspace]
resolver = "3"
members = ["apps/*", "crates/*", "napi/*"]
exclude = ["wasm/*"]

[workspace.package]
version = "0.0.0"
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
fontmin_config = { path = "crates/fontmin_config" }
fontmin_core = { path = "crates/fontmin_core" }
fontmin_detect = { path = "crates/fontmin_detect" }
fontmin_diagnostics = { path = "crates/fontmin_diagnostics" }
fontmin_subset = { path = "crates/fontmin_subset" }

bpaf = { version = "0.9.26", features = ["derive", "bright-color"] }
font-subset = "0.1.0"
indexmap = { version = "2", features = ["serde"] }
miette = { version = "7", features = ["fancy"] }
napi = { version = "3.10.3", features = ["napi8", "serde-json", "tokio_rt"] }
napi-build = "2.3.2"
napi-derive = "3.5.9"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tempfile = "3"
thiserror = "2"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "fs"] }
```

Create `rust-toolchain.toml` with:

```toml
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
```

Create `taplo.toml` with:

```toml
[formatting]
align_comments = false
align_entries = false
array_auto_expand = true
compact_arrays = false
reorder_keys = false
```

- [ ] **Step 3: Replace the root pnpm workspace configuration**

Replace `pnpm-workspace.yaml` with:

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
  '@types/node': ^26.1.0
  tsdown: ^0.22.3
  typescript: ^6.0.0
  vitest: ^4.1.10

minimumReleaseAge: 0
ignoreWorkspaceRootCheck: true
packageManagerStrict: false
shellEmulator: true

allowBuilds:
  '@napi-rs/cli': true

minimumReleaseAgeExclude:
  - '@napi-rs/*'
  - '@emnapi/*'
  - fontmin-rs
```

Replace `package.json` with:

```json
{
  "name": "fontmin-rs-monorepo",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "pnpm --workspace-concurrency=1 --filter fontmin-rs build",
    "build:debug": "pnpm --filter @fontmin-rs/binding build:debug",
    "build:release": "pnpm --filter @fontmin-rs/binding build:release",
    "check": "pnpm run format:check && pnpm run lint && pnpm run typecheck && pnpm run test",
    "format": "cargo fmt --all && oxfmt .",
    "format:check": "cargo fmt --all --check && oxfmt --check .",
    "lint": "cargo clippy --workspace --all-targets --all-features && oxlint .",
    "test": "pnpm --workspace-concurrency=1 --filter fontmin-rs test && cargo test --workspace",
    "typecheck": "pnpm --filter fontmin-rs typecheck"
  },
  "devDependencies": {
    "@napi-rs/cli": "catalog:",
    "@ntnyq/tsconfig": "^3.1.0",
    "@types/node": "catalog:",
    "@typescript/native-preview": "^7.0.0-dev.20260707.2",
    "bumpp": "^11.1.0",
    "husky": "^9.1.7",
    "nano-staged": "^1.0.2",
    "npm-run-all2": "^9.0.2",
    "oxfmt": "^0.58.0",
    "oxlint": "^1.73.0",
    "tsdown": "catalog:",
    "vitest": "catalog:"
  },
  "nano-staged": {
    "*.{js,ts,mjs,tsx}": "oxlint --fix --no-error-on-unmatched-pattern",
    "*": "oxfmt --no-error-on-unmatched-pattern"
  },
  "packageManager": "pnpm@11.10.0"
}
```

- [ ] **Step 4: Run metadata to verify the expected next failure**

Run:

```bash
rtk cargo metadata --format-version 1 --no-deps
```

Expected: FAIL with missing workspace member directories such as `apps/*`, `crates/*`, or `napi/*`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml rust-toolchain.toml taplo.toml package.json pnpm-workspace.yaml
git commit -m "chore: set up rust and pnpm workspace"
```

---

### Task 2: Diagnostics, Core Model, and Format Detection

**Files:**

- Create: `crates/fontmin_diagnostics/Cargo.toml`
- Create: `crates/fontmin_diagnostics/src/lib.rs`
- Create: `crates/fontmin_core/Cargo.toml`
- Create: `crates/fontmin_core/src/lib.rs`
- Create: `crates/fontmin_core/src/asset.rs`
- Create: `crates/fontmin_core/src/format.rs`
- Create: `crates/fontmin_core/src/text.rs`
- Create: `crates/fontmin_detect/Cargo.toml`
- Create: `crates/fontmin_detect/src/lib.rs`

**Interfaces:**

- Consumes: workspace dependencies from Task 1.
- Produces:
  - `fontmin_diagnostics::Result<T>`
  - `fontmin_diagnostics::FontminError`
  - `fontmin_core::FontFormat`
  - `fontmin_core::OutputFormat`
  - `fontmin_core::Asset`
  - `fontmin_core::collect_chars(text: Option<&str>, unicodes: &[u32], basic_text: bool) -> Result<BTreeSet<char>>`
  - `fontmin_detect::detect_format(bytes: &[u8]) -> FontFormat`

- [ ] **Step 1: Write failing tests for text normalization and format detection**

Create `crates/fontmin_core/Cargo.toml` with only package metadata first:

```toml
[package]
name = "fontmin_core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description.workspace = true

[dependencies]
fontmin_diagnostics = { workspace = true }
indexmap = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[lints]
workspace = true
```

Create `crates/fontmin_core/src/lib.rs` with:

```rust
pub mod asset;
pub mod format;
pub mod text;

pub use asset::{Asset, AssetMeta};
pub use format::{FontFormat, OutputFormat};
pub use text::collect_chars;
```

Create `crates/fontmin_core/src/text.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::collect_chars;

    #[test]
    fn collects_text_and_unicode_values_once() {
        let chars = collect_chars(Some("abca"), &[0x4e2d], false).unwrap();
        let collected: String = chars.into_iter().collect();

        assert_eq!(collected, "abc中");
    }

    #[test]
    fn rejects_invalid_unicode_values() {
        let error = collect_chars(None, &[0x11_0000], false).unwrap_err();

        assert!(error.to_string().contains("invalid unicode code point"));
    }
}
```

Create `crates/fontmin_detect/Cargo.toml` with:

```toml
[package]
name = "fontmin_detect"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description.workspace = true

[dependencies]
fontmin_core = { workspace = true }

[lints]
workspace = true
```

Create `crates/fontmin_detect/src/lib.rs` with:

```rust
use fontmin_core::FontFormat;

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

fn looks_like_eot(bytes: &[u8]) -> bool {
    bytes.len() >= 12
        && (bytes[8..12] == [0x01, 0x00, 0x02, 0x00]
            || bytes[8..12] == [0x02, 0x00, 0x02, 0x00])
}

fn looks_like_svg_font(bytes: &[u8]) -> bool {
    let Ok(prefix) = std::str::from_utf8(&bytes[..bytes.len().min(512)]) else {
        return false;
    };

    let trimmed = prefix.trim_start();
    trimmed.starts_with("<svg") || trimmed.starts_with("<?xml") && trimmed.contains("<svg")
}

#[cfg(test)]
mod tests {
    use super::detect_format;
    use fontmin_core::FontFormat;

    #[test]
    fn detects_common_font_magic_bytes() {
        assert_eq!(detect_format(&[0x00, 0x01, 0x00, 0x00]), FontFormat::Ttf);
        assert_eq!(detect_format(b"OTTO"), FontFormat::Otf);
        assert_eq!(detect_format(b"wOFF1234"), FontFormat::Woff);
        assert_eq!(detect_format(b"wOF21234"), FontFormat::Woff2);
        assert_eq!(detect_format(b"<svg><font /></svg>"), FontFormat::Svg);
        assert_eq!(detect_format(b"plain text"), FontFormat::Unknown);
    }
}
```

Run:

```bash
rtk cargo test -p fontmin_core -p fontmin_detect
```

Expected: FAIL because `fontmin_diagnostics`, `FontFormat`, and `Asset` do not exist yet.

- [ ] **Step 2: Implement diagnostics**

Create `crates/fontmin_diagnostics/Cargo.toml` with:

```toml
[package]
name = "fontmin_diagnostics"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description.workspace = true

[dependencies]
miette = { workspace = true }
thiserror = { workspace = true }

[lints]
workspace = true
```

Create `crates/fontmin_diagnostics/src/lib.rs` with:

```rust
use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, FontminError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontminErrorKind {
    Io,
    Config,
    UnsupportedFormat,
    InvalidFont,
    ConvertFailed,
    PluginFailed,
    NapiBridgeFailed,
}

#[derive(Debug, Diagnostic, Error)]
pub enum FontminError {
    #[error("I/O error while accessing {path}: {source}")]
    #[diagnostic(code(fontmin::io))]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("configuration error: {message}")]
    #[diagnostic(code(fontmin::config))]
    Config { message: String },

    #[error("unsupported font format: {format}")]
    #[diagnostic(code(fontmin::unsupported_format))]
    UnsupportedFormat { format: String },

    #[error("invalid font data: {message}")]
    #[diagnostic(code(fontmin::invalid_font))]
    InvalidFont { message: String },

    #[error("conversion failed: {message}")]
    #[diagnostic(code(fontmin::convert_failed))]
    ConvertFailed { message: String },

    #[error("plugin failed: {plugin}: {message}")]
    #[diagnostic(code(fontmin::plugin_failed))]
    PluginFailed { plugin: String, message: String },

    #[error("napi bridge failed: {message}")]
    #[diagnostic(code(fontmin::napi_bridge_failed))]
    NapiBridgeFailed { message: String },
}

impl FontminError {
    pub fn kind(&self) -> FontminErrorKind {
        match self {
            Self::Io { .. } => FontminErrorKind::Io,
            Self::Config { .. } => FontminErrorKind::Config,
            Self::UnsupportedFormat { .. } => FontminErrorKind::UnsupportedFormat,
            Self::InvalidFont { .. } => FontminErrorKind::InvalidFont,
            Self::ConvertFailed { .. } => FontminErrorKind::ConvertFailed,
            Self::PluginFailed { .. } => FontminErrorKind::PluginFailed,
            Self::NapiBridgeFailed { .. } => FontminErrorKind::NapiBridgeFailed,
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    pub fn invalid_font(message: impl Into<String>) -> Self {
        Self::InvalidFont {
            message: message.into(),
        }
    }

    pub fn unsupported(format: impl Into<String>) -> Self {
        Self::UnsupportedFormat {
            format: format.into(),
        }
    }
}
```

- [ ] **Step 3: Implement core models and text helpers**

Create `crates/fontmin_core/src/format.rs` with:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

Create `crates/fontmin_core/src/asset.rs` with:

```rust
use std::path::PathBuf;

use indexmap::IndexMap;

use crate::FontFormat;

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
    pub fn new(path: PathBuf, contents: Vec<u8>, format: FontFormat) -> Self {
        Self {
            path,
            contents,
            format,
            source_format: format,
            meta: AssetMeta::default(),
        }
    }

    pub fn rename_ext(&mut self, ext: &str) {
        self.path.set_extension(ext.trim_start_matches('.'));
    }
}
```

Replace `crates/fontmin_core/src/text.rs` with:

```rust
use std::collections::BTreeSet;

use fontmin_diagnostics::{FontminError, Result};

const BASIC_TEXT: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,;:!?-_()[]{}'\"/\\@#$%^&*+=<>|`~";

pub fn collect_chars(
    text: Option<&str>,
    unicodes: &[u32],
    basic_text: bool,
) -> Result<BTreeSet<char>> {
    let mut chars = BTreeSet::new();

    if basic_text {
        chars.extend(BASIC_TEXT.chars());
    }

    if let Some(text) = text {
        chars.extend(text.chars());
    }

    for codepoint in unicodes {
        let Some(character) = char::from_u32(*codepoint) else {
            return Err(FontminError::config(format!(
                "invalid unicode code point: 0x{codepoint:x}",
            )));
        };
        chars.insert(character);
    }

    Ok(chars)
}

#[cfg(test)]
mod tests {
    use super::collect_chars;

    #[test]
    fn collects_text_and_unicode_values_once() {
        let chars = collect_chars(Some("abca"), &[0x4e2d], false).unwrap();
        let collected: String = chars.into_iter().collect();

        assert_eq!(collected, "abc中");
    }

    #[test]
    fn includes_basic_text_when_requested() {
        let chars = collect_chars(None, &[], true).unwrap();

        assert!(chars.contains(&'A'));
        assert!(chars.contains(&'z'));
        assert!(chars.contains(&'0'));
    }

    #[test]
    fn rejects_invalid_unicode_values() {
        let error = collect_chars(None, &[0x11_0000], false).unwrap_err();

        assert!(error.to_string().contains("invalid unicode code point"));
    }
}
```

- [ ] **Step 4: Run focused tests**

Run:

```bash
rtk cargo test -p fontmin_core -p fontmin_detect -p fontmin_diagnostics
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/fontmin_diagnostics crates/fontmin_core crates/fontmin_detect
git commit -m "feat: add core font models and detection"
```

---

### Task 3: Config Model

**Files:**

- Create: `crates/fontmin_config/Cargo.toml`
- Create: `crates/fontmin_config/src/lib.rs`
- Create: `crates/fontmin_config/src/config.rs`

**Interfaces:**

- Consumes:
  - `fontmin_core::OutputFormat`
  - `fontmin_subset::LayoutSubsetMode` will be created in Task 4; this task defines a config-local layout enum so Task 3 compiles independently.
- Produces:
  - `fontmin_config::FontminConfig`
  - `fontmin_config::SubsetConfig`
  - `fontmin_config::OutputConfig`
  - `fontmin_config::CssConfig`
  - `fontmin_config::PluginConfig`
  - `fontmin_config::DiagnosticsConfig`
  - `fontmin_config::ParallelConfig`
  - `fontmin_config::CacheConfig`

- [ ] **Step 1: Write failing config default tests**

Create `crates/fontmin_config/Cargo.toml` with:

```toml
[package]
name = "fontmin_config"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description.workspace = true

[dependencies]
fontmin_core = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[lints]
workspace = true
```

Create `crates/fontmin_config/src/lib.rs` with:

```rust
mod config;

pub use config::{
    CacheConfig, CssConfig, DiagnosticsConfig, FontminConfig, LayoutSubsetMode, OutputConfig,
    ParallelConfig, PluginConfig, SubsetConfig,
};
```

Create `crates/fontmin_config/src/config.rs` with only tests:

```rust
#[cfg(test)]
mod tests {
    use fontmin_core::OutputFormat;

    use super::{FontminConfig, OutputConfig};

    #[test]
    fn default_config_matches_fontmin_compat_outputs() {
        let config = FontminConfig::default();
        let formats: Vec<_> = config.outputs.iter().map(|output| output.format).collect();

        assert_eq!(
            formats,
            vec![
                OutputFormat::Eot,
                OutputFormat::Woff,
                OutputFormat::Woff2,
                OutputFormat::Svg,
                OutputFormat::Css,
            ],
        );
        assert_eq!(config.out_dir.as_deref(), Some("build"));
        assert!(config.preserve_original);
    }

    #[test]
    fn output_config_uses_clone_by_default() {
        let output = OutputConfig::format(OutputFormat::Woff2);

        assert_eq!(output.format, OutputFormat::Woff2);
        assert!(output.clone);
        assert!(output.file_name.is_none());
        assert!(output.ext.is_none());
    }
}
```

Run:

```bash
rtk cargo test -p fontmin_config
```

Expected: FAIL because config structs are not defined.

- [ ] **Step 2: Implement config models**

Replace `crates/fontmin_config/src/config.rs` with:

```rust
use fontmin_core::OutputFormat;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for FontminConfig {
    fn default() -> Self {
        Self {
            cwd: None,
            input: Vec::new(),
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
            plugins: Vec::new(),
            diagnostics: DiagnosticsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParallelConfig {
    pub threads: ThreadCount,
    pub per_file: bool,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            threads: ThreadCount::Auto,
            per_file: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThreadCount {
    Auto,
    Count(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheConfig {
    pub enabled: bool,
    pub dir: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            dir: "node_modules/.cache/fontmin-rs".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for SubsetConfig {
    fn default() -> Self {
        Self {
            text: None,
            text_file: None,
            unicodes: Vec::new(),
            basic_text: false,
            preserve_hinting: false,
            trim: true,
            keep_notdef: true,
            keep_layout: LayoutSubsetMode::Conservative,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutSubsetMode {
    Drop,
    Conservative,
    Preserve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputConfig {
    pub format: OutputFormat,
    pub clone: bool,
    pub file_name: Option<String>,
    pub ext: Option<String>,
}

impl OutputConfig {
    pub fn format(format: OutputFormat) -> Self {
        Self {
            format,
            clone: true,
            file_name: None,
            ext: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssConfig {
    pub font_path: String,
    pub base64: bool,
    pub glyph: bool,
    pub icon_prefix: String,
    pub font_family: Option<String>,
    pub as_file_name: bool,
    pub local: bool,
    pub font_display: String,
}

impl Default for CssConfig {
    fn default() -> Self {
        Self {
            font_path: "./".into(),
            base64: false,
            glyph: false,
            icon_prefix: "icon".into(),
            font_family: None,
            as_file_name: false,
            local: true,
            font_display: "swap".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfig {
    pub name: String,
    pub options: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsConfig {
    pub level: DiagnosticLevel,
    pub pretty: bool,
    pub fail_on_warning: bool,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self {
            level: DiagnosticLevel::Warn,
            pretty: true,
            fail_on_warning: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticLevel {
    Error,
    Warn,
    Info,
    Silent,
}

#[cfg(test)]
mod tests {
    use fontmin_core::OutputFormat;

    use super::{FontminConfig, OutputConfig};

    #[test]
    fn default_config_matches_fontmin_compat_outputs() {
        let config = FontminConfig::default();
        let formats: Vec<_> = config.outputs.iter().map(|output| output.format).collect();

        assert_eq!(
            formats,
            vec![
                OutputFormat::Eot,
                OutputFormat::Woff,
                OutputFormat::Woff2,
                OutputFormat::Svg,
                OutputFormat::Css,
            ],
        );
        assert_eq!(config.out_dir.as_deref(), Some("build"));
        assert!(config.preserve_original);
    }

    #[test]
    fn output_config_uses_clone_by_default() {
        let output = OutputConfig::format(OutputFormat::Woff2);

        assert_eq!(output.format, OutputFormat::Woff2);
        assert!(output.clone);
        assert!(output.file_name.is_none());
        assert!(output.ext.is_none());
    }
}
```

- [ ] **Step 3: Run focused tests**

Run:

```bash
rtk cargo test -p fontmin_config
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/fontmin_config
git commit -m "feat: add shared config model"
```

---

### Task 4: Native TTF Subset and Public Rust Facade

**Files:**

- Create: `fixtures/fonts/ttf/roboto-regular.ttf`
- Create: `fixtures/fonts/ttf/roboto-regular.ttf.sha256`
- Create: `crates/fontmin_subset/Cargo.toml`
- Create: `crates/fontmin_subset/src/lib.rs`
- Create: `crates/fontmin/Cargo.toml`
- Create: `crates/fontmin/src/lib.rs`

**Interfaces:**

- Consumes:
  - `fontmin_core::collect_chars(text: Option<&str>, unicodes: &[u32], basic_text: bool) -> Result<BTreeSet<char>>`
  - `fontmin_core::OutputFormat`
  - `fontmin_diagnostics::{FontminError, Result}`
- Produces:
  - `fontmin_subset::LayoutSubsetMode`
  - `fontmin_subset::SubsetOptions`
  - `fontmin_subset::subset_ttf(input: &[u8], options: SubsetOptions) -> Result<Vec<u8>>`
  - `fontmin::subset_ttf(input: &[u8], options: SubsetOptions) -> Result<Vec<u8>>`
  - `fontmin::convert(input: &[u8], target: OutputFormat) -> Result<Vec<u8>>`

- [ ] **Step 1: Add the reproducible font fixture**

Run:

```bash
rtk mkdir -p fixtures/fonts/ttf
rtk curl -L --fail --silent --show-error -o fixtures/fonts/ttf/roboto-regular.ttf https://github.com/googlefonts/roboto/raw/main/src/hinted/Roboto-Regular.ttf
printf "56a45233d29f11b4dfb86d248e921939d115778f87325e7ae8cc108383d6664d  fixtures/fonts/ttf/roboto-regular.ttf\n" > fixtures/fonts/ttf/roboto-regular.ttf.sha256
rtk shasum -a 256 -c fixtures/fonts/ttf/roboto-regular.ttf.sha256
```

Expected: PASS with `fixtures/fonts/ttf/roboto-regular.ttf: OK`.

- [ ] **Step 2: Write failing subset tests**

Create `crates/fontmin_subset/Cargo.toml` with:

```toml
[package]
name = "fontmin_subset"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description.workspace = true

[dependencies]
font-subset = { workspace = true }
fontmin_core = { workspace = true }
fontmin_diagnostics = { workspace = true }
serde = { workspace = true }

[lints]
workspace = true
```

Create `crates/fontmin_subset/src/lib.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::{subset_ttf, LayoutSubsetMode, SubsetOptions};

    const ROBOTO: &[u8] = include_bytes!("../../../fixtures/fonts/ttf/roboto-regular.ttf");

    #[test]
    fn subsets_ttf_to_a_smaller_valid_opentype_buffer() {
        let output = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                unicodes: Vec::new(),
                basic_text: false,
                preserve_hinting: false,
                trim: true,
                keep_notdef: true,
                layout: LayoutSubsetMode::Conservative,
            },
        )
        .unwrap();

        assert!(output.len() < ROBOTO.len());
        assert!(
            output.starts_with(&[0x00, 0x01, 0x00, 0x00]) || output.starts_with(b"OTTO"),
            "subset output must remain OpenType data",
        );
    }

    #[test]
    fn rejects_empty_subset_requests() {
        let error = subset_ttf(ROBOTO, SubsetOptions::default()).unwrap_err();

        assert!(error.to_string().contains("at least one character"));
    }

    #[test]
    fn rejects_invalid_font_data() {
        let error = subset_ttf(b"not a font", SubsetOptions::with_text("Hello")).unwrap_err();

        assert!(error.to_string().contains("invalid font data"));
    }
}
```

Run:

```bash
rtk cargo test -p fontmin_subset
```

Expected: FAIL because `SubsetOptions`, `LayoutSubsetMode`, and `subset_ttf` do not exist.

- [ ] **Step 3: Implement subset options and native subset**

Replace `crates/fontmin_subset/src/lib.rs` with:

```rust
use font_subset::{Font, FontReader};
use fontmin_core::collect_chars;
use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutSubsetMode {
    Drop,
    Conservative,
    Preserve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsetOptions {
    pub text: Option<String>,
    pub unicodes: Vec<u32>,
    pub basic_text: bool,
    pub preserve_hinting: bool,
    pub trim: bool,
    pub keep_notdef: bool,
    pub layout: LayoutSubsetMode,
}

impl Default for SubsetOptions {
    fn default() -> Self {
        Self {
            text: None,
            unicodes: Vec::new(),
            basic_text: false,
            preserve_hinting: false,
            trim: true,
            keep_notdef: true,
            layout: LayoutSubsetMode::Conservative,
        }
    }
}

impl SubsetOptions {
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }
}

pub fn subset_ttf(input: &[u8], options: SubsetOptions) -> Result<Vec<u8>> {
    let chars = collect_chars(options.text.as_deref(), &options.unicodes, options.basic_text)?;

    if chars.is_empty() {
        return Err(FontminError::config(
            "subset requires at least one character from text, unicodes, or basicText",
        ));
    }

    let reader = FontReader::new(input)
        .map_err(|error| FontminError::invalid_font(format!("invalid font data: {error}")))?;
    let font: Font<'_> = reader
        .read()
        .map_err(|error| FontminError::invalid_font(format!("invalid font data: {error}")))?;

    let permissions = font.permissions();
    if !permissions.allow_subsetting {
        return Err(FontminError::invalid_font(
            "font license does not allow subsetting",
        ));
    }

    let subset = font
        .subset(&chars)
        .map_err(|error| FontminError::invalid_font(format!("invalid font data: {error}")))?;

    Ok(subset.to_opentype())
}

#[cfg(test)]
mod tests {
    use super::{subset_ttf, LayoutSubsetMode, SubsetOptions};

    const ROBOTO: &[u8] = include_bytes!("../../../fixtures/fonts/ttf/roboto-regular.ttf");

    #[test]
    fn subsets_ttf_to_a_smaller_valid_opentype_buffer() {
        let output = subset_ttf(
            ROBOTO,
            SubsetOptions {
                text: Some("Hello".into()),
                unicodes: Vec::new(),
                basic_text: false,
                preserve_hinting: false,
                trim: true,
                keep_notdef: true,
                layout: LayoutSubsetMode::Conservative,
            },
        )
        .unwrap();

        assert!(output.len() < ROBOTO.len());
        assert!(
            output.starts_with(&[0x00, 0x01, 0x00, 0x00]) || output.starts_with(b"OTTO"),
            "subset output must remain OpenType data",
        );
    }

    #[test]
    fn rejects_empty_subset_requests() {
        let error = subset_ttf(ROBOTO, SubsetOptions::default()).unwrap_err();

        assert!(error.to_string().contains("at least one character"));
    }

    #[test]
    fn rejects_invalid_font_data() {
        let error = subset_ttf(b"not a font", SubsetOptions::with_text("Hello")).unwrap_err();

        assert!(error.to_string().contains("invalid font data"));
    }
}
```

- [ ] **Step 4: Add the public Rust facade**

Create `crates/fontmin/Cargo.toml` with:

```toml
[package]
name = "fontmin"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description.workspace = true

[dependencies]
fontmin_config = { workspace = true }
fontmin_core = { workspace = true }
fontmin_diagnostics = { workspace = true }
fontmin_subset = { workspace = true }

[lints]
workspace = true
```

Create `crates/fontmin/src/lib.rs` with:

```rust
pub use fontmin_config::FontminConfig;
pub use fontmin_core::{Asset, FontFormat, OutputFormat};
pub use fontmin_diagnostics::{FontminError, Result};
pub use fontmin_subset::{LayoutSubsetMode, SubsetOptions};

pub fn subset_ttf(input: &[u8], options: SubsetOptions) -> Result<Vec<u8>> {
    fontmin_subset::subset_ttf(input, options)
}

pub fn convert(input: &[u8], target: OutputFormat) -> Result<Vec<u8>> {
    match target {
        OutputFormat::Ttf => Ok(input.to_vec()),
        OutputFormat::Woff => Err(FontminError::unsupported("woff")),
        OutputFormat::Woff2 => Err(FontminError::unsupported("woff2")),
        OutputFormat::Eot => Err(FontminError::unsupported("eot")),
        OutputFormat::Svg => Err(FontminError::unsupported("svg")),
        OutputFormat::Css => Err(FontminError::unsupported("css")),
    }
}

#[cfg(test)]
mod tests {
    use fontmin_core::OutputFormat;
    use fontmin_diagnostics::FontminErrorKind;

    use super::convert;

    #[test]
    fn ttf_convert_keeps_bytes_for_now() {
        assert_eq!(convert(b"abc", OutputFormat::Ttf).unwrap(), b"abc");
    }

    #[test]
    fn unsupported_conversions_return_typed_errors() {
        let error = convert(b"abc", OutputFormat::Woff2).unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::UnsupportedFormat);
    }
}
```

- [ ] **Step 5: Run focused tests**

Run:

```bash
rtk cargo test -p fontmin_subset -p fontmin
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add fixtures/fonts/ttf crates/fontmin_subset crates/fontmin
git commit -m "feat: add native ttf subset api"
```

---

### Task 5: Rust CLI App

**Files:**

- Create: `apps/fontmin/Cargo.toml`
- Create: `apps/fontmin/src/main.rs`
- Create: `apps/fontmin/src/cli.rs`
- Create: `apps/fontmin/src/commands/mod.rs`
- Create: `apps/fontmin/src/commands/subset.rs`
- Create: `apps/fontmin/src/commands/inspect.rs`
- Create: `apps/fontmin/src/commands/doctor.rs`
- Create: `apps/fontmin/tests/cli.rs`

**Interfaces:**

- Consumes:
  - `fontmin::subset_ttf(input: &[u8], options: SubsetOptions) -> Result<Vec<u8>>`
  - `fontmin_detect::detect_format(bytes: &[u8]) -> FontFormat`
- Produces:
  - Binary `fontmin-rs`
  - CLI command `fontmin-rs subset INPUT -o OUTPUT -t TEXT`
  - CLI command `fontmin-rs inspect INPUT --json`
  - CLI command `fontmin-rs doctor`

- [ ] **Step 1: Write failing CLI integration tests**

Create `apps/fontmin/Cargo.toml` with:

```toml
[package]
name = "fontmin_app"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish = false
description.workspace = true

[[bin]]
name = "fontmin-rs"
path = "src/main.rs"

[dependencies]
bpaf = { workspace = true }
fontmin = { workspace = true }
fontmin_detect = { workspace = true }
miette = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }

[lints]
workspace = true
```

Create `apps/fontmin/tests/cli.rs` with:

```rust
use std::process::Command;

const ROBOTO: &[u8] = include_bytes!("../../../fixtures/fonts/ttf/roboto-regular.ttf");

#[test]
fn subset_command_writes_a_smaller_font() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    let output = tempdir.path().join("output.ttf");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("subset")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-t")
        .arg("Hello")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(std::fs::metadata(output).unwrap().len() < ROBOTO.len() as u64);
}

#[test]
fn doctor_command_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("doctor")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("fontmin-rs doctor ok"));
}
```

Run:

```bash
rtk cargo test -p fontmin_app --test cli
```

Expected: FAIL because the CLI source files do not exist.

- [ ] **Step 2: Implement CLI parsing**

Create `apps/fontmin/src/cli.rs` with:

```rust
use std::path::PathBuf;

use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options, version)]
pub enum Command {
    #[bpaf(command("subset"))]
    Subset {
        #[bpaf(positional("INPUT"))]
        input: PathBuf,

        #[bpaf(short('o'), long("output"), argument("OUTPUT"))]
        output: PathBuf,

        #[bpaf(short('t'), long("text"), argument("TEXT"))]
        text: String,

        #[bpaf(long("basic-text"))]
        basic_text: bool,
    },

    #[bpaf(command("inspect"))]
    Inspect {
        #[bpaf(positional("INPUT"))]
        input: PathBuf,

        #[bpaf(long("json"))]
        json: bool,
    },

    #[bpaf(command("doctor"))]
    Doctor,
}

pub fn parse() -> Command {
    command().run()
}
```

- [ ] **Step 3: Implement CLI commands**

Create `apps/fontmin/src/commands/mod.rs` with:

```rust
use miette::Result;

use crate::cli::Command;

pub mod doctor;
pub mod inspect;
pub mod subset;

pub async fn run(command: Command) -> Result<i32> {
    match command {
        Command::Subset {
            input,
            output,
            text,
            basic_text,
        } => subset::run(input, output, text, basic_text).await,
        Command::Inspect { input, json } => inspect::run(input, json).await,
        Command::Doctor => doctor::run().await,
    }
}
```

Create `apps/fontmin/src/commands/subset.rs` with:

```rust
use std::path::PathBuf;

use fontmin::SubsetOptions;
use miette::{Context, IntoDiagnostic, Result};

pub async fn run(input: PathBuf, output: PathBuf, text: String, basic_text: bool) -> Result<i32> {
    let bytes = tokio::fs::read(&input)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", input.display()))?;

    let subset = fontmin::subset_ttf(
        &bytes,
        SubsetOptions {
            text: Some(text),
            basic_text,
            ..SubsetOptions::default()
        },
    )
    .into_diagnostic()?;

    if let Some(parent) = output.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
    }

    tokio::fs::write(&output, subset)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to write {}", output.display()))?;

    Ok(0)
}
```

Create `apps/fontmin/src/commands/inspect.rs` with:

```rust
use std::path::PathBuf;

use fontmin_detect::detect_format;
use miette::{Context, IntoDiagnostic, Result};
use serde_json::json;

pub async fn run(input: PathBuf, json_output: bool) -> Result<i32> {
    let bytes = tokio::fs::read(&input)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read {}", input.display()))?;
    let format = detect_format(&bytes);

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "path": input,
                "format": format,
                "size": bytes.len()
            }))
            .into_diagnostic()?,
        );
    } else {
        println!("{}: {:?}, {} bytes", input.display(), format, bytes.len());
    }

    Ok(0)
}
```

Create `apps/fontmin/src/commands/doctor.rs` with:

```rust
use miette::Result;

pub async fn run() -> Result<i32> {
    println!("fontmin-rs doctor ok");
    Ok(0)
}
```

Create `apps/fontmin/src/main.rs` with:

```rust
mod cli;
mod commands;

#[tokio::main]
async fn main() -> miette::Result<()> {
    let command = cli::parse();
    let code = commands::run(command).await?;
    std::process::exit(code);
}
```

- [ ] **Step 4: Run CLI tests**

Run:

```bash
rtk cargo test -p fontmin_app --test cli
```

Expected: PASS.

- [ ] **Step 5: Run manual CLI smoke commands**

Run:

```bash
rtk cargo run -p fontmin_app -- subset fixtures/fonts/ttf/roboto-regular.ttf -o temp/roboto-subset.ttf -t Hello
rtk cargo run -p fontmin_app -- inspect temp/roboto-subset.ttf --json
rtk cargo run -p fontmin_app -- doctor
```

Expected:

- First command exits 0 and creates `temp/roboto-subset.ttf`.
- Second command prints JSON with `"format": "ttf"` or `"format": "otf"` and a size smaller than the fixture.
- Third command prints `fontmin-rs doctor ok`.

- [ ] **Step 6: Commit**

```bash
git add apps/fontmin
git commit -m "feat: add rust cli subset command"
```

---

### Task 6: Napi Binding Package

**Files:**

- Create: `napi/fontmin/Cargo.toml`
- Create: `napi/fontmin/build.rs`
- Create: `napi/fontmin/src/lib.rs`
- Create: `napi/fontmin/package.json`
- Create: `napi/fontmin/src-js/index.js`
- Create: `napi/fontmin/test/api.test.ts`

**Interfaces:**

- Consumes:
  - `fontmin::subset_ttf(input: &[u8], options: SubsetOptions) -> Result<Vec<u8>>`
  - `fontmin::SubsetOptions`
- Produces:
  - npm workspace package `@fontmin-rs/binding`
  - napi export `subsetTtf(input: Buffer, options?: JsSubsetOptions): Buffer`

- [ ] **Step 1: Write failing napi package files and API test**

Create `napi/fontmin/package.json` with:

```json
{
  "name": "@fontmin-rs/binding",
  "version": "0.0.0",
  "private": true,
  "description": "Native binding for fontmin-rs.",
  "type": "module",
  "main": "src-js/index.js",
  "types": "src-js/index.d.ts",
  "exports": {
    ".": {
      "types": "./src-js/index.d.ts",
      "default": "./src-js/index.js"
    },
    "./package.json": "./package.json"
  },
  "files": ["src-js"],
  "scripts": {
    "build": "napi build --esm --platform --js bindings.js --dts index.d.ts --output-dir src-js --release",
    "build:debug": "napi build --esm --platform --js bindings.js --dts index.d.ts --output-dir src-js",
    "build:release": "pnpm run build",
    "test": "vitest --dir test"
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
  "devDependencies": {
    "vitest": "catalog:"
  }
}
```

Create `napi/fontmin/test/api.test.ts` with:

```ts
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'
import { expect, it } from 'vitest'

import { subsetTtf } from '../src-js/index.js'

const currentDir = dirname(fileURLToPath(import.meta.url))
const fixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
)

it('subsets a TTF buffer through napi', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { text: 'Hello' })

  expect(Buffer.isBuffer(output)).toBe(true)
  expect(output.byteLength).toBeLessThan(input.byteLength)
})
```

Run:

```bash
rtk pnpm --filter @fontmin-rs/binding test
```

Expected: FAIL because `src-js/index.js` and native binding do not exist.

- [ ] **Step 2: Implement Rust napi binding**

Create `napi/fontmin/Cargo.toml` with:

```toml
[package]
name = "fontmin_napi"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description.workspace = true

[lib]
crate-type = ["cdylib"]
path = "src/lib.rs"

[dependencies]
fontmin = { workspace = true }
napi = { workspace = true }
napi-derive = { workspace = true }

[build-dependencies]
napi-build = { workspace = true }

[lints]
workspace = true
```

Create `napi/fontmin/build.rs` with:

```rust
fn main() {
    napi_build::setup();
}
```

Create `napi/fontmin/src/lib.rs` with:

```rust
use fontmin::{LayoutSubsetMode, SubsetOptions};
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi(object)]
pub struct JsSubsetOptions {
    pub text: Option<String>,
    pub unicodes: Option<Vec<u32>>,
    pub basic_text: Option<bool>,
    pub preserve_hinting: Option<bool>,
    pub trim: Option<bool>,
    pub keep_notdef: Option<bool>,
    pub keep_layout: Option<String>,
}

#[napi(js_name = "subsetTtf")]
pub fn subset_ttf(input: Buffer, options: Option<JsSubsetOptions>) -> napi::Result<Buffer> {
    let options = subset_options_from_js(options)?;
    let output =
        fontmin::subset_ttf(&input, options).map_err(|error| napi::Error::from_reason(error.to_string()))?;

    Ok(output.into())
}

fn subset_options_from_js(options: Option<JsSubsetOptions>) -> napi::Result<SubsetOptions> {
    let Some(options) = options else {
        return Ok(SubsetOptions::default());
    };

    Ok(SubsetOptions {
        text: options.text,
        unicodes: options.unicodes.unwrap_or_default(),
        basic_text: options.basic_text.unwrap_or(false),
        preserve_hinting: options.preserve_hinting.unwrap_or(false),
        trim: options.trim.unwrap_or(true),
        keep_notdef: options.keep_notdef.unwrap_or(true),
        layout: layout_mode_from_js(options.keep_layout)?,
    })
}

fn layout_mode_from_js(value: Option<String>) -> napi::Result<LayoutSubsetMode> {
    match value.as_deref().unwrap_or("conservative") {
        "drop" => Ok(LayoutSubsetMode::Drop),
        "conservative" => Ok(LayoutSubsetMode::Conservative),
        "preserve" => Ok(LayoutSubsetMode::Preserve),
        other => Err(napi::Error::from_reason(format!(
            "unknown keepLayout value: {other}",
        ))),
    }
}
```

- [ ] **Step 3: Implement JS loader wrapper**

Create `napi/fontmin/src-js/index.js` with:

```js
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const binding = require('./bindings.js')

export const subsetTtf = binding.subsetTtf
```

- [ ] **Step 4: Build and test the binding**

Run:

```bash
rtk pnpm --filter @fontmin-rs/binding build:debug
rtk pnpm --filter @fontmin-rs/binding test
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add napi/fontmin
git commit -m "feat: expose subset through napi"
```

---

### Task 7: TypeScript Public Package and Compat Shell

**Files:**

- Create: `packages/fontmin/package.json`
- Create: `packages/fontmin/tsconfig.json`
- Create: `packages/fontmin/tsdown.config.ts`
- Create: `packages/fontmin/src/index.ts`
- Create: `packages/fontmin/src/native.ts`
- Create: `packages/fontmin/src/types.ts`
- Create: `packages/fontmin/src/config.ts`
- Create: `packages/fontmin/src/plugins.ts`
- Create: `packages/fontmin/src/compat.ts`
- Create: `packages/fontmin/bin/fontmin-rs.mjs`
- Create: `packages/fontmin/test/api.test.ts`
- Delete: `src/index.ts`
- Delete: `tests/index.test.ts`
- Delete: `tsdown.config.ts`
- Delete: `vitest.config.ts`

**Interfaces:**

- Consumes:
  - `@fontmin-rs/binding.subsetTtf(input: Uint8Array, options?: SubsetOptions): Buffer`
- Produces:
  - `subsetTtf(input: Uint8Array, options?: SubsetOptions): Buffer`
  - `defineConfig<T extends FontminConfig>(config: T): T`
  - `definePlugin<T extends FontminPlugin>(plugin: T): T`
  - `glyph(options?: GlyphOptions): FontminPlugin`
  - `ttf2woff(options?: Ttf2WoffOptions): FontminPlugin`
  - `ttf2woff2(options?: Ttf2Woff2Options): FontminPlugin`
  - default export `FontminCompat`

- [ ] **Step 1: Write failing TypeScript package test**

Create `packages/fontmin/package.json` with:

```json
{
  "name": "fontmin-rs",
  "version": "0.0.0",
  "description": "Fast font subsetter and converter written in Rust with Node.js bindings.",
  "keywords": ["font", "subset", "ttf", "napi", "fontmin"],
  "homepage": "https://github.com/ntnyq/fontmin-rs#readme",
  "bugs": {
    "url": "https://github.com/ntnyq/fontmin-rs/issues"
  },
  "license": "MIT",
  "author": {
    "name": "ntnyq",
    "email": "ntnyq13@gmail.com"
  },
  "repository": {
    "type": "git",
    "url": "git+https://github.com/ntnyq/fontmin-rs.git"
  },
  "files": ["dist", "bin"],
  "type": "module",
  "sideEffects": false,
  "main": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "bin": {
    "fontmin-rs": "./bin/fontmin-rs.mjs"
  },
  "exports": {
    "./package.json": "./package.json",
    ".": {
      "types": "./dist/index.d.ts",
      "default": "./dist/index.js"
    },
    "./plugins": {
      "types": "./dist/plugins.d.ts",
      "default": "./dist/plugins.js"
    },
    "./compat": {
      "types": "./dist/compat.d.ts",
      "default": "./dist/compat.js"
    }
  },
  "publishConfig": {
    "access": "public"
  },
  "scripts": {
    "build": "tsdown",
    "dev": "tsdown --watch",
    "format": "oxfmt",
    "format:check": "oxfmt --check",
    "lint": "oxlint",
    "prepublishOnly": "pnpm run build",
    "test": "vitest --run",
    "typecheck": "tsgo --noEmit"
  },
  "dependencies": {
    "@fontmin-rs/binding": "workspace:*"
  },
  "devDependencies": {
    "@ntnyq/tsconfig": "^3.1.0",
    "@types/node": "catalog:",
    "tsdown": "catalog:",
    "vitest": "catalog:"
  }
}
```

Create `packages/fontmin/test/api.test.ts` with:

```ts
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { expect, it } from 'vitest'

import Fontmin, {
  defineConfig,
  definePlugin,
  glyph,
  subsetTtf,
  ttf2woff2,
} from '../src/index'

const currentDir = dirname(fileURLToPath(import.meta.url))
const fixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
)

it('subsets through the public package api', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { text: 'Hello' })

  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('returns typed config and plugin objects', () => {
  const config = defineConfig({
    input: ['fonts/*.ttf'],
    outDir: 'build',
    plugins: [glyph({ text: 'Hello' }), ttf2woff2()],
  })
  const plugin = definePlugin({ name: 'example' })

  expect(config.plugins).toHaveLength(2)
  expect(plugin.name).toBe('example')
})

it('builds a fontmin-compatible chain', () => {
  const instance = new Fontmin()
    .src('fixtures/fonts/ttf/roboto-regular.ttf')
    .use(Fontmin.glyph({ text: 'Hello' }))
    .dest('build')

  expect(instance.config()).toMatchObject({
    input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
    outDir: 'build',
  })
})
```

Run:

```bash
rtk pnpm --filter fontmin-rs test
```

Expected: FAIL because package source files do not exist.

- [ ] **Step 2: Add TypeScript build config**

Create `packages/fontmin/tsconfig.json` with:

```json
{
  "extends": "@ntnyq/tsconfig/strict.json",
  "compilerOptions": {
    "target": "ESNext",
    "lib": ["DOM", "ES2023"],
    "module": "ESNext",
    "types": ["node"],
    "esModuleInterop": true,
    "skipLibCheck": true
  },
  "include": ["src", "test"]
}
```

Create `packages/fontmin/tsdown.config.ts` with:

```ts
import { defineConfig } from 'tsdown'

export default defineConfig({
  clean: true,
  dts: {
    tsgo: true,
  },
  entry: ['src/index.ts', 'src/plugins.ts', 'src/compat.ts'],
  platform: 'node',
})
```

- [ ] **Step 3: Add public types and helpers**

Create `packages/fontmin/src/types.ts` with:

```ts
export type FontFormat =
  'ttf' | 'otf' | 'woff' | 'woff2' | 'eot' | 'svg' | 'unknown'

export type OutputFormat = 'ttf' | 'woff' | 'woff2' | 'eot' | 'svg' | 'css'

export type LayoutSubsetMode = 'drop' | 'conservative' | 'preserve'

export interface SubsetOptions {
  text?: string
  unicodes?: number[]
  basicText?: boolean
  preserveHinting?: boolean
  trim?: boolean
  keepNotdef?: boolean
  keepLayout?: LayoutSubsetMode
  hinting?: boolean
  clone?: boolean
}

export interface FontAsset {
  path: string
  contents: Uint8Array
  format: FontFormat
  sourceFormat: FontFormat
  meta: Record<string, unknown>
}

export interface PluginContext {
  cwd: string
  emitFile(asset: FontAsset): void
}

export type MaybePromise<T> = T | Promise<T>

export interface FontminPlugin {
  name: string
  enforce?: 'pre' | 'post'
  native?: {
    kind: 'builtin'
    name: string
    options: Record<string, unknown>
  }
  buildStart?(ctx: PluginContext): MaybePromise<void>
  transform?(
    asset: FontAsset,
    ctx: PluginContext,
  ): MaybePromise<FontAsset | FontAsset[] | null | undefined>
  generateBundle?(assets: FontAsset[], ctx: PluginContext): MaybePromise<void>
  buildEnd?(ctx: PluginContext): MaybePromise<void>
}

export interface Ttf2WoffOptions {
  clone?: boolean
  deflate?: boolean
  compressionLevel?: number
}

export interface Ttf2Woff2Options {
  clone?: boolean
  quality?: number
  fallback?: 'native' | 'wasm' | 'js' | 'auto'
}

export interface CssOptions {
  fontPath?: string
  base64?: boolean
  glyph?: boolean
  iconPrefix?: string
  fontFamily?: string
  asFileName?: boolean
  local?: boolean
  fontDisplay?: 'auto' | 'block' | 'swap' | 'fallback' | 'optional'
  target?: 'css' | 'scss' | 'less'
}

export interface FontminConfig {
  cwd?: string
  input?: Array<string | Uint8Array>
  outDir?: string
  clean?: boolean
  preserveOriginal?: boolean
  subset?: SubsetOptions
  plugins?: FontminPlugin[]
}
```

Create `packages/fontmin/src/config.ts` with:

```ts
import type { FontminConfig } from './types'

export function defineConfig<T extends FontminConfig>(config: T): T {
  return config
}
```

Create `packages/fontmin/src/plugins.ts` with:

```ts
import type {
  CssOptions,
  FontminPlugin,
  SubsetOptions,
  Ttf2Woff2Options,
  Ttf2WoffOptions,
} from './types'

export function definePlugin<T extends FontminPlugin>(plugin: T): T {
  return plugin
}

export function glyph(options: SubsetOptions = {}): FontminPlugin {
  const preserveHinting = options.preserveHinting ?? options.hinting ?? false

  return {
    name: 'fontmin:glyph',
    native: {
      kind: 'builtin',
      name: 'glyph',
      options: {
        text: options.text,
        unicodes: options.unicodes,
        basicText: options.basicText,
        hinting: options.hinting,
        trim: options.trim,
        keepNotdef: options.keepNotdef,
        keepLayout: options.keepLayout,
        clone: options.clone,
        preserveHinting,
      },
    },
  }
}

export function ttf2woff(options: Ttf2WoffOptions = {}): FontminPlugin {
  return {
    name: 'fontmin:ttf2woff',
    native: {
      kind: 'builtin',
      name: 'ttf2woff',
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

export function css(options: CssOptions = {}): FontminPlugin {
  return {
    name: 'fontmin:css',
    native: {
      kind: 'builtin',
      name: 'css',
      options,
    },
  }
}
```

- [ ] **Step 4: Add native wrapper and compat class**

Create `packages/fontmin/src/native.ts` with:

```ts
import { subsetTtf as nativeSubsetTtf } from '@fontmin-rs/binding'

import type { SubsetOptions } from './types'

interface NativeSubsetOptions {
  text?: string
  unicodes?: number[]
  basicText?: boolean
  preserveHinting?: boolean
  trim?: boolean
  keepNotdef?: boolean
  keepLayout?: string
}

export function subsetTtf(
  input: Uint8Array,
  options: SubsetOptions = {},
): Buffer {
  const nativeOptions: NativeSubsetOptions = {
    text: options.text,
    unicodes: options.unicodes,
    basicText: options.basicText,
    preserveHinting: options.preserveHinting ?? options.hinting,
    trim: options.trim,
    keepNotdef: options.keepNotdef,
    keepLayout: options.keepLayout,
  }

  return nativeSubsetTtf(input, nativeOptions)
}
```

Create `packages/fontmin/src/compat.ts` with:

```ts
import { defineConfig } from './config'
import { css, glyph, ttf2woff, ttf2woff2 } from './plugins'
import type { FontAsset, FontminConfig, FontminPlugin } from './types'

export default class FontminCompat {
  static glyph = glyph
  static ttf2woff = ttf2woff
  static ttf2woff2 = ttf2woff2
  static css = css

  private input: Array<string | Uint8Array> = []
  private outputDir?: string
  private plugins: FontminPlugin[] = []

  src(file: string | string[] | Uint8Array): this {
    this.input = Array.isArray(file) ? file : [file]
    return this
  }

  dest(dir: string): this {
    this.outputDir = dir
    return this
  }

  use(plugin: FontminPlugin): this {
    this.plugins.push(plugin)
    return this
  }

  config(): FontminConfig {
    return defineConfig({
      input: this.input,
      outDir: this.outputDir,
      plugins: this.plugins,
    })
  }

  async runAsync(): Promise<FontAsset[]> {
    throw new Error(
      'fontmin-rs optimize pipeline is not available in v0.1; use subsetTtf for native subsetting',
    )
  }

  run(callback: (error: Error | null, files?: FontAsset[]) => void): void {
    this.runAsync().then(
      files => callback(null, files),
      error => callback(error),
    )
  }
}
```

Create `packages/fontmin/src/index.ts` with:

```ts
export { defineConfig } from './config'
export { default } from './compat'
export { subsetTtf } from './native'
export { css, definePlugin, glyph, ttf2woff, ttf2woff2 } from './plugins'
export type * from './types'
```

Create `packages/fontmin/bin/fontmin-rs.mjs` with:

```js
#!/usr/bin/env node

console.error(
  'The JavaScript CLI wrapper is not available in v0.1. Use the Rust binary with cargo run -p fontmin_app -- subset.',
)
process.exit(1)
```

- [ ] **Step 5: Remove the old root starter package source**

Delete these files:

```bash
rm src/index.ts tests/index.test.ts tsdown.config.ts vitest.config.ts
```

- [ ] **Step 6: Install workspace links and run package tests**

Run:

```bash
rtk pnpm install
rtk pnpm --filter @fontmin-rs/binding build:debug
rtk pnpm --filter fontmin-rs test
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add package.json pnpm-lock.yaml packages/fontmin napi/fontmin src tests tsdown.config.ts vitest.config.ts
git commit -m "feat: add public typescript package"
```

---

### Task 8: CI, README, and Full Verification

**Files:**

- Modify: `.github/workflows/ci.yml`
- Modify: `README.md`

**Interfaces:**

- Consumes:
  - Root commands from Task 1.
  - Rust crates from Tasks 2 through 5.
  - napi and TypeScript packages from Tasks 6 and 7.
- Produces:
  - CI gates that run Rust and Node checks.
  - User-facing README documenting v0.1 scope.

- [ ] **Step 1: Write the failing full verification command**

Run:

```bash
rtk pnpm run check
```

Expected: FAIL if CI/README updates have not resolved workspace package paths, build outputs, or stale starter files.

- [ ] **Step 2: Update CI**

Replace `.github/workflows/ci.yml` with:

```yaml
name: CI

on:
  push:
    branches:
      - main
  pull_request:
    branches:
      - main

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: pnpm/action-setup@v6
      - uses: actions/setup-node@v6
        with:
          node-version: lts/*
          cache: pnpm
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy,rustfmt

      - run: pnpm install --frozen-lockfile
      - run: pnpm run format:check
      - run: pnpm run lint
      - run: pnpm run typecheck

  test:
    runs-on: ${{ matrix.os }}
    needs: [check]
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        node: [22.x, 24.x, 26.x]
      fail-fast: false
    steps:
      - uses: actions/checkout@v7
      - uses: pnpm/action-setup@v6
      - uses: actions/setup-node@v6
        with:
          node-version: ${{ matrix.node }}
          cache: pnpm
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy,rustfmt

      - run: pnpm install --frozen-lockfile
      - run: pnpm --filter @fontmin-rs/binding build:debug
      - run: pnpm run build
      - run: pnpm run test
```

- [ ] **Step 3: Update README**

Replace `README.md` with:

````markdown
# fontmin-rs

[![CI](https://github.com/ntnyq/fontmin-rs/workflows/CI/badge.svg)](https://github.com/ntnyq/fontmin-rs/actions)
[![NPM VERSION](https://img.shields.io/npm/v/fontmin-rs.svg)](https://www.npmjs.com/package/fontmin-rs)
[![NPM DOWNLOADS](https://img.shields.io/npm/dy/fontmin-rs.svg)](https://www.npmjs.com/package/fontmin-rs)
[![LICENSE](https://img.shields.io/github/license/ntnyq/fontmin-rs.svg)](https://github.com/ntnyq/fontmin-rs/blob/main/LICENSE)

Fast font subsetting and conversion tooling written in Rust with Node.js bindings.

## v0.1 Scope

The first working slice provides:

- a Rust workspace split into core crates, CLI, napi binding, and TypeScript package;
- native TTF subsetting through `subsetTtf(input, { text })`;
- a Rust CLI command: `fontmin-rs subset input.ttf -t "Hello" -o output.ttf`;
- typed JS helpers for config and built-in plugin declarations;
- a Fontmin-compatible chain shell for `.src().use().dest()`.

## Node API

```ts
import { readFileSync, writeFileSync } from 'node:fs'
import { subsetTtf } from 'fontmin-rs'

const input = readFileSync('fixtures/fonts/ttf/roboto-regular.ttf')
const output = subsetTtf(input, { text: 'Hello' })

writeFileSync('build/roboto-subset.ttf', output)
```
````

## Rust CLI

```shell
cargo run -p fontmin_app -- subset fixtures/fonts/ttf/roboto-regular.ttf -t "Hello" -o build/roboto-subset.ttf
```

## Roadmap

The design document in `docs/fontmin-rs-design.md` tracks the longer-term plan: WOFF, WOFF2, CSS generation, iconfont support, config loading, cache, wasm fallback, and full Fontmin compatibility.

## License

[MIT](./LICENSE) License © 2026-PRESENT [ntnyq](https://github.com/ntnyq)

````

- [ ] **Step 4: Run full verification**

Run:

```bash
rtk cargo fmt --all --check
rtk cargo clippy --workspace --all-targets --all-features
rtk cargo test --workspace
rtk pnpm --filter @fontmin-rs/binding build:debug
rtk pnpm run build
rtk pnpm run test
pnpm typecheck
````

Expected: PASS for every command.

- [ ] **Step 5: Check git state**

Run:

```bash
rtk git status --short
```

Expected: only intended files from Task 8 are modified.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/ci.yml README.md
git commit -m "chore: wire ci for rust and node packages"
```

---

## Self-Review

- Spec coverage: This plan implements the design document's v0.1 milestone and minimum initial directory. It covers workspace setup, Rust core models, native subset API, Rust CLI subset command, napi binding, JS facade, Fontmin-compatible class shell, fixtures, tests, and CI. Later design sections for WOFF, WOFF2, CSS, SVG, OTF, cache, custom JS plugins, benchmarks, release artifacts, and wasm are explicitly scoped into future milestone plans.
- Placeholder scan: No task contains open-ended validation instructions, unnamed files, unnamed commands, or missing test commands.
- Type consistency: `SubsetOptions`, `LayoutSubsetMode`, `FontFormat`, `OutputFormat`, `FontminConfig`, `FontminPlugin`, and `subsetTtf` names are consistent across Rust, napi, and TypeScript tasks.
