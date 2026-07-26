# Performance policy

Performance evidence is collected from optimized native bindings. Debug
bindings are useful for development and correctness checks, but they are not a
release-performance signal.

## Release gate

`pnpm run bench:report` builds the native binding with Cargo's release profile,
runs every Vitest benchmark three times, and writes the median report to
`benchmarks/current.json`. The CI benchmark job pins Ubuntu 24.04, Node.js 24,
and the repository Rust toolchain so that reports remain like-for-like at the
software boundary.

The representative compatibility case runs the same Roboto input and
`glyph + ttf2woff` request through fontmin-rs and classic Fontmin in each trial.
Its paired mean-time ratio is release-blocking when fontmin-rs exceeds 1.10.
Comparing both implementations in the same process makes this gate less
sensitive to hosted-runner hardware variation than an absolute millisecond
threshold.

The committed [`benchmarks/baseline.json`](../benchmarks/baseline.json) records
the machine fingerprint, fixture checksum, three individual means, median
metrics, and parity result. Re-record it only with:

```sh
pnpm run bench:baseline
```

Review the full diff before committing a new baseline. A slower result must be
confirmed with three additional like-for-like runs and either fixed or
documented as an intentional correctness tradeoff.

## Current result

The release-profile baseline records the representative fontmin-rs pipeline at
0.1482 times the classic Fontmin mean, or roughly 6.75 times faster. The earlier
debug-profile snapshot was not a product regression; replacing it with a
release-profile gate resolves that measurement error.

Absolute timings for subset, WOFF, WOFF2, SVG, and the modern-web pipeline stay
in the report for diagnosis. Hosted-runner absolute timings are evidence, not a
hard gate, because CPU allocation can change between jobs.
