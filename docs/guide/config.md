# Configuration

`fontmin-rs build` and the TypeScript package share configuration files. Discovery uses this order: `fontmin.config.ts`, `.mts`, `.mjs`, `.cjs`, `.json`, then `.jsonc`.

JSON and JSONC are parsed directly and do not require Node.js. Module configs require Node.js 22 or newer and may export `default` or `config` as an object, synchronous factory, or asynchronous factory. Module configs are executable project code and should only be used when trusted. The Rust CLI accepts serializable built-in plugins and presets; custom JavaScript hooks and function-valued CSS families remain Node-pipeline-only.

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

export default async () =>
  defineConfig({
    input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
    outDir: 'build',
    cache: { enabled: true },
    plugins: modernWeb({
      text: 'Hello',
      fontFamily: 'Roboto',
      fontPath: './',
    }),
  })
```

Module factories may also be synchronous, and a named `config` export is
accepted when no default export exists.

The Rust CLI can execute the same module directly:

```sh
fontmin-rs build --config fontmin.config.ts
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
| `otf`              | OTF-to-TTF options, including CFF2 variation coordinates                  |
| `subset`           | Subsetting options                                                        |
| `outputs`          | Output objects with format plus optional file name / extension overrides  |
| `css`              | `@font-face` CSS generation options                                       |
| `delivery`         | Named Unicode delivery slices                                             |
| `cache`            | Native pipeline cache options                                             |
| `plugins`          | Built-in descriptors; custom JS hooks are supported by Node only          |
| `runtime`          | Node `optimize()` runtime: `native`, `wasm`, or `auto`                    |

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
