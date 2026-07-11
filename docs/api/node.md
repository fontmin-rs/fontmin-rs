# Node API

The `fontmin-rs` Node API has three layers:

- Low-level native helpers that operate directly on `Uint8Array`.
- The `optimize(config)` pipeline for input files, plugins, caching, and output.
- A Fontmin-compatible default export for migrating existing Fontmin chains.

## Native helpers

```ts
import {
  eotToTtf,
  generateFontFaceCss,
  inspect,
  subsetTtf,
  svgFontToTtf,
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
const subset = subsetTtf(input, { text: 'Hello' })
const woff2 = ttfToWoff2(subset)
validateWoff2(woff2)
const decodedWoff2 = woff2ToTtf(woff2)
const info = inspect(woff2)

writeFileSync('build/roboto-subset.woff2', woff2)
writeFileSync('build/roboto-decoded-woff2.ttf', decodedWoff2)
console.log(info.format)
```

`ttfToWoff(input, options)` accepts `metadata` XML and `privateData` bytes for WOFF 1.0 auxiliary blocks. The metadata is zlib-compressed in the WOFF file; private data is stored as the final block.

`ttfToWoff2(input, { fallback })` stays synchronous and native-only. It accepts
`native` and `auto`; `fallback: 'wasm'` explains that the WASM path is
asynchronous.

Use `ttfToWoff2Async()` when a native artifact may be unavailable. It loads the
packaged WASM runtime only when requested. `fallback: 'wasm'` always uses WASM;
`fallback: 'auto'` tries the native binding first and falls back only when that
binding cannot load. Invalid font data and native encoder failures are returned
without a WASM retry.

```ts
const woff2 = await ttfToWoff2Async(input, { fallback: 'auto' })
```

`fallback: 'js'` remains unsupported. The file-based `optimize()` pipeline is
also synchronous and native-only.

`validateWoff2(input)` validates the WOFF2 header and table directory, returning normally for valid input and throwing for invalid data. `inspect(woff2)` performs the same validation and reads sfnt metadata tables such as `name`, `head`, `hhea`, and `maxp`. `woff2ToTtf(input)` decodes WOFF2 back to TTF through the native binding.

## Browser WASM API

For browser-only processing, use the separate
[Browser WASM API](./wasm). It documents initialization, direct conversions,
the in-memory pipeline, custom browser plugins, and browser-only boundaries.

## optimize

```ts
import { css, glyph, optimize, ttf2woff, ttf2woff2 } from 'fontmin-rs'

const assets = await optimize({
  input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
  outDir: 'build',
  cache: { enabled: true },
  plugins: [
    glyph({ text: 'Hello' }),
    ttf2woff(),
    ttf2woff2(),
    css({ fontFamily: 'Roboto', fontPath: './' }),
  ],
})

console.log(assets.map(asset => asset.path))
```

## modernWeb preset

```ts
import { modernWeb, optimize } from 'fontmin-rs'

await optimize({
  input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
  outDir: 'build',
  plugins: modernWeb({
    text: 'Hello',
    fontFamily: 'Roboto',
    fontPath: './',
  }),
})
```

`modernWeb()` first normalizes supported CFF/CFF2 OTF input to static TTF, then
combines `glyph()`, `ttf2woff()`, `ttf2woff2()`, and `css()`. Pass
`variationCoordinates` to select a CFF2 instance; the source OTF is not
emitted. It does not generate EOT or SVG; add `ttf2eot()` or `ttf2svg()`
explicitly if you need those formats.

## Fontmin compatibility preset

```ts
import { fontminCompatPreset, optimize } from 'fontmin-rs'

await optimize({
  input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
  outDir: 'build',
  plugins: fontminCompatPreset({
    text: 'Hello',
    fontFamily: 'Roboto',
    fontPath: './',
  }),
})
```

`fontminCompatPreset()` follows the classic Fontmin order: `otf2ttf()`, `glyph()`, `ttf2eot()`, `ttf2svg()`, `ttf2woff()`, `ttf2woff2()`, and `css()`. It is also available from the `fontmin-rs/presets` subpath.

`otfToTtf()` converts static CFF OpenType fonts or instantiates CFF2 variable fonts into static TrueType `glyf` fonts. Pass user-space coordinates with `variationCoordinates`:

```ts
otfToTtf(input, { variationCoordinates: { wght: 700, opsz: 14 } })
```

Glyph IDs, cmap mappings, metrics, names, and supported OpenType layout tables are retained. The output removes CFF2 and variation tables, and Type 2 hinting is discarded.

## Plugins

```ts
import { definePlugin, optimize } from 'fontmin-rs'

const report = definePlugin({
  name: 'example:report',
  generateBundle(assets) {
    for (const asset of assets) {
      console.log(asset.path, asset.format, asset.contents.byteLength)
    }
  },
})

await optimize({
  input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
  plugins: [report],
})
```

Plugins can implement `buildStart`, `transform`, `generateBundle`, and `buildEnd`. Built-in plugins run core font operations through the native binding; custom plugins are useful for renaming, reports, extra file generation, and project-specific integrations.

Each hook receives a `PluginContext` with `cwd`, `resolve(path)`, `readFile(path)`, `writeFile(path, contents)`, `emitFile(asset)`, `warn(message)`, and `diagnostics`. Relative paths are resolved from `cwd`, and `writeFile` creates parent directories.

```ts
const manifest = definePlugin({
  name: 'example:manifest',
  async generateBundle(assets, ctx) {
    ctx.warn(`writing manifest for ${assets.length} assets`)
    await ctx.writeFile(
      'build/fontmin-manifest.json',
      JSON.stringify(
        assets.map(asset => ({
          format: asset.format,
          path: asset.path,
          size: asset.contents.byteLength,
        })),
        undefined,
        2,
      ),
    )
  },
})
```

## Fontmin-compatible chain

```ts
import Fontmin from 'fontmin-rs'

await new Fontmin()
  .src('fixtures/fonts/ttf/roboto-regular.ttf')
  .use(Fontmin.glyph({ text: 'Hello' }))
  .use(Fontmin.ttf2woff2())
  .dest('build')
  .runAsync()
```

This entry point is intended for migration. New projects should prefer `optimize(config)`, because configuration objects are easier to serialize, cache, and test.
