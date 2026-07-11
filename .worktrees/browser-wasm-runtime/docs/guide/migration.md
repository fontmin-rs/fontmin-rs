# Migration From Fontmin

`fontmin-rs` keeps the main Fontmin workflow familiar while moving the heavy font operations into Rust and N-API bindings. This page is for projects that already use `fontmin` in build scripts and want a staged migration.

## Install

```sh
pnpm add fontmin-rs
```

During migration you can keep `fontmin` installed and move one build target at a time. The package name and native platform packages are separate, so both tools can exist in the same repository.

## Choose An Entry Point

Use the Fontmin-compatible chain when you want the smallest code change:

```ts
import Fontmin from 'fontmin-rs'

await new Fontmin()
  .src('fonts/roboto.ttf')
  .use(Fontmin.glyph({ text: 'Hello' }))
  .use(Fontmin.ttf2woff2())
  .use(Fontmin.css({ fontFamily: 'Roboto', fontPath: './' }))
  .dest('build')
  .runAsync()
```

Use `optimize(config)` for new or larger migrations. It is easier to test, serialize, cache, and share with CLI config files:

```ts
import { css, glyph, optimize, ttf2woff2 } from 'fontmin-rs'

await optimize({
  input: ['fonts/roboto.ttf'],
  outDir: 'build',
  cache: { enabled: true },
  plugins: [
    glyph({ text: 'Hello' }),
    ttf2woff2(),
    css({ fontFamily: 'Roboto', fontPath: './' }),
  ],
})
```

## Plugin Mapping

| Fontmin-style operation | `fontmin-rs` API                         | Notes                                                                                                                |
| ----------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `glyph(options)`        | `glyph(options)`                         | Supports text, text files, Unicode lists, and layout modes.                                                          |
| `ttf2woff(options)`     | `ttf2woff(options)` / `ttfToWoff()`      | Supports WOFF metadata and private data in the low-level API.                                                        |
| `ttf2woff2(options)`    | `ttf2woff2(options)` / `ttfToWoff2()`    | `native` and `auto` fallback modes use native today. `wasm` and `js` currently report clear unsupported diagnostics. |
| `ttf2eot(options)`      | `ttf2eot(options)` / `ttfToEot()`        | Intended for legacy IE compatibility.                                                                                |
| `ttf2svg(options)`      | `ttf2svg(options)` / `ttfToSvg()`        | Emits SVG font output.                                                                                               |
| `svg2ttf(options)`      | `svg2ttf(options)` / `svgFontToTtf()`    | Converts SVG font input to TTF.                                                                                      |
| `svgs2ttf(options)`     | `svgs2ttf(options)` / `svgsToTtf()`      | Combines multiple SVG icons into one TTF iconfont.                                                                   |
| `css(options)`          | `css(options)` / `generateFontFaceCss()` | Supports CSS, SCSS, Less targets and optional glyph classes.                                                         |

For a broad Fontmin-style output group, use `fontminCompatPreset(options)`:

```ts
import { fontminCompatPreset, optimize } from 'fontmin-rs'

await optimize({
  input: ['fonts/roboto.ttf'],
  outDir: 'build',
  plugins: fontminCompatPreset({
    text: 'Hello',
    fontFamily: 'Roboto',
    fontPath: './',
  }),
})
```

For modern web output only, use `modernWeb(options)`. It emits WOFF2, WOFF, and CSS without EOT or SVG.

## CLI Replacement

Many Fontmin build scripts can move to the CLI first:

```sh
fontmin-rs build fonts/roboto.ttf \
  --out-dir build \
  --text "Hello" \
  --preset compat \
  --font-family Roboto \
  --font-path ./
```

Use `--preset modern-web` for WOFF2, WOFF, and CSS. Use `--preset iconfont` with SVG icon inputs:

```sh
fontmin-rs build icons/home.svg icons/user.svg \
  --out-dir build/icons \
  --preset iconfont \
  --font-family "Project Icons"
```

## Config Files

Move repeated CLI options into `fontmin.config.jsonc`:

```jsonc
{
  "input": ["fonts/roboto.ttf"],
  "outDir": "build",
  "clean": true,
  "subset": {
    "text": "Hello",
    "basicText": true,
  },
  "outputs": [{ "format": "woff2" }, { "format": "woff" }, { "format": "css" }],
  "css": {
    "fontFamily": "Roboto",
    "fontPath": "./",
    "fontDisplay": "swap",
  },
  "cache": {
    "enabled": true,
  },
}
```

Then run:

```sh
fontmin-rs build --config fontmin.config.jsonc
```

## Behavior Differences

- The compatibility chain supports common Fontmin-style usage, but it is not a Node stream clone. Prefer `runAsync()` and `optimize(config)` for new code.
- Custom JavaScript plugins receive typed asset and context objects instead of vinyl streams.
- OTF inspection is supported. `otf2ttf()` / `otfToTtf()` convert static CFF OTF fonts and default/explicit CFF2 instances to static TrueType `glyf` fonts, and can also rewrite glyf-backed OTF wrappers. CFF2 and variation tables are removed from the static output.
- WOFF2 `fallback: 'wasm'` and `fallback: 'js'` are reserved for future fallback implementations and currently fail explicitly.
- Native packages are platform-specific optional dependencies. If installation fails, remove `node_modules` and the lockfile for the package manager involved, then reinstall.

## Verification Checklist

1. Compare generated file names and extensions.
2. Inspect CSS `font-family`, `font-path`, and `font-display` output.
3. Run `fontmin-rs inspect <font> --json` on generated fonts.
4. Load generated WOFF2/WOFF/CSS in your app or browser test.
5. Enable cache only after the uncached build output is correct.
