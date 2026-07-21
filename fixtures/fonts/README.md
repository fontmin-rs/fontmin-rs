# Font fixtures

The checked-in fonts are a small, reproducible correctness corpus shared by
Rust, Node.js, WASM, CLI, package-smoke, and documentation tests. The
machine-readable inventory is [`manifest.json`](./manifest.json); every binary
also has a companion SHA-256 file.

| Fixture | Shape | Primary coverage | Upstream license |
| --- | --- | --- | --- |
| `ttf/roboto-regular.ttf` | static TrueType `glyf` | Latin subsetting and web-font conversion | [Apache-2.0](https://github.com/googlefonts/roboto/blob/main/LICENSE) |
| `otf/source-sans-3-regular.otf` | static OpenType CFF | CFF-to-TTF conversion | [OFL-1.1](https://github.com/adobe-fonts/source-sans/blob/release/LICENSE.md) |
| `otf/source-serif-4-variable-roman.otf` | variable OpenType CFF2 | variation instancing and CFF2-to-TTF conversion | [OFL-1.1](https://github.com/adobe-fonts/source-serif/blob/release/LICENSE.md) |

These binaries remain governed by their upstream licenses; the repository's
MIT license applies to fontmin-rs source code, not to third-party font files.
The exact download URLs and checksums are recorded in the manifest.

Run `pnpm run fixtures:check` after changing this directory. It verifies the
inventory, file signatures, manifest digests, and companion checksum files.
Derived WOFF, WOFF2, EOT, SVG, and subset outputs should normally be generated
inside tests instead of checked in.

The current binary corpus deliberately stays compact. Before 1.0 it still
needs a redistributable CJK subset, a representative icon font, and malformed
inputs for parser/fuzz regression tests; these gaps are tracked in the
[roadmap](../../docs/roadmap.md).
