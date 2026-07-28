# Public contracts

The stable public boundary is recorded in
[`contracts/public-api.json`](https://github.com/fontmin-rs/fontmin-rs/blob/main/contracts/public-api.json).
CI compares the implementation with that machine-readable inventory. Changing
the inventory is an intentional compatibility decision: update the changelog,
migration guidance, and affected tests in the same pull request.

## CLI

The commands `build`, `subset`, `coverage`, `inspect`, `convert`, `bench`,
`init`, and `doctor` are frozen together with the long flags listed in the
contract. Successful commands exit with `0`; argument, configuration, I/O, and
processing failures exit with `1`. Scripts should additionally branch on a
stable diagnostic code when one is available.

Help presentation, whitespace, terminal colors, and human-readable wording are
not frozen. The command names, accepted flags, exit status, and diagnostic
codes are.

## Configuration

The Rust CLI, Node package, and browser WASM package have deliberately
different boundaries:

- Rust accepts project fields, serializable built-in plugin descriptors, and
  the six documented config-file extensions.
- Node accepts filesystem or in-memory inputs, custom plugin hooks, and
  `runtime: "native" | "wasm" | "auto"`.
- Browser WASM accepts only in-memory `assets` and `plugins`.

The exact top-level fields are in the contract inventory. Option behavior and
nested fields remain documented in [Configuration](./guide/config.md).

## JavaScript exports and plugin lifecycle

The runtime export names for `fontmin-rs`, its `./plugins`, `./presets`, and
`./compat` subpaths, and `@fontmin-rs/wasm` are frozen. Type-only exports follow
the same compatibility policy, with TypeScript compilation tests acting as the
gate.

Node plugins run `buildStart`, `transform`, `generateBundle`, and `buildEnd` in
that order. A breaking hook signature or ordering change follows the
deprecation policy and requires explicit release planning.

## Diagnostics and file names

Stable Rust-originated diagnostics use the `fontmin::*` codes listed in the
inventory, including `fontmin::invalid_font`. Node and WASM expose those codes
through `FontminDiagnosticError`.

Generated files follow these templates:

| Output                         | Template                     |
| ------------------------------ | ---------------------------- |
| Normal transform               | `{stem}.{extension}`         |
| Named Unicode delivery slice   | `{stem}-{slice}.{extension}` |
| Default SVG icon-font stem     | `iconfont`                   |
| Preserved original input asset | `{input-file-name}`          |

Explicit `fileName`, `ext`, or icon-font `fontName` options override the
corresponding default. Tests freeze representative output sets and CSS URLs,
not encoder byte identity.

## Compatibility rule

Additive changes still require an inventory update and a successful full
release gate. Removing or changing an item follows the
[deprecation policy](./deprecation.md). A future `1.0.0` release will run a
separately versioned release-candidate cycle before finalizing its contract.
