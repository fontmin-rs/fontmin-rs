# fontmin-rs

[![CI](https://github.com/fontmin-rs/fontmin-rs/workflows/CI/badge.svg)](https://github.com/fontmin-rs/fontmin-rs/actions)
[![NPM VERSION](https://img.shields.io/npm/v/fontmin-rs.svg)](https://www.npmjs.com/package/fontmin-rs)
[![NPM DOWNLOADS](https://img.shields.io/npm/dy/fontmin-rs.svg)](https://www.npmjs.com/package/fontmin-rs)
[![LICENSE](https://img.shields.io/github/license/fontmin-rs/fontmin-rs.svg)](https://github.com/fontmin-rs/fontmin-rs/blob/main/LICENSE)

Fast font subsetting and conversion tooling written in Rust with Node.js bindings.

## Current Scope

The current working slices provide:

- a Rust workspace split into core crates, CLI, napi binding, and TypeScript package;
- a minimal `optimize(config)` pipeline for `glyph`, `ttf2eot`, `ttf2svg`, `svg2ttf`, `svgs2ttf`, `ttf2woff`, `ttf2woff2`, and `css` built-in plugins;
- a `modernWeb(options)` preset for subset + WOFF + WOFF2 + CSS output;
- a `fontminCompatPreset(options)` preset for Fontmin-style EOT, SVG, WOFF, WOFF2, and CSS output;
- cache reuse for matching native `optimize(config)` inputs and built-in plugin options;
- pipeline-wide `native`, `wasm`, and native-first `auto` runtimes for Node `optimize(config)`, with runtime-separated cache entries;
- a browser-only WASM package with direct helpers and the in-memory `optimizeBrowser(config)` pipeline;
- native TTF subsetting through `subsetTtf(input, { text })`;
- TTF to WOFF conversion through `ttfToWoff(input)`;
- WOFF metadata and private data blocks through `ttfToWoff(input, { metadata, privateData })`;
- WOFF to TTF decoding through `woffToTtf(input)`;
- TTF to WOFF2 conversion through `ttfToWoff2(input)`;
- WOFF2 to TTF decoding through `woff2ToTtf(input)`;
- TTF to legacy EOT conversion through `ttfToEot(input)`;
- EOT to TTF decoding through `eotToTtf(input)`;
- TTF to SVG font conversion through `ttfToSvg(input)`;
- SVG font to TTF conversion through `svgFontToTtf(input)`;
- multiple SVG icons to TTF iconfont conversion through `svgsToTtf(inputs)`;
- `@font-face` CSS generation through `generateFontFaceCss(sources, options)`;
- real TTF, OTF, WOFF, WOFF2, and EOT metadata inspection through `inspect(input)` and `fontmin-rs inspect`;
- Rust CLI commands for `init`, `subset`, `convert`, `build`, `bench`, `inspect`, and `doctor`;
- shared JSON/JSONC/TS/MTS/MJS/CJS configuration loading in the Rust CLI, with Node 22+ used on demand for module configs;
- an `iconfont` build preset that groups SVG icon inputs into `iconfont.ttf` and glyph class CSS;
- an npm package bin wrapper for `init`, `subset`, `convert`, `build`, `bench`, and `inspect`;
- typed JS helpers for config and built-in plugin declarations, including static CFF/CFF2 `otf2ttf` conversion;
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
  validateWoff2,
  woff2ToTtf,
  woffToTtf,
} from 'fontmin-rs'
import { readFileSync, writeFileSync } from 'node:fs'

const input = readFileSync('fixtures/fonts/ttf/roboto-regular.ttf')
const info = inspect(input)
const output = subsetTtf(input, { text: 'Hello' })
const woff = ttfToWoff(output)
const decoded = woffToTtf(woff)
const woff2 = ttfToWoff2(output)
validateWoff2(woff2)
const decodedWoff2 = woff2ToTtf(woff2)
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
const woff2Info = inspect(woff2)
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
console.log(woff2Info.metadata.tables)
console.log(eotInfo.format)
writeFileSync('build/roboto-subset.ttf', output)
writeFileSync('build/roboto-subset.woff', woff)
writeFileSync('build/roboto-decoded.ttf', decoded)
writeFileSync('build/roboto-subset.woff2', woff2)
writeFileSync('build/roboto-decoded-woff2.ttf', decodedWoff2)
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
cargo run -p fontmin_app -- init
cargo run -p fontmin_app -- subset fixtures/fonts/ttf/roboto-regular.ttf -t "Hello" -o build/roboto-subset.ttf
cargo run -p fontmin_app -- convert fixtures/fonts/ttf/roboto-regular.ttf -f woff2 -o build/roboto.woff2
cargo run -p fontmin_app -- convert fixtures/fonts/ttf/roboto-regular.ttf -f eot -o build/roboto.eot
cargo run -p fontmin_app -- convert fixtures/fonts/ttf/roboto-regular.ttf -f svg -o build/roboto.svg
cargo run -p fontmin_app -- convert fixtures/fonts/otf/source-sans-3-regular.otf -f ttf -o build/source-sans-3.ttf
cargo run -p fontmin_app -- convert fixtures/fonts/otf/source-serif-4-variable-roman.otf -f ttf --variation wght=700 --variation opsz=14 -o build/source-serif-4.ttf
cargo run -p fontmin_app -- convert build/roboto.eot -f ttf -o build/roboto-from-eot.ttf
cargo run -p fontmin_app -- build fixtures/fonts/ttf/roboto-regular.ttf -o build --text "Hello" --preset modern-web --font-family Roboto
cargo run -p fontmin_app -- build icons/home.svg icons/user.svg -o build/icons --preset iconfont --font-family "Project Icons"
cargo run -p fontmin_app -- inspect fixtures/fonts/ttf/roboto-regular.ttf --json
```

The published package bin exposes the same Node binding commands:

```shell
fontmin-rs init
fontmin-rs subset fixtures/fonts/ttf/roboto-regular.ttf -o build/roboto-subset.ttf --text "Hello"
fontmin-rs convert fixtures/fonts/ttf/roboto-regular.ttf -f woff2 -o build/roboto.woff2
fontmin-rs convert fixtures/fonts/ttf/roboto-regular.ttf -f eot -o build/roboto.eot
fontmin-rs convert fixtures/fonts/ttf/roboto-regular.ttf -f svg -o build/roboto.svg
fontmin-rs convert fixtures/fonts/otf/source-sans-3-regular.otf -f ttf -o build/source-sans-3.ttf
fontmin-rs convert fixtures/fonts/otf/source-serif-4-variable-roman.otf -f ttf --variation wght=700 --variation opsz=14 -o build/source-serif-4.ttf
fontmin-rs convert build/roboto.woff -f ttf -o build/roboto.ttf
fontmin-rs convert build/roboto.woff2 -f ttf -o build/roboto-from-woff2.ttf
fontmin-rs convert build/roboto.eot -f ttf -o build/roboto-from-eot.ttf
fontmin-rs build fixtures/fonts/ttf/roboto-regular.ttf -o build --text "Hello" --preset compat --font-family Roboto
fontmin-rs build icons/home.svg icons/user.svg -o build/icons --preset iconfont --font-family "Project Icons"
fontmin-rs inspect fixtures/fonts/ttf/roboto-regular.ttf --json
fontmin-rs inspect build/roboto.woff --json
fontmin-rs inspect build/roboto.woff2 --json
fontmin-rs inspect build/roboto.eot --json
```

EOT is provided for older IE compatibility. OTF metadata inspection is supported, and `otf2ttf` / `otfToTtf` convert static CFF OpenType fonts into static TrueType `glyf` fonts and instantiate CFF2 variable fonts at their default or requested axis coordinates. Glyph IDs, cmap mappings, metrics, names, and supported OpenType layout tables are retained. The output is always static: CFF2, `fvar`, `avar`, `STAT`, HVAR, MVAR, and other variation tables are removed, and Type 2 hinting is discarded. `validateWoff2(input)` validates the WOFF2 header and table directory, WOFF2 inspect reads sfnt metadata tables such as `name`, `head`, `hhea`, and `maxp`, and `woff2ToTtf(input)` decodes WOFF2 back to TTF. The `modernWeb(options)` preset and CLI `--preset modern-web` intentionally emit WOFF, WOFF2, and CSS only. CLI `--preset iconfont` expects SVG icon inputs and emits `iconfont.ttf` plus `iconfont.css`. Use CLI `--text-file` or `--unicodes` to drive subset text outside inline strings, and `--no-original` when a `ttf` format is requested but the original TTF should not be written.

## Roadmap

The design document in `docs/fontmin-rs-design.md` tracks the longer-term plan. Remaining work is focused on release hardening, richer diagnostics and performance work, and broader Fontmin compatibility.

## License

[MIT](./LICENSE) License © 2026-PRESENT [ntnyq](https://github.com/ntnyq)
