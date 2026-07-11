# Get Started

fontmin-rs provides two entry points:

- The `fontmin-rs` command line tool for one-off processing, CI, and scripts.
- The TypeScript API for build integrations and custom font pipelines.

## Install

```sh
pnpm add fontmin-rs
```

When developing inside this repository, build the debug native binding first:

```sh
pnpm install
pnpm run build:debug
```

## Generate Web Fonts With the CLI

```sh
fontmin-rs build fixtures/fonts/ttf/roboto-regular.ttf \
  -o build \
  --text "Hello, fontmin-rs" \
  --preset modern-web \
  --font-family Roboto \
  --font-path ./
```

This command:

1. Reads the input TTF.
2. Keeps only the glyphs required by `--text`.
3. Emits WOFF2, WOFF, and CSS.
4. Generates a ready-to-use `@font-face` rule.

## Use the TypeScript API

```ts
import { modernWeb, optimize } from 'fontmin-rs'

await optimize({
  input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
  outDir: 'build',
  cache: { enabled: true },
  plugins: modernWeb({
    text: 'Hello, fontmin-rs',
    fontFamily: 'Roboto',
    fontPath: './',
    fontDisplay: 'swap',
  }),
})
```

`modernWeb()` is a preset plugin group that runs subsetting, generates WOFF, generates WOFF2, and emits CSS.

## Use the Browser WASM API

For client-side processing, install `@fontmin-rs/wasm`. Its API is asynchronous
and works on named in-memory assets rather than file paths:

```ts
import { initWasm, modernWeb, optimizeBrowser } from '@fontmin-rs/wasm'

await initWasm()
const assets = await optimizeBrowser({
  assets: [{ contents: ttfBytes, fileName: 'roboto.ttf' }],
  plugins: modernWeb({ text: 'Hello', fontFamily: 'Roboto' }),
})
```

The browser package has no filesystem, glob, CLI, or disk-cache support.

## Inspect Font Metadata

```sh
fontmin-rs inspect fixtures/fonts/ttf/roboto-regular.ttf --json
```

The same operation is available from the Node API:

```ts
import { inspect, validateWoff2 } from 'fontmin-rs'
import { readFileSync } from 'node:fs'

const input = readFileSync('fixtures/fonts/ttf/roboto-regular.ttf')
const info = inspect(input)

console.log(info.format)
console.log(info.metadata.familyName)

const woff2 = readFileSync('build/roboto-regular.woff2')
validateWoff2(woff2)
```

## Compatibility Notes

EOT output is provided for old IE compatibility. OTF metadata inspection is available, and `otf2ttf` / `otfToTtf` convert static CFF OTF fonts to static TrueType `glyf` fonts, instantiate CFF2 variable fonts at the default or requested coordinates, or rewrite glyf-backed OTF wrappers to TTF. The result is static, with CFF2 and variation tables removed; Type 2 hinting is discarded. WOFF2 inspect/validate checks headers and table directories, reads sfnt metadata tables, and supports WOFF2-to-TTF decode through `woff2ToTtf()` or CLI `convert -f ttf`.
