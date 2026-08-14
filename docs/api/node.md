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
  subsetTtfWithReport,
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

| Helper                                             | Operation                                                          |
| -------------------------------------------------- | ------------------------------------------------------------------ |
| `analyzeCoverage(input, options)`                  | Report requested, supported, and missing Unicode values.           |
| `subsetTtf(input, options)`                        | Subset TTF data by text, Unicode selection, or original GIDs.      |
| `subsetTtfWithReport(input, options)`              | Subset TTF data and return size, table, and glyph mapping details. |
| `ttfToWoff(input, options)` / `woffToTtf(input)`   | Convert between TTF and WOFF 1.0.                                  |
| `ttfToWoff2(input, options)` / `woff2ToTtf(input)` | Convert between TTF and WOFF2.                                     |
| `ttfToWoff2Async(input, options)`                  | Encode WOFF2 with selectable native/WASM fallback.                 |
| `validateWoff2(input)`                             | Validate the WOFF2 header and table directory.                     |
| `ttfToEot(input, options)` / `eotToTtf(input)`     | Convert between TTF and EOT.                                       |
| `ttfToSvg(input, options)`                         | Convert TTF data to an SVG font string.                            |
| `svgFontToTtf(input, options)`                     | Convert an SVG font string to TTF.                                 |
| `svgsToTtf(icons, options)`                        | Build a TTF icon font from SVG icons.                              |
| `instantiateFont(input, options)`                  | Pin every variable-font axis and emit a static TTF.                |
| `otfToTtf(input, options)`                         | Convert static CFF OTF or instantiate CFF2 OTF to TTF.             |
| `inspect(input)`                                   | Detect the format and read font metadata.                          |
| `generateFontFaceCss(sources, options)`            | Generate `@font-face` CSS from named font sources.                 |

`analyzeCoverage()` accepts the same `text`, `unicodes`, `unicodeRanges`, and
`basicText` selectors used for subsetting and returns `coveragePercent` plus
sorted `requested`, `supported`, and `missing` arrays. `subsetTtf()` and the
glyph presets accept `missingGlyphs: 'ignore' | 'warn' | 'error'`; `warn` is
the default and emits a `FONTMIN_MISSING_GLYPHS` process warning, while
`error` rejects incomplete coverage before subsetting.

`subsetTtf(input, { gids: [1, 7] })` selects original glyph IDs directly and
can be combined with text, code points, or ranges. `glyphNames: ['A', 'space']`
selects exact PostScript glyph names; fonts without stored names expose stable
`gidDDD` synthesized names.

`subsetTtfWithReport()` accepts the same options and returns `{ data, report }`.
The report records source and subset sizes, retained tables and glyph count,
requested/supported/missing GIDs and glyph names, glyph-name-to-original-GID,
old-to-new and new-to-old GID mappings, and the Unicode-to-original-GID mapping
used by the subset. This is the stable API
for downstream CSS manifests, glyph diagnostics, and cache metadata.

Subset policy options are observable and shared with WASM: `preserveHinting`
keeps the `cvt `, `fpgm`, and `prep` tables, `keepNotdef: false` emits an empty
glyph-zero outline, and `retainGids: true` keeps original IDs while emitting
empty intermediate glyph slots. Empty slots are `null` in `report.newToOld`.
`retainGlyphNames: true` rewrites a version 2 `post` table in the new GID order;
the default version 3 table intentionally omits names for smaller output.
`retainLegacyCmap` and `retainSymbolCmap` opt into source encoding records that
the default Unicode-only `cmap` omits. Their surviving mappings are rewritten
to the subset's new GIDs; source formats 0, 4, 6, 10, 12, and 13 are supported
and normalized to format 4 or 12 while retaining record identity and language.
`keepLayout` selects dropped, conservatively remapped, or strict layout
handling. Strict mode rejects known contextual loss and unsupported
FeatureVariations instead of silently degrading. `trim: false` returns the
validated source bytes unchanged.

`layoutFeatures`, `layoutScripts`, and `layoutLanguages` accept OpenType tag
whitelists applied to both GSUB and GPOS. Empty or omitted arrays keep all
entries. Use `default` in `layoutLanguages` for each selected script's
DefaultLangSys; three-character language tags such as `ENG` are space-padded.

`nameIds` and `nameLanguages` filter OpenType `name` records. Language IDs are
platform-specific numeric values (for example, Windows English is `0x0409`).
Empty or omitted arrays retain every record; when both filters are present, a
record must match both. Format 1 language-tag records remain valid after
filtering.

`dropTables` removes named optional tables after the normal rewrite pipeline;
`passThroughTables` copies explicitly named source tables verbatim and rebuilds
the SFNT checksums. Tags are exactly four printable ASCII bytes. Required and
subset-rewritten tables cannot be overridden, `DSIG` cannot be retained, and
known glyph-indexed pass-through tables require `retainGids: true`. Missing
source tags are ignored so one policy can be shared by a mixed font batch.
Explicit unknown tags are treated as caller-asserted custom metadata.

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

## Diagnostics

Native helpers and native-backed built-in plugins throw
`FontminDiagnosticError` for structured fontmin-rs failures. Its `code` is a
stable machine-readable value such as `fontmin::invalid_font`; `message`
contains the human-readable detail. Native and forced-WASM runtimes return the
same code and message for the shared malformed-input corpus.

```ts
import { FontminDiagnosticError, inspect } from 'fontmin-rs'

try {
  inspect(new Uint8Array([0]))
} catch (error) {
  if (error instanceof FontminDiagnosticError) {
    console.error(error.code, error.message)
  }
}
```

Malformed input is rejected without crossing the public API as a Rust panic.
Runtime-loading, JavaScript plugin, and option-validation failures that do not
originate in the Rust diagnostic layer remain their existing error types.

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
`variationCoordinates` to fully instance either a `glyf` variable TTF or CFF2
OTF before subsetting; omitted axes use their defaults. It does not generate
EOT or SVG; add `ttf2eot()` or `ttf2svg()` explicitly if you need those formats.

Use `variationAxes` instead when the output must remain variable. Numeric
values pin axes, range objects narrow retained axes, and unlisted axes remain
variable:

```ts
modernWeb({
  variationAxes: {
    wdth: 100,
    wght: { min: 300, max: 700, default: 500 },
  },
})
```

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

Use `instantiateFont()` when the input is already variable. It accepts
`glyf`-backed TTF, WOFF, WOFF2, or EOT and CFF2 OTF, and always returns one
static TTF:

```ts
const staticBold = instantiateFont(variableFont, {
  variationCoordinates: { wght: 700 },
})
```

Every axis is pinned; omitted axes use their `fvar` defaults. Unknown,
non-finite, and out-of-range values are rejected instead of clamped. Glyph IDs
remain stable. Variation tables and TrueType hinting programs are removed after
evaluation because they no longer describe the static outlines.

`reduceVariationSpace()` preserves a variable font while pinning selected axes
or narrowing their ranges. It accepts TTF, OTF, WOFF, WOFF2, and EOT input;
wrapped input is returned as unwrapped SFNT data. Set `downgradeCff2: true` to
convert CFF2 to CFF1 when every axis is pinned.

```ts
const reduced = reduceVariationSpace(variableFont, {
  axes: {
    wdth: 100,
    wght: { min: 300, max: 700 },
  },
})
```

`otfToTtf({ preserveHinting: true })` and
`svgFontToTtf({ hinting: true })` remain accepted compatibility options. The
former cannot translate CFF/CFF2 Type 2 hints and the latter does not generate
TrueType instructions, so these values intentionally do not alter converted
outlines.

## Plugins

Built-in factories are `glyph`, `deliverySlices`, `variationSpace`, `otf2ttf`, `ttf2woff`,
`ttf2woff2`, `ttf2eot`, `ttf2svg`, `svg2ttf`, `svgs2ttf`, and `css`. They are
available from the package root and the `fontmin-rs/plugins` subpath.

`variationSpace(options)` exposes the same reduction in a composable pipeline.
It replaces the input by default; `clone: true` emits a `*-reduced.ttf` or
`*-reduced.otf` sibling.

For compatibility, `otf2ttf()` keeps its established name, but when
`variationCoordinates` is present it also instances variable TTF assets. With
the default `clone: true`, the static sibling is named `*-instance.ttf`; use
`clone: false` to replace the variable input in place.

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
