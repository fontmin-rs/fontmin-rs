# Font fixtures

The checked-in fonts are a small, reproducible correctness corpus shared by
Rust, Node.js, WASM, CLI, package-smoke, and documentation tests. The
machine-readable inventory is [`manifest.json`](./manifest.json); every binary
also has a companion SHA-256 file.

| Fixture | Shape | Primary coverage | Upstream license |
| --- | --- | --- | --- |
| `ttf/noto-sans-sc-compact.ttf` | derived static TrueType `glyf` | Compact Simplified Chinese and Latin subsetting | [OFL-1.1](https://github.com/notofonts/noto-cjk/blob/f8d157532fbfaeda587e826d4cd5b21a49186f7c/Sans/LICENSE) |
| `ttf/roboto-regular.ttf` | static TrueType `glyf` | Latin subsetting and web-font conversion | [Apache-2.0](https://github.com/googlefonts/roboto/blob/main/LICENSE) |
| `otf/font-awesome-free-solid-900.otf` | static OpenType CFF | Icon-font conversion and private-use coverage | [OFL-1.1](https://github.com/FortAwesome/Font-Awesome/blob/14c65a3747d0f3b751f15831fc719236aea8729d/LICENSE.txt) |
| `otf/source-sans-3-regular.otf` | static OpenType CFF | CFF-to-TTF conversion | [OFL-1.1](https://github.com/adobe-fonts/source-sans/blob/release/LICENSE.md) |
| `otf/source-serif-4-variable-roman.otf` | variable OpenType CFF2 | variation instancing and CFF2-to-TTF conversion | [OFL-1.1](https://github.com/adobe-fonts/source-serif/blob/release/LICENSE.md) |

These binaries remain governed by their upstream licenses; the repository's
MIT license applies to fontmin-rs source code, not to third-party font files.
The exact download URLs and checksums are recorded in the manifest.
Derived fixtures additionally record the upstream digest and every deterministic
fontmin-rs command used to produce the checked-in binary.

Run `pnpm run fixtures:check` after changing this directory. It verifies the
inventory, file signatures, manifest digests, and companion checksum files.
Derived WOFF, WOFF2, EOT, SVG, and subset outputs should normally be generated
inside tests instead of checked in.

Malformed parser inputs live in [`../malformed`](../malformed) so they can be
shared by regression tests and fuzz targets without being mistaken for valid
font fixtures.
