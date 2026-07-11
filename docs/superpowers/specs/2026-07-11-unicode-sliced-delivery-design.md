# Unicode-Sliced Font Delivery Design

## Goal

Turn the existing `unicode-range` descriptor into a real delivery
optimization: one input font can produce named subset files, and generated
CSS routes matching code points to the correct file.

## Scope

The feature is explicit and cross-runtime. A slice has a stable name and one
or more Unicode ranges. The same model is available to Rust configuration,
the CLI, N-API, Node, WASM, and the browser Playground.

```json
{
  "delivery": {
    "slices": [
      { "name": "latin", "unicodeRanges": ["U+0000-00FF"] },
      { "name": "cjk", "unicodeRanges": ["U+4E00-9FFF"] }
    ]
  }
}
```

Given `roboto.ttf`, the example produces `roboto-latin.woff2` and
`roboto-cjk.woff2`. CSS contains one `@font-face` block for each output with
the matching canonical `unicode-range` value.

## Domain model

`UnicodeRange` moves from `fontmin_css` to `fontmin_core`. It remains an
inclusive scalar-value range, accepts only `U+HEX` and `U+HEX-HEX`, rejects
wildcards, invalid scalar values, inverted endpoints, and control text, and
serializes as the camel-case string field `unicodeRanges`.

`fontmin_core` adds:

```rust
pub struct FontDeliverySlice {
    pub name: String,
    pub unicode_ranges: Vec<UnicodeRange>,
}
```

Names contain ASCII letters, digits, `_`, and `-`; they cannot be empty,
duplicate, start with `.`, or contain a path separator. Each slice has at
least one range. This keeps output paths deterministic and prevents path
injection.

`SubsetOptions` gains `unicode_ranges: Vec<UnicodeRange>`. The existing text,
explicit Unicode values, and basic text are unioned with the configured range
code points. Range expansion is capped at 65,536 scalar values per subset
operation. A larger declared range returns a clear configuration error rather
than allocating an unbounded character set.

## Pipeline and CSS flow

`FontminConfig` gains optional `delivery: DeliveryConfig`, where
`DeliveryConfig { slices: Vec<FontDeliverySlice> }`. When absent, the current
single-file pipeline is unchanged.

When slices are present, `SlicePlugin` runs after normal TTF normalization and
before output conversion. It replaces each eligible TTF with one TTF asset per
slice, naming each `<stem>-<slice>.ttf`, subsetting with that slice's ranges,
and recording canonical ranges in asset metadata. Normal WOFF, WOFF2, EOT,
and SVG plugins preserve the renamed stems.

`CssFontSource` gains optional `unicode_ranges`. `CssPlugin` reads the ranges
recorded on each asset and generates a separate `@font-face` block per source
range. A manually supplied `CssOptions::unicode_ranges` remains the fallback
for unsliced sources. Slice ranges always win for sliced assets, so a CSS
block never advertises another slice's code points.

## Public APIs

- Config uses `delivery.slices[].unicodeRanges`.
- `build` accepts repeated `--delivery-slice NAME:RANGE[,RANGE...]` flags.
  Repeated flags with the same name append ranges; CLI values replace config
  slices only when at least one flag is supplied.
- N-API and Node expose `unicodeRanges?: string[]` on subset options and a
  `deliverySlices()` built-in plugin factory.
- WASM accepts the same subset option JSON and exposes the same plugin model
  for `optimizeBrowser()`.
- The Playground offers opt-in Latin and CJK delivery presets plus custom
  comma-separated ranges. It sends one slice per enabled preset and downloads
  all generated files and CSS in the existing ZIP workflow.

## Validation and compatibility

Single-file behavior, existing option names, and existing CSS output remain
unchanged when delivery slices are omitted. Delivery requests reject duplicate
names, invalid names, empty range lists, invalid descriptors, and a CSS-only
output selection before files are generated.

There is no automatic language detection, no wildcard syntax, no inferred
range list, no package publishing, no version bump, and no release workflow
change.

## Verification

- Rust tests cover range parsing ownership, slice-name validation, unioned
  subset characters, expansion limits, path-safe names, output naming, and
  per-source CSS descriptors.
- CLI and config tests cover repeated flags, config parity, replacement
  semantics, and invalid requests.
- N-API, Node, and WASM tests assert matching WOFF2 names and CSS blocks.
- Playground unit/component tests cover preset selection, custom range errors,
  request forwarding, and no-slice compatibility.
- Chromium uploads a fixture, selects at least Latin and CJK slices, verifies
  both WOFF2 downloads and their CSS descriptors, and verifies the ZIP name.
- `pnpm run check`, `pnpm run package:smoke`, and the docs browser test pass.
