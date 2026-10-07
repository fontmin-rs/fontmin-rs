# Font fixtures

The checked-in fonts are a small, reproducible correctness corpus shared by
Rust, Node.js, WASM, CLI, package-smoke, and documentation tests. The
machine-readable inventory is [`manifest.json`](./manifest.json); every binary
also has a companion SHA-256 file.

| Fixture | Shape | Primary coverage | Upstream license |
| --- | --- | --- | --- |
| `ttf/estedad-variable.ttf` | two-axis variable TrueType `glyf` | Partial axis pinning and range reduction | [OFL-1.1](https://github.com/aminabedi68/Estedad/blob/0dbe689787b8c2ea302373cb601d0f352f9f98e5/OFL.txt) |
| `ttf/noto-sans-sc-compact.ttf` | derived static TrueType `glyf` | Compact Simplified Chinese and Latin subsetting | [OFL-1.1](https://github.com/notofonts/noto-cjk/blob/f8d157532fbfaeda587e826d4cd5b21a49186f7c/Sans/LICENSE) |
| `ttf/noto-sans-sc-variable-compact.ttf` | derived variable TrueType `glyf` | Compact static-instancing coverage | [OFL-1.1](https://github.com/notofonts/noto-cjk/blob/f8d157532fbfaeda587e826d4cd5b21a49186f7c/Sans/LICENSE) |
| `ttf/recursive-variable.ttf` | five-axis variable TrueType `glyf` | GSUB FeatureVariations and static GPOS/GDEF instancing | [OFL-1.1](./ttf/recursive-OFL.txt) |
| `ttf/roboto-regular.ttf` | static TrueType `glyf` | Latin subsetting and web-font conversion | [Apache-2.0](https://github.com/googlefonts/roboto/blob/main/LICENSE) |
| `otf/font-awesome-free-solid-900.otf` | static OpenType CFF | Icon-font conversion and private-use coverage | [OFL-1.1](https://github.com/FortAwesome/Font-Awesome/blob/14c65a3747d0f3b751f15831fc719236aea8729d/LICENSE.txt) |
| `otf/source-sans-3-regular.otf` | static OpenType CFF | CFF-to-TTF conversion | [OFL-1.1](https://github.com/adobe-fonts/source-sans/blob/release/LICENSE.md) |
| `otf/source-serif-4-variable-roman.otf` | variable OpenType CFF2 | variation instancing and CFF2-to-TTF conversion | [OFL-1.1](https://github.com/adobe-fonts/source-serif/blob/release/LICENSE.md) |

These binaries remain governed by their upstream licenses; the repository's
MIT license applies to fontmin-rs source code, not to third-party font files.
The exact commit-pinned download URLs and checksums are recorded in the
manifest.
Derived fixtures additionally record the upstream digest and every deterministic
fontmin-rs command used to produce the checked-in binary.

Run `pnpm run fixtures:check` after changing this directory or
`fixtures/malformed`. It verifies both inventories, immutable provenance, file
signatures, manifest digests, companion checksum files, and malformed
diagnostic contracts.
Derived WOFF, WOFF2, EOT, SVG, and subset outputs should normally be generated
inside tests instead of checked in.

Malformed parser inputs live in [`../malformed`](../malformed) so they can be
shared by regression tests and fuzz targets without being mistaken for valid
font fixtures.

Recursive is the unmodified upstream font, including all 11 FeatureVariations
records, axis mappings, and alternate glyphs. Its bundled OFL text has SHA-256
`f9f539cf7549bd417159dbdb9c400943a5b60a7366c2c6fbde9f095173d82479`.

The compact Noto variable fixture was regenerated on 2026-10-07 using the
recorded source and commands with the layout fixes in `6843318d`. The previous
SHA-256 `2fab33cdf8b2a5597089b9c8e434fd3a7e6d891942101539aa20c2fc21fa515f`
contained GPOS data that HarfBuzz could shape but could not fully instance.
The regenerated fixture passes full instancing and matches source advances and
offsets for `AB中文` at weights 100, 500, and 900. The source binary and selected
characters are unchanged.
