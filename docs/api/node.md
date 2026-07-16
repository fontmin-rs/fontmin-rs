# Node API

The `fontmin-rs` Node API has four pieces:

- Low-level native helpers that operate directly on `Uint8Array`.
- `defineConfig()` and `loadConfig()` helpers for typed project configuration.
- The `optimize(config)` pipeline for input files, plugins, caching, and output.
- A Fontmin-compatible default export for migrating existing Fontmin chains.

## Native helpers

```ts
import {
  analyzeCoverage,
  eotToTtf,
  generateFontFaceCss,
  inspect,
  otfToTtf,
  subsetTtf,
  svgFontToTtf,
  svgsToTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
  ttfToWoff2Async,
  validateWoff2,
  woff2ToTtf,
  woffToTtf,
} from 'fontmin-rs'
import { readFileSync, writeFileSync } from 'node:fs'

const input = readFileSync('fixtures/fonts/ttf/roboto-regular.ttf')
const coverage = analyzeCoverage(input, { text: 'A𠮷' })
const subset = subsetTtf(input, { text: 'Hello' })
const woff2 = ttfToWoff2(subset)
validateWoff2(woff2)
const decodedWoff2 = woff2ToTtf(woff2)
const info = inspect(woff2)

writeFileSync('build/roboto-subset.woff2', woff2)
writeFileSync('build/roboto-decoded-woff2.ttf', decodedWoff2)
console.log(info.format)
console.log(coverage.missing)
```

| Helper                                             | Operation                                                |
| -------------------------------------------------- | -------------------------------------------------------- |
| `analyzeCoverage(input, options)`                  | Report requested, supported, and missing Unicode values. |
| `subsetTtf(input, options)`                        | Subset TTF data by text, code points, or Unicode ranges. |
| `ttfToWoff(input, options)` / `woffToTtf(input)`   | Convert between TTF and WOFF 1.0.                        |
| `ttfToWoff2(input, options)` / `woff2ToTtf(input)` | Convert between TTF and WOFF2.                           |
| `ttfToWoff2Async(input, options)`                  | Encode WOFF2 with selectable native/WASM fallback.       |
| `validateWoff2(input)`                             | Validate the WOFF2 header and table directory.           |
| `ttfToEot(input, options)` / `eotToTtf(input)`     | Convert between TTF and EOT.                             |
| `ttfToSvg(input, options)`                         | Convert TTF data to an SVG font string.                  |
| `svgFontToTtf(input, options)`                     | Convert an SVG font string to TTF.                       |
| `svgsToTtf(icons, options)`                        | Build a TTF icon font from SVG icons.                    |
| `otfToTtf(input, options)`                         | Convert static CFF OTF or instantiate CFF2 OTF to TTF.   |
| `inspect(input)`                                   | Detect the format and read font metadata.                |
| `generateFontFaceCss(sources, options)`            | Generate `@font-face` CSS from named font sources.       |

`analyzeCoverage()` accepts the same `text`, `unicodes`, `unicodeRanges`, and
`basicText` selectors used for subsetting and returns `coveragePercent` plus
sorted `requested`, `supported`, and `missing` arrays. `subsetTtf()` and the
glyph presets accept `missingGlyphs: 'ignore' | 'warn' | 'error'`; `warn` is
the default and emits a `FONTMIN_MISSING_GLYPHS` process warning, while
`error` rejects incomplete coverage before subsetting.

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

`fallback: 'js'` remains unsupported. These fallback options on the low-level
helpers are separate from the runtime selection for the file-based
`optimize()` pipeline described below.

`validateWoff2(input)` validates the WOFF2 header and table directory, returning normally for valid input and throwing for invalid data. `inspect(woff2)` performs the same validation and reads sfnt metadata tables such as `name`, `head`, `hhea`, and `maxp`. `woff2ToTtf(input)` decodes WOFF2 back to TTF through the native binding.

## Browser WASM API

For browser-only processing, use the separate
[Browser WASM API](./wasm). It documents initialization, direct conversions,
the in-memory pipeline, custom browser plugins, and browser-only boundaries.

## Config helpers

Use `defineConfig()` to keep object configs type-checked, and `loadConfig()` to
load an explicit file or discover the first supported `fontmin.config.*` file.
When `cwd` is omitted, `loadConfig()` sets it to the config file's directory so
relative inputs, output paths, cache paths, and `textFile` values stay anchored
to the project config.

```ts
import { defineConfig, loadConfig, modernWeb, optimize } from 'fontmin-rs'

const config = defineConfig({
  input: ['fonts/*.ttf'],
  outDir: 'build',
  plugins: modernWeb({ text: 'Hello' }),
})

await optimize(config)
```

To discover and run a config file instead, call
`await optimize(await loadConfig())` from a project script.

See [Configuration](../guide/config) for file discovery, executable module
security, and the differences between the Rust CLI and Node configuration
models.

## optimize

```ts
import { modernWeb, optimize } from 'fontmin-rs'

await optimize({
  input: ['fonts/*.ttf'],
  outDir: 'build',
  runtime: 'auto',
  plugins: modernWeb({ text: 'Hello' }),
})
```

### Pipeline runtime

`runtime` controls every built-in font operation in one `optimize()` call:

- `native` is the default and requires the platform-specific native binding.
- `wasm` loads the packaged WASM module and forces every built-in operation to
  use it.
- `auto` selects native when the binding loads, otherwise selects WASM. It
  falls back only for a native binding load error. Invalid input, unsupported
  options, and conversion failures are returned without retrying in WASM.

One runtime is selected for the whole pipeline; built-in operations are never
mixed between native and WASM. Input discovery, file reads and writes, caching,
and custom JavaScript plugin hooks still run in Node. Only the built-in font
operations cross the selected native or WASM boundary.

For compatibility, `fallback` on built-in `ttf2woff2()` plugins can select the
pipeline runtime when `runtime` is omitted. The complete compatibility matrix
is:

| `runtime`                   | `ttf2woff2({ fallback })`                                               | Result                                            |
| --------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------- |
| omitted                     | omitted                                                                 | Select `native`                                   |
| `native`, `wasm`, or `auto` | omitted                                                                 | Select the configured runtime                     |
| omitted                     | `native`, `wasm`, or `auto`                                             | Select the fallback value as the pipeline runtime |
| a mode                      | the same mode                                                           | Select that mode                                  |
| a mode                      | a different mode                                                        | Throw a runtime/fallback conflict error           |
| any value                   | `js`                                                                    | Throw an unsupported fallback error               |
| any value                   | more than one distinct `native`, `wasm`, or `auto` value across plugins | Throw a conflicting fallback modes error          |

## modernWeb preset

```ts
import { modernWeb, optimize } from 'fontmin-rs'

await optimize({
  input: ['fonts/*.ttf'],
  outDir: 'build',
  runtime: 'auto',
  plugins: modernWeb({ text: 'Hello' }),
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

Built-in factories are `glyph`, `deliverySlices`, `otf2ttf`, `ttf2woff`,
`ttf2woff2`, `ttf2eot`, `ttf2svg`, `svg2ttf`, `svgs2ttf`, and `css`. They are
available from the package root and the `fontmin-rs/plugins` subpath.

### Unicode delivery slices

`deliverySlices()` replaces each TTF asset with one subset per named Unicode
range group. Put it after any required OTF normalization and before format
conversion and CSS generation. Each slice carries its ranges into the
generated `unicode-range` descriptor.

```ts
import { css, deliverySlices, optimize, ttf2woff2 } from 'fontmin-rs'

await optimize({
  input: ['fonts/roboto.ttf'],
  outDir: 'build',
  plugins: [
    deliverySlices([
      { name: 'latin', unicodeRanges: ['U+0000-00FF'] },
      { name: 'cjk', unicodeRanges: ['U+4E00-9FFF'] },
    ]),
    ttf2woff2({ clone: false }),
    css({ fontFamily: 'Roboto', fontPath: './' }),
  ],
})
```

Slice names must be unique and may contain only letters, digits, hyphens, and
underscores. Every slice needs at least one Unicode range.

### Custom plugins

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

Plugins can implement `buildStart`, `transform`, `generateBundle`, and `buildEnd`. Built-in plugins run core font operations through the pipeline's selected runtime; custom plugins remain in Node and are useful for renaming, reports, extra file generation, and project-specific integrations.

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
