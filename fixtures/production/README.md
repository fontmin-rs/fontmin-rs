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
| Noto Sans SC VF | 17.8 MB, 31,036-glyph Simplified Chinese variable TrueType font |

The regular checked-in corpus under [`../fonts`](../fonts) remains the default
for fast correctness tests. Production fixtures are prepared only by the
dedicated conformance and performance jobs.

Run `pnpm run bench:production` to execute the complete native/WASM conformance
check followed by the isolated latency and peak-RSS budgets declared in
`benchmarks/production-budgets.json`.
