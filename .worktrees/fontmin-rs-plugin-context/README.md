# fontmin-rs

[![CI](https://github.com/ntnyq/fontmin-rs/workflows/CI/badge.svg)](https://github.com/ntnyq/fontmin-rs/actions)
[![NPM VERSION](https://img.shields.io/npm/v/fontmin-rs.svg)](https://www.npmjs.com/package/fontmin-rs)
[![NPM DOWNLOADS](https://img.shields.io/npm/dy/fontmin-rs.svg)](https://www.npmjs.com/package/fontmin-rs)
[![LICENSE](https://img.shields.io/github/license/ntnyq/fontmin-rs.svg)](https://github.com/ntnyq/fontmin-rs/blob/main/LICENSE)

Fast font subsetting and conversion tooling written in Rust with Node.js bindings.

## Current Scope

The current working slices provide:

- a Rust workspace split into core crates, CLI, napi binding, and TypeScript package;
- a minimal `optimize(config)` pipeline for `glyph`, `ttf2eot`, `ttf2svg`, `svg2ttf`, `svgs2ttf`, `ttf2woff`, `ttf2woff2`, and `css` built-in plugins;
- a `modernWeb(options)` preset for subset + WOFF + WOFF2 + CSS output;
- cache reuse for matching native `optimize(config)` inputs and built-in plugin options;
- native TTF subsetting through `subsetTtf(input, { text })`;
- TTF to WOFF conversion through `ttfToWoff(input)`;
- WOFF to TTF decoding through `woffToTtf(input)`;
- TTF to WOFF2 conversion through `ttfToWoff2(input)`;
- TTF to legacy EOT conversion through `ttfToEot(input)`;
- EOT to TTF decoding through `eotToTtf(input)`;
- TTF to SVG font conversion through `ttfToSvg(input)`;
- SVG font to TTF conversion through `svgFontToTtf(input)`;
- multiple SVG icons to TTF iconfont conversion through `svgsToTtf(inputs)`;
- `@font-face` CSS generation through `generateFontFaceCss(sources, options)`;
- real TTF, OTF, WOFF, and EOT metadata inspection through `inspect(input)` and `fontmin-rs inspect`;
- Rust CLI commands for `subset`, `convert`, `build`, `inspect`, and `doctor`;
- an npm package bin wrapper for `subset`, `convert`, `build`, and `inspect`;
- typed JS helpers for config and built-in plugin declarations, including a staged `otf2ttf` entry point;
- custom JS plugin hooks with filesystem, emit, and warning diagnostics context helpers;
- a Fontmin-compatible chain for `.src().use().dest().runAsync()`.

## Node API

```ts
import {
  eotToTtf,
  generateFontFaceCss,
  inspect,
  modernWeb,
  optimize,
  subsetTtf,
  svg2ttf,
  svgFontToTtf,
  svgs2ttf,
  svgsToTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
  woffToTtf,
} from 'fontmin-rs'
import { readFileSync, writeFileSync } from 'node:fs'

const input = readFileSync('fixtures/fonts/ttf/roboto-regular.ttf')
const info = inspect(input)
const output = subsetTtf(input, { text: 'Hello' })
const woff = ttfToWoff(output)
const decoded = woffToTtf(woff)
const woff2 = ttfToWoff2(output)
const eot = ttfToEot(output)
const decodedEot = eotToTtf(eot)
const svg = ttfToSvg(output)
const ttfFromSvgFont = svgFontToTtf(
  '<svg><defs><font horiz-adv-x="1000"><font-face font-family="Icons" units-per-em="1000" ascent="850" descent="-150" /><glyph unicode="&#xE001;" d="M100 100 L900 100 L900 900 L100 900 Z" /></font></defs></svg>',
)
const iconFont = svgsToTtf(
  [
    {
      name: 'home',
      contents:
        '<svg viewBox="0 0 1000 1000"><path d="M100 500 L500 100 L900 500 L900 900 L100 900 Z"/></svg>',
      unicode: 0xe001,
    },
  ],
  { fontName: 'Icons' },
)
const woffInfo = inspect(woff)
const eotInfo = inspect(eot)
const cssText = generateFontFaceCss(
  [
    { fileName: 'roboto-subset.woff2', format: 'woff2' },
    { fileName: 'roboto-subset.woff', format: 'woff' },
    { fileName: 'roboto-subset.svg', format: 'svg' },
  ],
  { fontFamily: 'Roboto', fontPath: './' },
)

console.log(info.metadata.familyName)
console.log(woffInfo.format)
console.log(eotInfo.format)
writeFileSync('build/roboto-subset.ttf', output)
writeFileSync('build/roboto-subset.woff', woff)
writeFileSync('build/roboto-decoded.ttf', decoded)
writeFileSync('build/roboto-subset.woff2', woff2)
writeFileSync('build/roboto-subset.eot', eot)
writeFileSync('build/roboto-decoded-eot.ttf', decodedEot)
writeFileSync('build/roboto-subset.svg', svg)
writeFileSync('build/icons-from-svg-font.ttf', ttfFromSvgFont)
writeFileSync('build/icons.ttf', iconFont)
writeFileSync('build/roboto.css', cssText)

await optimize({
  input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
  outDir: 'build',
  cache: { enabled: true },
  plugins: modernWeb({ text: 'Hello', fontFamily: 'Roboto', fontPath: './' }),
})

await optimize({
  input: ['fixtures/fonts/svg-font/icons.svg'],
  outDir: 'build',
  plugins: [svg2ttf({ clone: false })],
})

await optimize({
  input: [
    'fixtures/fonts/icon-svg/home.svg',
    'fixtures/fonts/icon-svg/user.svg',
  ],
  outDir: 'build',
  plugins: [svgs2ttf({ fontName: 'icons' })],
})
```

## Rust CLI

```shell
cargo run -p fontmin_app -- subset fixtures/fonts/ttf/roboto-regular.ttf -t "Hello" -o build/roboto-subset.ttf
cargo run -p fontmin_app -- convert fixtures/fonts/ttf/roboto-regular.ttf -f woff2 -o build/roboto.woff2
cargo run -p fontmin_app -- convert fixtures/fonts/ttf/roboto-regular.ttf -f eot -o build/roboto.eot
cargo run -p fontmin_app -- convert fixtures/fonts/ttf/roboto-regular.ttf -f svg -o build/roboto.svg
cargo run -p fontmin_app -- convert build/roboto.eot -f ttf -o build/roboto-from-eot.ttf
cargo run -p fontmin_app -- build fixtures/fonts/ttf/roboto-regular.ttf -o build --text "Hello" --formats woff2,woff,svg,css --font-family Roboto
cargo run -p fontmin_app -- inspect fixtures/fonts/ttf/roboto-regular.ttf --json
```

The published package bin exposes the same Node binding commands:

```shell
fontmin-rs subset fixtures/fonts/ttf/roboto-regular.ttf -o build/roboto-subset.ttf --text "Hello"
fontmin-rs convert fixtures/fonts/ttf/roboto-regular.ttf -f woff2 -o build/roboto.woff2
fontmin-rs convert fixtures/fonts/ttf/roboto-regular.ttf -f eot -o build/roboto.eot
fontmin-rs convert fixtures/fonts/ttf/roboto-regular.ttf -f svg -o build/roboto.svg
fontmin-rs convert build/roboto.woff -f ttf -o build/roboto.ttf
fontmin-rs convert build/roboto.eot -f ttf -o build/roboto-from-eot.ttf
fontmin-rs build fixtures/fonts/ttf/roboto-regular.ttf -o build --text "Hello" --formats woff2,woff,eot,svg,css --font-family Roboto
fontmin-rs inspect fixtures/fonts/ttf/roboto-regular.ttf --json
fontmin-rs inspect build/roboto.woff --json
fontmin-rs inspect build/roboto.eot --json
```

EOT is provided for older IE compatibility. OTF metadata inspection is supported; `otf2ttf` / `otfToTtf` currently report a clear unsupported diagnostic until OTF to TTF outline conversion lands. The `modernWeb(options)` preset intentionally emits WOFF, WOFF2, and CSS only.

## Roadmap

The design document in `docs/fontmin-rs-design.md` tracks the longer-term plan: iconfont support, wasm fallback, richer CLI commands, multi-platform publishing, and full Fontmin compatibility.

## License

[MIT](./LICENSE) License © 2026-PRESENT [ntnyq](https://github.com/ntnyq)
