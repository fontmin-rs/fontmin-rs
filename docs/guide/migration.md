# Migration From Fontmin

`fontmin-rs` keeps the main Fontmin workflow familiar while moving the heavy font operations into Rust and N-API bindings. This page is for projects that already use `fontmin` in build scripts and want a staged migration.

## Upgrade From 0.3 To 1.0

No public API is removed or renamed in `1.0`. The independently validated
contract retains the `0.3` CLI commands and flags, Node and browser exports,
configuration fields, stable diagnostic codes, and generated file naming
rules.

Install the stable release:

```sh
pnpm add fontmin-rs@latest
pnpm add @fontmin-rs/wasm@latest
```

The reviewed candidate remains available for reproducing the promotion
evidence:

```sh
pnpm add fontmin-rs@1.0.0-rc.1
pnpm add @fontmin-rs/wasm@1.0.0-rc.1
```

The support boundary is now machine-readable in
[`contracts/support.json`](https://github.com/fontmin-rs/fontmin-rs/blob/main/contracts/support.json):

- Node.js 22.18, 24, and 26 are release-blocking; the package engine remains
  `>=22.18.0`.
- `runtime: "native"` remains the default. `"auto"` falls back to WASM only
  when the native binding cannot load, not after a processing error.
- The same eight native targets, Chromium/Firefox/WebKit browser engines, Rust
  1.88.0 MSRV, diagnostics, and generated naming templates remain supported.
- The Fontmin-compatible default export, `glyph({ hinting })` alias, and
  `ttf2woff2({ fallback })` runtime compatibility path remain available. None
  is eligible for removal in `1.0`.

Run the [standalone compatibility projects](../compatibility.md) against the
exact version selected for production before upgrading.

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

As in classic Fontmin, a compatibility chain with no `.use()` calls emits the
original TTF plus EOT, WOFF, WOFF2, SVG font, and CSS outputs. It does not
subset glyphs unless `Fontmin.glyph()` is added explicitly.

Calling `.src()` or `.dest()` without arguments returns the arguments last
configured on that compatibility chain, matching classic Fontmin's getter
behavior.

The package also exports the classic `plugins`, `mime`, and `util` helpers.
They are available as named exports and as `Fontmin.plugins`, `Fontmin.mime`,
and `Fontmin.util` on the compatibility class.

Plugin factories on the compatibility class retain classic defaults:
`Fontmin.glyph()` preserves TrueType hinting, `Fontmin.css()` does not add a
`local()` source unless requested, `Fontmin.otf2ttf()` replaces its OTF input,
and an empty `Fontmin.glyph()` is a pass-through. Named plugin exports keep the
modern `fontmin-rs` defaults. On the compatibility class,
`Fontmin.css({ asFileName: true })` uses the source file stem as the
`font-family`, matching classic Fontmin.

The compatibility `glyph` plugin also accepts Fontmin's mutable `use(ttf)`
callback. When that plugin precedes `Fontmin.css()`, a `fontFamily(info, ttf)`
callback receives the rewritten TTF object as its second argument:

```ts
new Fontmin()
  .src('fonts/roboto.ttf')
  .use(Fontmin.glyph({
    text: 'Hello',
    use(ttf) {
      ttf.setName({ fontFamily: 'Roboto Subset' })
    },
  }))
  .use(Fontmin.css({
    fontFamily(info, ttf) {
      return ttf.name.fontFamily || info.fontFile
    },
  }))
```

These mutable callbacks are Node compatibility features backed by the same
`fonteditor-core@2.4.1` object model as the locked Fontmin baseline. The modern
named `glyph()` and `css()` exports remain typed, runtime-neutral operations.

`run(callback)` returns an object-mode Node.js stream while retaining the
callback result. Its data events currently contain typed `FontAsset` objects;
use `runAsync()` when a stream is not required.

Legacy Gulp pipelines and plugins that depend on Vinyl file methods can opt in
to the dedicated adapter. It uses `vinyl-fs` for source and destination options,
returns real Vinyl files, and accepts ordinary Vinyl Transform streams between
the typed conversion plugins:

```ts
import { Transform } from 'node:stream'
import Fontmin from 'fontmin-rs/vinyl'

await new Fontmin()
  .src('fonts/*.ttf', { base: 'fonts' })
  .use(Fontmin.glyph({ text: 'Hello' }))
  .use(() => new Transform({ objectMode: true, transform(file, _, done) {
    file.stem = `${file.stem}-subset`
    done(null, file)
  }}))
  .dest('build', { overwrite: true })
  .runAsync()
```

The Vinyl adapter buffers each typed plugin segment. Vinyl files whose
`contents` are streams are rejected; use the default buffered `vinyl-fs.src()`
mode. Keep using the main entry for new code that does not require Gulp/Vinyl.

Use `optimize(config)` for new or larger migrations. It is easier to test, serialize, cache, and share with CLI config files:

```ts
import { css, glyph, optimize, ttf2woff2 } from 'fontmin-rs'

await optimize({
  input: ['fonts/roboto.ttf'],
  outDir: 'build',
  runtime: 'auto',
  cache: { enabled: true },
  plugins: [
    glyph({ text: 'Hello' }),
    ttf2woff2(),
    css({ fontFamily: 'Roboto', fontPath: './' }),
  ],
})
```

## Plugin Mapping

| Fontmin-style operation | `fontmin-rs` API                         | Notes                                                                                                                                       |
| ----------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `glyph(options)`        | `glyph(options)`                         | Supports text, text files, Unicode lists, and layout modes.                                                                                 |
| `ttf2woff(options)`     | `ttf2woff(options)` / `ttfToWoff()`      | Supports WOFF metadata and private data in the low-level API.                                                                               |
| `ttf2woff2(options)`    | `ttf2woff2(options)` / `ttfToWoff2()`    | Pipeline `native`, `wasm`, and `auto` modes are supported; legacy plugin `fallback` selects the pipeline runtime when `runtime` is omitted. |
| `ttf2eot(options)`      | `ttf2eot(options)` / `ttfToEot()`        | Intended for legacy IE compatibility.                                                                                                       |
| `ttf2svg(options)`      | `ttf2svg(options)` / `ttfToSvg()`        | Emits SVG font output.                                                                                                                      |
| `svg2ttf(options)`      | `svg2ttf(options)` / `svgFontToTtf()`    | Converts SVG font input to TTF.                                                                                                             |
| `svgs2ttf(file, options)` | `svgs2ttf(file, options)` / `svgs2ttf(options)` / `svgsToTtf()` | Combines multiple SVG icons into one TTF iconfont; the classic output-file overload and the options-only form are both supported. |
| `css(options)`          | `css(options)` / `generateFontFaceCss()` | Supports CSS, SCSS, Less targets and optional glyph classes.                                                                                |

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

The iconfont preset does not support delivery slices.

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

- The main compatibility chain emits typed `FontAsset` objects. Use the opt-in `fontmin-rs/vinyl` entry when an existing build requires real Vinyl files, `vinyl-fs` options, or Transform plugins; prefer `runAsync()` and `optimize(config)` for new code.
- Plugins created with `definePlugin()` receive typed assets and a context object. Plugins passed to `fontmin-rs/vinyl` may instead be ordinary Vinyl Transform streams. Both adapters and all file I/O remain Node-side even when built-in operations use WASM.
- Rust plugins should use `AssetMeta.unicode`, `AssetMeta.css_glyphs`, and `AssetMeta.css_unicode_ranges` for metadata consumed by built-ins. `AssetMeta.custom` remains the extension map for third-party keys.
- OTF inspection is supported. `otf2ttf()` / `otfToTtf()` convert static CFF OTF fonts and default/explicit CFF2 instances to static TrueType `glyf` fonts, and can also rewrite glyf-backed OTF wrappers. CFF2 and variation tables are removed from the static output.
- `optimize({ runtime })` selects one runtime for every built-in operation: `native` is the default, `wasm` forces WASM, and `auto` falls back only when the native binding cannot load. Conversion failures never cause a retry in WASM.
- For legacy `ttf2woff2({ fallback })` plugins, an omitted pipeline `runtime` inherits `native`, `wasm`, or `auto`; a matching explicit runtime is accepted, a different runtime or distinct plugin fallback values conflict, and `js` remains unsupported. The low-level `ttfToWoff2Async(input, { fallback: 'wasm' | 'auto' })` remains available independently.
- Native packages are platform-specific optional dependencies. If installation fails, remove `node_modules` and the lockfile for the package manager involved, then reinstall.

## Verification Checklist

1. Compare generated file names and extensions.
2. Inspect CSS `font-family`, `font-path`, and `font-display` output.
3. Run `fontmin-rs inspect <font> --json` on generated fonts.
4. Load generated WOFF2/WOFF/CSS in your app or browser test.
5. Enable cache only after the uncached build output is correct.
