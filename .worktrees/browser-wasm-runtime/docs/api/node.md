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

`ttfToWoff2(input, { fallback })` accepts `native` and `auto`, both of which use the native binding today. The synchronous Node API does not load WASM automatically; use the explicit browser package below when native modules are unavailable.

`validateWoff2(input)` validates the WOFF2 header and table directory, returning normally for valid input and throwing for invalid data. `inspect(woff2)` performs the same validation and reads sfnt metadata tables such as `name`, `head`, `hhea`, and `maxp`. `woff2ToTtf(input)` decodes WOFF2 back to TTF through the native binding.

## Browser WASM API

Install `@fontmin-rs/wasm` for browser-only processing. Initialize it once,
then pass named in-memory assets to `optimizeBrowser()`:

```ts
import { initWasm, modernWeb, optimizeBrowser } from '@fontmin-rs/wasm'

await initWasm()
const assets = await optimizeBrowser({
  assets: [{ contents: ttfBytes, fileName: 'roboto.ttf' }],
  plugins: modernWeb({ text: 'Hello', fontFamily: 'Roboto' }),
})
```

The browser package exposes asynchronous conversion and inspection helpers for
the supported formats, plus an in-memory built-in pipeline. It does not accept
paths, globs, CLI options, disk cache, or Node filesystem plugin hooks.

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

`modernWeb()` is equivalent to combining `glyph()`, `ttf2woff()`, `ttf2woff2()`, and `css()`. It does not generate EOT or SVG; add `ttf2eot()` or `ttf2svg()` explicitly if you need those formats.

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
