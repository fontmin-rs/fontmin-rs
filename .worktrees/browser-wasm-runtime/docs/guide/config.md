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
  "cache": {
    "enabled": true,
    "dir": "node_modules/.cache/fontmin-rs",
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
| `subset`           | Subsetting options                                                        |
| `outputs`          | Output objects with format plus optional file name / extension overrides  |
| `css`              | `@font-face` CSS generation options                                       |
| `cache`            | Native pipeline cache options                                             |
| `plugins`          | TypeScript pipeline plugin list                                           |

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

## CSS Options

| Field         | Description                                |
| ------------- | ------------------------------------------ |
| `fontFamily`  | `font-family` value for `@font-face`       |
| `fontPath`    | Path prefix for font files in CSS          |
| `fontDisplay` | `font-display` value                       |
| `local`       | Whether to generate a local source         |
| `glyph`       | Generate icon glyph class rules            |
| `iconPrefix`  | Class prefix for generated glyph rules     |
| `asFileName`  | Use SVG icon file names for class suffixes |
| `base64`      | Whether to inline font contents            |
| `target`      | CSS, SCSS, or Less output target           |
