# Features

fontmin-rs is currently in beta. This page records the implemented public
surface and the boundaries that matter when choosing an entry point. For a
first project, continue with [Get Started](./getting-started).

## Entry Points

| Entry point           | Best for                                  | Runtime model                                     |
| --------------------- | ----------------------------------------- | ------------------------------------------------- |
| `fontmin-rs` CLI      | Shell scripts, CI, and one-off processing | Native Rust executable                            |
| `fontmin-rs` Node API | Build integrations and custom pipelines   | Native binding, forced WASM, or native-first auto |
| `@fontmin-rs/wasm`    | Browser and worker applications           | Asynchronous, in-memory WASM                      |

The npm package also exposes the CLI command, typed configuration helpers,
built-in plugin declarations, presets, and a Fontmin-compatible chain.

## Font Processing

| Capability          | Supported operations                                                                                  |
| ------------------- | ----------------------------------------------------------------------------------------------------- |
| Subsetting          | Keep glyphs by text, text file, code points, basic text, or Unicode range groups.                     |
| Web font conversion | TTF to and from WOFF or WOFF2, including WOFF metadata and private data blocks.                       |
| Legacy conversion   | TTF to and from EOT, plus TTF to SVG font and SVG font to TTF.                                        |
| OpenType conversion | Convert static CFF OTF or instantiate CFF2 variable OTF as a static TrueType `glyf` font.             |
| Icon fonts          | Combine multiple SVG icons into TTF and generate optional glyph class CSS.                            |
| CSS generation      | Generate `@font-face` CSS, SCSS, or Less with local sources, Base64 data, and `unicode-range` values. |
| Inspection          | Detect and inspect TTF, OTF, WOFF, WOFF2, and EOT metadata.                                           |
| Character coverage  | Report requested, supported, and missing Unicode values before subsetting; optionally fail strictly.  |

The low-level Node and browser APIs expose these operations directly. The
file-based pipelines compose the same operations through built-in plugins.
See the [Node API](../api/node) and [Browser WASM API](../api/wasm) for the
complete callable surface.

## Pipelines and Presets

- `optimize(config)` discovers file inputs, applies plugins, reuses matching
  cache entries, and writes outputs in Node.js.
- `optimizeBrowser(config)` applies built-in and custom browser plugins to
  named in-memory assets without filesystem access.
- `modernWeb(options)` normalizes supported OTF input, subsets it, and emits
  WOFF, WOFF2, and CSS.
- `fontminCompatPreset(options)` follows the classic Fontmin output order for
  EOT, SVG, WOFF, WOFF2, and CSS.
- The CLI `iconfont` preset groups SVG icon inputs into `iconfont.ttf` and
  `iconfont.css`.
- `deliverySlices()` and CLI delivery slices create named subsets with matching
  CSS `unicode-range` descriptors.

Built-in plugin factories are `glyph`, `deliverySlices`, `otf2ttf`,
`ttf2woff`, `ttf2woff2`, `ttf2eot`, `ttf2svg`, `svg2ttf`, `svgs2ttf`, and
`css`.

Node `optimize(config)` selects one built-in runtime for the whole pipeline:
`native`, `wasm`, or native-first `auto`. Cache entries are separated by the
selected runtime. Filesystem work and custom JavaScript hooks always remain in
Node.js.

## CLI and Configuration

The CLI provides `init`, `coverage`, `subset`, `convert`, `build`, `bench`,
`inspect`, and `doctor` commands. It can load JSON, JSONC, TS, MTS, MJS, and CJS configuration
files; executable module configs require Node.js 22 or newer.

Use [Command Line](./cli) for command flags and examples. Use
[Configuration](./config) for file discovery, output control, caching,
subsetting modes, OTF variation coordinates, and the boundary between Rust
built-ins and custom Node.js plugins.

## Compatibility and Limits

- EOT exists for older Internet Explorer compatibility; modern projects should
  normally prefer WOFF2 with WOFF as a fallback.
- CFF/CFF2 conversion always produces a static TTF. It removes variation
  tables, and Type 2 hinting is not preserved.
- `modernWeb()` intentionally emits WOFF, WOFF2, and CSS only. Add legacy
  plugins explicitly or use `fontminCompatPreset()` when required.
- The browser package has no path inputs, glob expansion, CLI, disk cache,
  output directory, or filesystem hooks.
- Custom Node.js plugins may use filesystem and diagnostic context helpers.
  Browser plugins use a smaller, memory-only hook surface.

Existing Fontmin users should read [Migration From Fontmin](./migration).
Internal package and runtime boundaries are documented in
[Architecture](../architecture).

## Project Status

Current development is focused on release hardening, richer diagnostics,
performance, and broader Fontmin compatibility. The concrete stable-release
criteria are tracked in the [Roadmap to 1.0](../roadmap); deeper design context
remains in the [design document](../fontmin-rs-design). Maintainers can use the
[release preparation checklist](../releasing) for the candidate gate.
