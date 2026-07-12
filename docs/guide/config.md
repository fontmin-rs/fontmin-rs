# Configuration

`fontmin-rs build` can run from a configuration file. The Rust CLI currently supports `fontmin.config.json` and `fontmin.config.jsonc`; the TypeScript package also supports `fontmin.config.ts`, `.mts`, `.mjs`, and `.cjs`.

Run `fontmin-rs init` to create a starter `fontmin.config.jsonc` in the current directory.

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
import { defineConfig, modernWeb } from 'fontmin-rs'

export default defineConfig({
  input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
  outDir: 'build',
  runtime: 'auto',
  cache: { enabled: true },
  plugins: modernWeb({
    text: 'Hello',
    fontFamily: 'Roboto',
    fontPath: './',
  }),
})
```

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
| `plugins`          | TypeScript pipeline plugin list                                           |

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
