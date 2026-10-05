# Production fixture corpus

This corpus covers inputs that are too large to keep in Git history but are
required for real-world conformance and performance checks.
[`manifest.json`](./manifest.json) pins each upstream source to an immutable
commit and records its byte length, SHA-256 digest, Git blob identity, license,
expected metadata, and exercised scenarios.

Run:

```sh
pnpm run fixtures:production
```

The command downloads byte-identical files through commit-pinned CDN URLs,
verifies their length and digest, and stores them under the ignored
`fixtures/production/.cache/` directory. A valid cache entry is reused; a
truncated or modified entry is replaced atomically.

| Fixture | Production evidence |
| --- | --- |
| Noto Color Emoji | 10.7 MB bitmap color font with `CBDT` and `CBLC` tables |
| Noto COLRv1 | 5.0 MB color paint graphs and referenced outlines |
| Noto Sans SC CID | 8.3 MB CID-keyed CFF with CJK text and Latin kerning |
| Noto Sans SC VF | 17.8 MB, 31,036-glyph Simplified Chinese variable TrueType font |
| Source Serif 4 CFF2 | 1.9 MB variable CFF2; ligatures, kerning, weight and optical size |

The regular checked-in corpus under [`../fonts`](../fonts) remains the default
for fast correctness tests. Production fixtures are prepared only by the
dedicated conformance and performance jobs.

Run `pnpm run bench:production` to execute the complete native/WASM conformance
check followed by the isolated latency and peak-RSS budgets declared in
`benchmarks/production-budgets.json`.

Run `pnpm run fixtures:production:rendering` for the independent HarfBuzz
shaping and Cairo rendering gate. It requires `hb-shape` and `hb-view` on
`PATH` (Ubuntu: `sudo apt-get install libharfbuzz-bin`; macOS:
`brew install harfbuzz`). CI runs it on the Ubuntu 24.04 benchmark job and
records the installed tool versions. It compares source and subset on the
same host; no cross-platform pixel baseline is assumed.

Every `shape-render` fixture declares its text, language and variation
coordinates in the manifest. The gate tests both native and WASM subsets,
maps new GIDs back to source GIDs, and compares glyph sequence, clusters,
advances and offsets. Pixel comparisons cover CID outlines, CFF2 at default
and two non-default axis positions, and COLRv1 paint layers. Blank output and
monochrome color-font fallback fail explicitly. Reports and source/subset
PNGs are retained under ignored `benchmarks/rendering-current/`, including
on failure, and uploaded with the CI benchmark artifacts.

These samples exercise the default `conservative` layout policy and record
dropped contextual subtables. They do not establish complete contextual
shaping or `FeatureVariations` support for every string; the strict
`preserve` policy continues to reject layouts it cannot fully retain.
