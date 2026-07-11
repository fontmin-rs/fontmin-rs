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
import { initWasm } from '@fontmin-rs/wasm'

await initWasm()
```

## Direct transformations

Every direct helper returns a `Promise` and accepts in-memory data:

| Helper                                             | Operation                                                |
| -------------------------------------------------- | -------------------------------------------------------- |
| `subsetTtf(input, options)`                        | Subset a TTF by text or Unicode values.                  |
| `ttfToWoff(input, options)` / `woffToTtf(input)`   | Convert between TTF and WOFF 1.0.                        |
| `ttfToWoff2(input, options)` / `woff2ToTtf(input)` | Convert between TTF and WOFF2.                           |
| `validateWoff2(input)`                             | Validate a WOFF2 header and table directory.             |
| `ttfToEot(input, options)` / `eotToTtf(input)`     | Convert between TTF and EOT.                             |
| `ttfToSvg(input, options)`                         | Convert TTF to an SVG font string.                       |
| `svgFontToTtf(input, options)`                     | Convert an SVG font string to TTF.                       |
| `svgsToTtf(icons, options)`                        | Build a TTF icon font from SVG icons.                    |
| `otfToTtf(input, options)`                         | Convert static CFF OTF or instantiate CFF2 OTF into TTF. |
| `inspect(input)`                                   | Read format and font metadata.                           |
| `generateFontFaceCss(sources, options)`            | Generate `@font-face` CSS.                               |

```ts
import {
  initWasm,
  subsetTtf,
  ttfToWoff2,
  validateWoff2,
} from '@fontmin-rs/wasm'

await initWasm()

const ttf = new Uint8Array(
  await (await fetch('/fonts/roboto.ttf')).arrayBuffer(),
)
const subset = await subsetTtf(ttf, { text: 'Hello' })
const woff2 = await ttfToWoff2(subset)

await validateWoff2(woff2)
```

`generateFontFaceCss()` accepts named font sources in memory. Set `base64: true`
to embed source bytes as data URLs.

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

Built-in plugins are `glyph`, `ttf2woff`, `ttf2woff2`, `ttf2eot`, `ttf2svg`,
`otf2ttf`, `svg2ttf`, `svgs2ttf`, and `css`.

- `modernWeb()` normalizes supported CFF/CFF2 OTF input to static TTF, then
  combines subsetting, WOFF, WOFF2, and CSS output. Pass
  `variationCoordinates` to select a CFF2 instance; the source OTF is replaced.
- `fontminCompatPreset()` adds OTF conversion, EOT, and SVG output for classic
  Fontmin-compatible output sets.
- `css({ base64: true })` embeds the pipeline's in-memory font bytes.

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
