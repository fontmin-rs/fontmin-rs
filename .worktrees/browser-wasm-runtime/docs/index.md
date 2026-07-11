---
layout: home

hero:
  name: fontmin-rs
  text: Fast font subsetting and conversion
  tagline: A Rust-powered font pipeline with Node.js bindings, CLI commands, and typed APIs for modern frontend workflows.
  image:
    src: /logo.svg
    alt: fontmin-rs
  actions:
    - theme: brand
      text: Get Started
      link: /guide/getting-started
    - theme: alt
      text: Node API
      link: /api/node

features:
  - title: Font subsetting
    details: Keep only the glyphs required by text, textFile, unicodes, or basicText, with conservative and preserve layout modes.
  - title: Multi-format output
    details: Generate TTF, WOFF, WOFF2, EOT, SVG font, and @font-face CSS for modern web and legacy compatibility targets.
  - title: Rust + Node
    details: Core crates, the CLI, the N-API binding, and the TypeScript package share the same behavior goals.
---

## Current Status

fontmin-rs is still early, but it already provides a usable font processing path:

- `fontmin-rs subset` trims TTF fonts by text.
- `fontmin-rs convert` converts between TTF, WOFF, WOFF2, EOT, SVG font, and related formats.
- `fontmin-rs build` creates multi-format assets and CSS from input fonts.
- `fontmin-rs inspect` reports metadata for TTF, OTF, WOFF, EOT, and other supported formats.
- `fontmin-rs init` creates a starter JSONC config file.
- The `fontmin-rs` npm package provides the bin command, low-level native helpers, an `optimize(config)` pipeline, and a Fontmin-compatible chain.

## Install

::: code-group

```sh [pnpm]
pnpm add fontmin-rs
```

```sh [npm]
npm install fontmin-rs
```

```sh [yarn]
yarn add fontmin-rs
```

:::

## Minimal Example

```sh
fontmin-rs build fixtures/fonts/ttf/roboto-regular.ttf \
  -o build \
  --text "Hello" \
  --formats woff2,woff,css \
  --font-family Roboto
```

Continue with [Get Started](/guide/getting-started) to learn how the CLI, configuration files, and Node API fit together. If you already use Fontmin, start with the [migration guide](/guide/migration).
