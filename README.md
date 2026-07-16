# fontmin-rs

Fast font subsetting and conversion tooling powered by Rust, with CLI, Node.js, and browser WASM APIs.

[Documentation](https://fontmin-rs.ntnyq.dev/) · [Getting Started](https://fontmin-rs.ntnyq.dev/guide/getting-started) · [API Reference](https://fontmin-rs.ntnyq.dev/api/node) · [Playground](https://fontmin-rs.ntnyq.dev/playground) · [简体中文](https://fontmin-rs.ntnyq.dev/zh/)

[![CI](https://github.com/fontmin-rs/fontmin-rs/workflows/CI/badge.svg)](https://github.com/fontmin-rs/fontmin-rs/actions)
[![NPM VERSION](https://img.shields.io/npm/v/fontmin-rs.svg)](https://www.npmjs.com/package/fontmin-rs)
[![NPM DOWNLOADS](https://img.shields.io/npm/dy/fontmin-rs.svg)](https://www.npmjs.com/package/fontmin-rs)
[![LICENSE](https://img.shields.io/github/license/fontmin-rs/fontmin-rs.svg)](https://github.com/fontmin-rs/fontmin-rs/blob/main/LICENSE)

> [!NOTE]
> fontmin-rs is currently in beta. APIs and generated output may change before 1.0.

## Features

- Subset fonts by text, code points, or Unicode ranges.
- Convert TTF to and from WOFF, WOFF2, EOT, and SVG font inputs, with OTF-to-TTF support.
- Generate web-font bundles and `@font-face` CSS with built-in presets.
- Run through the CLI, the typed Node.js API, or an in-memory browser WASM API.
- Migrate existing Fontmin pipelines with compatible plugins and chaining APIs.

See the [complete feature overview](https://fontmin-rs.ntnyq.dev/guide/features) for supported operations, presets, runtimes, and current limitations.

## Install

```sh
pnpm add fontmin-rs
```

For browser-only workflows, install [`@fontmin-rs/wasm`](https://fontmin-rs.ntnyq.dev/api/wasm).

## Quick Start

```sh
fontmin-rs build fonts/roboto.ttf \
  --out-dir build \
  --text "Hello, fontmin-rs" \
  --preset modern-web \
  --font-family Roboto
```

Continue with the [Getting Started guide](https://fontmin-rs.ntnyq.dev/guide/getting-started) for CLI, configuration, Node.js, and browser examples.

## License

[MIT](./LICENSE) License © 2026-PRESENT [ntnyq](https://github.com/ntnyq)
