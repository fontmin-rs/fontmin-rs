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

The same CI job prepares the commit-pinned production corpus under
`fixtures/production/.cache`. It verifies a 31,036-glyph Noto Sans SC variable
font and Noto Color Emoji through both native and WASM inspection, then requires
the mixed Latin, CJK, and punctuation delivery slices to be byte-identical
across runtimes. The cache key is the production manifest digest; downloaded
bytes are still checked against their recorded length and SHA-256 before use.

Run the complete production conformance path locally with:

```sh
pnpm run fixtures:production:conformance
```

## Production latency and memory budgets

`pnpm run bench:production` runs conformance first, then executes each
production stage in a fresh Node.js process. Three trials are collected per
stage. The median latency avoids treating one scheduler interruption as a
regression, while the largest process `maxRSS` is used for the memory gate.
Isolating stages makes a failure name the responsible runtime, operation, and
fixture.

The committed
[`benchmarks/production-budgets.json`](../benchmarks/production-budgets.json)
defines the Ubuntu 24.04 and Node.js 24 gate:

| Stage family          | Maximum median latency | Maximum peak RSS |
| --------------------- | ---------------------: | ---------------: |
| Native inspect        |                 500 ms |          128 MiB |
| WASM initialization   |                 250 ms |          128 MiB |
| WASM inspect          |                 250 ms |          160 MiB |
| Native mixed delivery |                 500 ms |          192 MiB |
| WASM mixed delivery   |               1,000 ms |          256 MiB |

CI always uploads `benchmarks/production-current.json`, including when a budget
fails. Each stage records its three latency and memory trials, aggregated
metrics, budget, output byte count, status, and violations. Absolute budgets
are release-blocking on the pinned runner; local reports remain diagnostic when
the host differs.

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
0.1485 times the classic Fontmin mean, or roughly 6.73 times faster. The earlier
debug-profile snapshot was not a product regression; replacing it with a
release-profile gate resolves that measurement error.

Absolute timings for subset, WOFF, WOFF2, SVG, and the modern-web pipeline stay
in the report for diagnosis. Hosted-runner absolute timings are evidence, not a
hard gate, because CPU allocation can change between jobs.

For a coarse CPU profile of the representative pipeline, run
`pnpm run bench:profile`. It executes 2,500 release-binding iterations and
writes an ignored `.cpuprofile` under `benchmarks/`. The beta.3 profile confirms
that glyph subsetting is the largest named block; JavaScript pipeline
orchestration is not a material hotspot. Because the paired gate is already
well above parity, beta.3 records no intentional correctness-for-performance
tradeoff.
