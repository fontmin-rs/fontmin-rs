# Configuration

`fontmin-rs build` and the TypeScript package support the same configuration
file names. Automatic discovery uses this exact order:

1. `fontmin.config.ts`
2. `fontmin.config.mts`
3. `fontmin.config.mjs`
4. `fontmin.config.cjs`
5. `fontmin.config.json`
6. `fontmin.config.jsonc`

Run `fontmin-rs init` to create a starter `fontmin.config.jsonc` in the current directory.

JSON and JSONC are dependency-free Rust CLI formats: the CLI parses them
entirely in Rust and does not start Node.js. Executable TS, MTS, MJS, and CJS
module configs require Node.js 22 or newer.

## JSONC Example

```jsonc
{
  "input": ["fixtures/fonts/ttf/roboto-regular.ttf"],
  "outDir": "build",
  "clean": true,
  "subset": {
    "text": "Hello",
    "basicText": true,
    "keepLayout": "conservative",
  },
  "outputs": [{ "format": "woff2" }, { "format": "woff" }, { "format": "css" }],
  "css": {
    "fontFamily": "Roboto",
    "fontPath": "./",
    "fontDisplay": "swap",
  },
  "delivery": {
    "slices": [
      { "name": "latin", "unicodeRanges": ["U+0000-00FF"] },
      { "name": "cjk", "unicodeRanges": ["U+4E00-9FFF"] },
    ],
  },
  "cache": {
    "enabled": true,
    "dir": "node_modules/.cache/fontmin-rs",
  },
  "otf": {
    "variationCoordinates": { "wght": 700, "opsz": 14 },
  },
}
```

Run it with:

```sh
fontmin-rs build --config fontmin.config.jsonc
```

For an SVG icon set, keep the input/output settings in JSONC and select the iconfont build preset from the command line:

```jsonc
{
  "input": ["icons/*.svg"],
  "outDir": "build/icons",
  "css": {
    "fontFamily": "Project Icons",
    "fontPath": "/icons",
  },
}
```

```sh
fontmin-rs build --config fontmin.config.jsonc --preset iconfont
```

## TypeScript Example

```ts
import { modernWeb } from 'fontmin-rs'

export default async () => ({
  input: ['fonts/*.ttf'],
  outDir: 'build',
  plugins: modernWeb({ text: 'Hello' }),
})
```

A module may export its configuration as `default` or as the named export
`config`. The export may be a configuration object or a synchronous or
asynchronous function returning one. When both exports exist, `default` takes
precedence.

Module configs are executable project code. The Rust CLI does not sandbox
them; only run configs you trust. They inherit the CLI's environment and
working directory so normal imports and environment lookups work.

## Rust CLI Module Boundary

The Rust CLI accepts JSON-compatible configuration data and serializable
descriptors for these built-ins: `glyph`, `unicodeSlices` (created by
`deliverySlices()`), `otf2ttf`,
`ttf2woff`, `ttf2woff2`, `ttf2eot`, `ttf2svg`, `svg2ttf`, `svgs2ttf`, and
`css`. The descriptors returned by `modernWeb()` and
`fontminCompatPreset()` are supported as well when their options stay within
this serializable built-in boundary.

The Rust CLI does not execute custom JavaScript plugin hooks. It rejects
custom plugin or transform functions, a function-valued `css.fontFamily`,
unknown built-in descriptors, and built-in options that the Rust pipeline
cannot represent. Diagnostics include the nearest field path, for example
`plugins[1].transform`, `plugins[0].native.options.fallback`, or
`css.fontFamily`. Runtime-only preset fields such as WOFF2 `fallback` are
therefore rejected by the Rust CLI. These restrictions apply to the Rust CLI
bridge; custom JavaScript plugins remain supported by the Node pipeline.

## Config Directory and Overrides

When `cwd` is omitted, both module and JSON/JSONC configs use the config file's
directory as `cwd`. Relative inputs, `outDir`, cache directories,
`subset.textFile`, and a built-in `glyph` plugin's `textFile` resolve from that
directory. An explicit `cwd` changes that base. The Rust CLI evaluates and
loads the config first, then applies command-line input, output, subset, cache,
preset, CSS, delivery, and variation overrides.

Load and run:

```ts
import { loadConfig, optimize } from 'fontmin-rs'

await optimize(await loadConfig())
```

## Key Fields

| Field              | Description                                                               |
| ------------------ | ------------------------------------------------------------------------- |
| `cwd`              | Base directory for relative paths; defaults to cwd or the config file dir |
| `input`            | Input file list; the CLI supports glob expansion                          |
| `outDir`           | Output directory                                                          |
| `clean`            | Clean the output directory before building                                |
| `preserveOriginal` | Whether original input assets are preserved                               |
| `runtime`          | Node pipeline runtime: `native` (default), `wasm`, or `auto`              |
| `otf`              | OTF-to-TTF options, including CFF2 variation coordinates                  |
| `subset`           | Subsetting options                                                        |
| `outputs`          | Output objects with format plus optional file name / extension overrides  |
| `css`              | `@font-face` CSS generation options                                       |
| `delivery`         | Named Unicode delivery slices                                             |
| `cache`            | Pipeline cache options                                                    |
| `plugins`          | Plugin list; the Rust CLI accepts serializable built-in descriptors       |

## Node Pipeline Runtime

The TypeScript `optimize()` pipeline accepts `runtime: 'native' | 'wasm' |
'auto'`. `native` is the default. `wasm` forces all built-in operations in the
pipeline to the packaged WASM module. `auto` selects one runtime for the whole
pipeline and falls back to WASM only when the native binding cannot load;
conversion errors never trigger fallback. Custom JavaScript plugins and all
file I/O remain Node-side.

The legacy `fallback` option on `ttf2woff2()` is treated as a pipeline runtime
when `runtime` is omitted. Matching values are allowed, different values throw
a conflict, multiple distinct plugin fallback values throw a conflict, and
`fallback: 'js'` is always unsupported. See the exact matrix in the
[Node API](../api/node#pipeline-runtime).

## Subsetting Options

| Field             | Description                                                              |
| ----------------- | ------------------------------------------------------------------------ |
| `text`            | Text whose glyphs should be kept                                         |
| `textFile`        | File content to read and append                                          |
| `unicodes`        | Unicode code points to keep                                              |
| `basicText`       | Keep the basic text character set                                        |
| `preserveHinting` | Preserve hinting information                                             |
| `trim`            | Trim unused glyphs; `false` keeps the original TTF data after validation |
| `keepNotdef`      | Keep the `.notdef` glyph                                                 |
| `keepLayout`      | `drop`, `conservative`, or `preserve`                                    |

## Unicode Delivery Slices

Set `delivery.slices` to generate one subset per named range group before the
selected output conversions. Slice names must be unique and use only letters,
digits, hyphens, or underscores. Each `unicodeRanges` entry accepts `U+HEX` or
`U+HEX-HEX` with one to six hexadecimal digits per endpoint.

The example above emits `roboto-regular-latin.*` and
`roboto-regular-cjk.*`. CSS output uses each slice's ranges in its own
`unicode-range` descriptor, overriding the global CSS `unicodeRanges` option
for that source.

## CFF/CFF2 OTF Inputs

The Rust build engine normalizes supported OTF input to static TrueType before
subsetting and Web conversion. Set `otf.variationCoordinates` for a CFF2
instance. Repeated `build --variation TAG=VALUE` flags override matching values
from this object while leaving other configured axes unchanged. CFF2 variation
tables and Type 2 hinting are not retained in the static output.

## CSS Options

| Field           | Description                                                          |
| --------------- | -------------------------------------------------------------------- |
| `fontFamily`    | `font-family` value for `@font-face`                                 |
| `fontPath`      | Path prefix for font files in CSS                                    |
| `fontDisplay`   | `font-display` value                                                 |
| `local`         | Whether to generate a local source                                   |
| `glyph`         | Generate icon glyph class rules                                      |
| `iconPrefix`    | Class prefix for generated glyph rules                               |
| `asFileName`    | Use SVG icon file names for class suffixes                           |
| `base64`        | Whether to inline font contents                                      |
| `target`        | CSS, SCSS, or Less output target                                     |
| `unicodeRanges` | Global `unicode-range` descriptors when sources do not define ranges |
