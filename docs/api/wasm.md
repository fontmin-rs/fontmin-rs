# Browser WASM API

`@fontmin-rs/wasm` runs fontmin-rs entirely in the browser. It is an
asynchronous, memory-only API: pass `Uint8Array` inputs and receive
`Uint8Array` outputs. It does not require a Node.js native binding.

## Install and initialize

```sh
pnpm add @fontmin-rs/wasm
```

Initialize once before using any conversion or pipeline API. With a normal
bundler or browser ESM import, the package loads its adjacent `.wasm` file
automatically. You can also pass the bytes or URL explicitly when your bundler
needs a custom asset strategy.

```ts
import { initWasm, isWasmInitialized } from '@fontmin-rs/wasm'

await initWasm()
console.log(isWasmInitialized()) // true
```

Repeated `initWasm()` calls reuse the same initialization promise. Use
`isWasmInitialized()` only for a synchronous status check; await `initWasm()`
before starting work.

## Direct transformations

Every direct helper returns a `Promise` and accepts in-memory data:

| Helper                                             | Operation                                                       |
| -------------------------------------------------- | --------------------------------------------------------------- |
| `analyzeCoverage(input, options)`                  | Report requested, supported, and missing Unicode values.        |
| `subsetTtf(input, options)`                        | Subset a TTF by text, Unicode values, or original GIDs.         |
| `createTtfSubsetPlan(input, options)`              | Resolve a cacheable plan bound to the source font.              |
| `subsetTtfWithPlan(input, plan)`                   | Execute a reusable plan and return actual subset statistics.    |
| `subsetTtfWithReport(input, options)`              | Subset a TTF and return size, table, and glyph mapping details. |
| `ttfToWoff(input, options)` / `woffToTtf(input)`   | Convert between TTF and WOFF 1.0.                               |
| `ttfToWoff2(input, options)` / `woff2ToTtf(input)` | Convert between TTF and WOFF2.                                  |
| `validateWoff2(input)`                             | Validate a WOFF2 header and table directory.                    |
| `ttfToEot(input, options)` / `eotToTtf(input)`     | Convert between TTF and EOT.                                    |
| `ttfToSvg(input, options)`                         | Convert TTF to an SVG font string.                              |
| `svgFontToTtf(input, options)`                     | Convert an SVG font string to TTF.                              |
| `svgsToTtf(icons, options)`                        | Build a TTF icon font from SVG icons.                           |
| `instantiateFont(input, options)`                  | Pin every variable-font axis and emit a static TTF.             |
| `otfToTtf(input, options)`                         | Convert static CFF OTF or instantiate CFF2 OTF into TTF.        |
| `inspect(input)`                                   | Read format and font metadata.                                  |
| `inspectCapabilities(input)`                       | Report structured color-font subset support.                    |
| `inspectCollection(input)`                         | List every face in a TTC/OTC collection.                        |
| `extractCollectionFace(input, faceIndex)`          | Extract one zero-based collection face as standalone SFNT.      |
| `generateFontFaceCss(sources, options)`            | Generate `@font-face` CSS.                                      |

`inspectCollection()` and `extractCollectionFace()` provide the same TTC/OTC
metadata and standalone TTF/OTF extraction as the Node API, asynchronously and
entirely in browser memory. The extracted bytes can be passed to any other WASM
transformation.

The capability report uses the same `subset`, `passthrough`, and `unsupported`
states as Node, including the explicit COLR v0/v1 distinction and incomplete
table-pair diagnostics.

SVG Font and icon conversion also matches Node: smooth curves (`S`/`T`),
elliptical arcs (`A`), relative variants, and supplementary Unicode code points
through cmap format 12 are supported in browser memory.

```ts
import {
  analyzeCoverage,
  createTtfSubsetPlan,
  initWasm,
  subsetTtf,
  subsetTtfWithPlan,
  subsetTtfWithReport,
  ttfToWoff2,
  validateWoff2,
} from '@fontmin-rs/wasm'

await initWasm()

const ttf = new Uint8Array(
  await (await fetch('/fonts/roboto.ttf')).arrayBuffer(),
)
const coverage = await analyzeCoverage(ttf, { text: 'A𠮷' })
const subset = await subsetTtf(ttf, { text: 'Hello' })
const woff2 = await ttfToWoff2(subset)

await validateWoff2(woff2)
console.log(coverage.missing)
```

`analyzeCoverage()` returns `coveragePercent` plus sorted `requested`,
`supported`, and `missing` arrays. `subsetTtf()` and glyph presets accept
`missingGlyphs: 'ignore' | 'warn' | 'error'`; `warn` is the default and calls
`console.warn`, while `error` rejects incomplete coverage before subsetting.

Pass `gids: [1, 7]` to retain original glyph IDs without requiring a Unicode
selector. Pass `glyphNames: ['A', 'space']` to select exact PostScript names;
`gidDDD` synthesized names are available when a font does not store names.

`await subsetTtfWithReport(input, options)` accepts the same selectors and
returns `{ data, report }`. The report includes source/subset sizes, retained
tables and glyph count, requested/supported/missing GIDs and glyph names,
glyph-name-to-original-GID, old-to-new and new-to-old GID mappings, and
Unicode-to-original-GID mappings.

`await createTtfSubsetPlan(input, options)` returns a JSON-safe plan containing
plan/source SHA-256 digests, resolved coverage and selector mappings, and seed GIDs.
`await subsetTtfWithPlan(input, plan)` reuses that work and returns the normal
`{ data, report }` result. Plans are rejected when the input bytes do not match
their recorded source, so they can be persisted without silently targeting a
different font; edited plans fail the same integrity check.

`preserveHinting`, `keepNotdef`, `retainGids`, `layout`, and `trim` have the same
observable semantics as the native helpers. Retained-ID subsets represent
empty intermediate slots as `null` in `report.newToOld`.
`retainGlyphNames: true` emits a version 2 `post` table in the new GID order;
the default version 3 table omits names. `retainLegacyCmap` and
`retainSymbolCmap` remap opt-in source encoding records; formats 0, 4, 6, 10,
12, and 13 are supported and normalized to format 4 or 12. `layout: 'preserve'`
rejects known contextual layout loss and unsupported FeatureVariations instead
of silently degrading.

`layoutFeatures`, `layoutScripts`, and `layoutLanguages` whitelist OpenType
layout tags in both GSUB and GPOS. Use `default` for DefaultLangSys;
three-character language tags are space-padded. Empty arrays retain all tags.

`nameIds` and `nameLanguages` filter OpenType `name` records. The latter uses
platform-specific numeric language IDs. Empty arrays retain every record, and
supplying both fields applies them with AND semantics.

`dropTables` removes named optional tables after rewriting, while
`passThroughTables` restores explicitly named source tables verbatim. Both use
exact four-byte printable ASCII tags. Required or rewritten tables and `DSIG`
are rejected; known glyph-indexed pass-through tables require `retainGids`.

`instantiateFont(input, { variationCoordinates })` accepts `glyf`-backed TTF,
WOFF, WOFF2, or EOT and CFF2 OTF. It fully pins every axis, using `fvar`
defaults for omitted tags, and returns a static TTF with stable glyph IDs.
Unknown, non-finite, and out-of-range values are rejected. Variation and
TrueType hinting tables are removed after their data has been evaluated.

`reduceVariationSpace(input, { axes, downgradeCff2 })` instead leaves unlisted
axes variable. Each `axes` value is either a numeric pin or
`{ min, max, default? }`; an omitted range default uses the original default
clamped into the new range. `variationSpace()` provides the same operation as
an in-memory browser plugin, and `modernWeb({ variationAxes })` inserts it
before subsetting and Web output generation.

The OTF `preserveHinting` and SVG `hinting` fields remain accepted compatibility
options. CFF/CFF2 Type 2 hints are not translated, and SVG conversion does not
generate TrueType hint instructions.

`generateFontFaceCss()` accepts named font sources in memory. Set `base64: true`
to embed source bytes as data URLs.

## Diagnostics

Direct helpers and built-in plugins reject with `FontminDiagnosticError` when
the Rust core reports a structured failure. Its `code` is a stable
machine-readable value such as `fontmin::invalid_font`, and its `message`
matches the Node native runtime for the shared malformed-input corpus.

```ts
import { FontminDiagnosticError, initWasm, inspect } from '@fontmin-rs/wasm'

await initWasm()

try {
  await inspect(new Uint8Array([0]))
} catch (error) {
  if (error instanceof FontminDiagnosticError) {
    console.error(error.code, error.message)
  }
}
```

Malformed input is rejected without crossing the WASM API as a Rust panic.
Initialization, browser-plugin, and JavaScript option errors that do not
originate in the Rust diagnostic layer remain their existing error types.

## In-memory pipeline

`optimizeBrowser()` applies plugins to named in-memory assets. It returns the
transformed and emitted assets; your application decides whether to download,
cache, or upload them.

```ts
import { initWasm, modernWeb, optimizeBrowser } from '@fontmin-rs/wasm'

await initWasm()

const assets = await optimizeBrowser({
  assets: [{ contents: ttf, fileName: 'roboto.ttf' }],
  plugins: modernWeb({
    text: 'Hello browser',
    fontFamily: 'Roboto',
    fontPath: './',
  }),
})

const woff2 = assets.find(asset => asset.fileName === 'roboto.woff2')
const css = assets.find(asset => asset.fileName === 'roboto.css')
```

Built-in plugins are `glyph`, `deliverySlices`, `ttf2woff`, `ttf2woff2`,
`ttf2eot`, `ttf2svg`, `otf2ttf`, `svg2ttf`, `svgs2ttf`, and `css`.

- `modernWeb()` normalizes supported CFF/CFF2 OTF input to static TTF, then
  combines subsetting, WOFF, WOFF2, and CSS output. Pass
  `variationCoordinates` to select a CFF2 instance; the source OTF is replaced.
- `fontminCompatPreset()` adds OTF conversion, EOT, and SVG output for classic
  Fontmin-compatible output sets.
- `css({ base64: true })` embeds the pipeline's in-memory font bytes.

### Unicode delivery slices

`deliverySlices()` replaces each TTF asset with one subset per named range and
preserves those ranges for CSS generation:

```ts
import {
  css,
  deliverySlices,
  optimizeBrowser,
  ttf2woff2,
} from '@fontmin-rs/wasm'

const assets = await optimizeBrowser({
  assets: [{ contents: ttf, fileName: 'roboto.ttf' }],
  plugins: [
    deliverySlices([
      { name: 'latin', unicodeRanges: ['U+0000-00FF'] },
      { name: 'cjk', unicodeRanges: ['U+4E00-9FFF'] },
    ]),
    ttf2woff2(),
    css({ fontFamily: 'Roboto', fontPath: './' }),
  ],
})
```

Slice names must be unique and contain only letters, digits, hyphens, or
underscores. Every slice requires at least one range.

## Custom plugins

Browser plugins can transform an asset, emit additional assets, and report
warnings. They cannot access a filesystem.

```ts
const rename = {
  name: 'example:rename',
  transform(asset, context) {
    context.warn(`processing ${asset.fileName}`)
    context.emitFile({
      contents: new Uint8Array([1]),
      fileName: 'manifest.bin',
    })
    return { ...asset, fileName: `web-${asset.fileName}` }
  },
}
```

The browser plugin surface is deliberately smaller than the Node pipeline:
there are no `buildStart`, `generateBundle`, or `buildEnd` hooks.

## Runtime boundary and browser support

This package has no path inputs, glob expansion, CLI, disk cache, output
directory, or Node.js filesystem hooks. Fetch inputs in your application and
handle returned bytes in memory.

The browser acceptance test loads generated WOFF2 bytes with `FontFace` in
Chromium, Firefox, and WebKit.
