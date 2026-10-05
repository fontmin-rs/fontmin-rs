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
across runtimes. Each variable-font slice must also remain a non-empty subset
and retain its `fvar` and `gvar` tables. The cache key is the production
manifest digest; downloaded bytes are still checked against their recorded
length and SHA-256 before use.

Run the complete production conformance path locally with:

```sh
pnpm run fixtures:production:conformance
```

## Production shaping and rendering

`pnpm run fixtures:production:rendering` also checks real CID CFF, variable
CFF2, and COLRv1 fonts with `hb-shape` and `hb-view` (install HarfBuzz first).
It compares native and WASM subsets with their source font on the same host,
mapping subset GIDs back before comparing glyphs, clusters, advances, and
offsets. Five samples cover Chinese text, Latin kerning and ligatures, three
CFF2 variation positions, and color paint layers. Pixel comparisons reject
blank output and monochrome color-font fallback.

CI installs these tools and uploads the report and source/subset PNGs from
`benchmarks/rendering-current/`, including failed runs. Tool versions and
dropped contextual subtables are recorded. This verifies the declared samples;
it does not promise complete contextual shaping or FeatureVariations support.

## Production latency and memory budgets

`pnpm run bench:production` runs conformance first, builds the release CLI,
then executes each production stage in a fresh process. Three trials are
collected per stage. The median latency avoids treating one scheduler
interruption as a regression, while the largest process `maxRSS` is used for
the memory gate. Isolating stages makes a failure name the responsible runtime,
operation, and fixture.

The committed
[`benchmarks/production-budgets.json`](../benchmarks/production-budgets.json)
defines the Ubuntu 24.04 and Node.js 24 gate:

| Stage family                           | Maximum median latency | Maximum peak RSS |
| -------------------------------------- | ---------------------: | ---------------: |
| Native inspect                         |                 500 ms |          128 MiB |
| WASM initialization                    |                 250 ms |          128 MiB |
| WASM inspect                           |                 250 ms |          160 MiB |
| Native mixed delivery                  |                 500 ms |          192 MiB |
| Native asynchronous WOFF2              |              15,000 ms |          288 MiB |
| Native four-file subset                |              20,000 ms |          512 MiB |
| Native automatic delivery              |              30,000 ms |          384 MiB |
| Native cache, 512 entries/8 concurrent |              20,000 ms |          160 MiB |
| Rust CLI build, 10,000 one-byte inputs |              90,000 ms |          256 MiB |
| Rust CLI font batch, cold/1 worker     |              15,000 ms |          384 MiB |
| Rust CLI font batch, cold/4 workers    |              15,000 ms |          512 MiB |
| Rust CLI font batch, warm/1 worker     |               5,000 ms |          256 MiB |
| Rust CLI font batch, warm/4 workers    |               5,000 ms |          384 MiB |
| WASM mixed delivery                    |               1,000 ms |          256 MiB |

The 10,000-input stage uses four CLI worker slots and measures the child
process directly. Its latency allowance is 90 seconds to accommodate filesystem
overhead across platforms, while retaining the three-trial median and 256 MiB
memory limit. A Rust unit test separately asserts that the scheduler never
has more active operations than its configured limit and preserves input
order. Together these checks catch both unbounded task creation and aggregate
memory regressions without placing a large font corpus in the repository.

Build output uses an independent limit of four concurrent atomic writes, while
`threads` controls per-file processing. Destination directories are prepared
once per parent. Writes still sync each temporary file before replacement,
check symlinks, and reject duplicate destinations, including aliases through
symlinked parents. If a write fails, its active batch finishes before the build
returns the error so temporary-file cleanup can complete.
Potential case or Unicode filename aliases, and names resembling temporary
files, fall back to ordered writes within their batch. This preserves overwrite
order and prevents collisions with another output's temporary file.

The scale stage validates every filename and byte after timing, and records a
SHA-256 digest of filenames and contents for comparison across trials. The
Node cache stage exercises 512 writes with eight concurrent requests, including
the default 256-entry eviction limit. Requests in the same process queue by
cache directory before acquiring the cross-process file lock, avoiding local
25 ms lock-retry waits. Different directories can still make progress together.

The font-batch stages separately exercise real subset and WOFF2 work: eight
inputs use the pinned 17,773,132-byte Noto Sans SC variable font, with one or
four CLI worker slots. Each input selects the same mixed Latin, Chinese, and
punctuation text and produces one WOFF2 file. Copies have distinct paths and
cache keys; temporary hard links avoid duplicating the fixture on disk.

Every trial starts with a fresh application cache. Cold measurements include
subsetting, compression, cache population, and output writes. Warm trials first
complete an unmeasured build, validate it, and delete its output directory;
then a new measured CLI process must restore every output without rewriting
the cache index. Warm-up memory is excluded from the measured child RSS.
Cold and warm refer to the application cache, not the operating system page
cache. Fixture preparation and output validation are outside the timed region.

Validation requires the exact output filenames, complete WOFF2 headers and
lengths, successful CLI inspection, complete coverage of the requested text,
a reduced non-empty glyph set, and retained `fvar`/`gvar` tables.
A SHA-256 digest of filenames and output contents must
match across the three trials and between warm-up and restored outputs. These
checks prevent missing, corrupt, or stale output from appearing as a speedup.

The new font-batch limits are conservative initial ceilings and have not yet
been calibrated on the pinned Ubuntu runner. Local macOS runs use sampled
child RSS; the Linux gate samples the kernel's process high-water mark.
Local results validate the scenarios but do not establish Ubuntu baselines.

CI always uploads `benchmarks/production-current.json`, including when a budget
fails. Each stage records its three latency and memory trials, aggregated
metrics, budget, output byte count, status, and violations. Font-batch stages
also record input count, worker count, cache state, and the output digest.
Absolute budgets are release-blocking on the pinned runner; local reports
remain diagnostic when the host differs.

The committed [`benchmarks/baseline.json`](../benchmarks/baseline.json) records
the machine fingerprint, fixture checksum, three individual means, median
metrics, and parity result. Re-record it only with:

```sh
pnpm run bench:baseline
```

Review the full diff before committing a new baseline. A slower result must be
confirmed with three additional like-for-like runs and either fixed or
documented as an intentional correctness tradeoff.

## Cache and output optimization (2026-10-05)

The [before/after report](../benchmarks/cache-output-2026-10-05.json) records
three release-profile trials on Apple M1 Pro, macOS arm64, Node.js 24.21.0,
and the shell's Homebrew Rust 1.99.0. Full repository checks use pinned Rust
1.98.0; the benchmark records the actual compiler for reproducibility.
Both builds include the same pre-existing workspace changes. Latency is the
median; memory is the largest observed process RSS across the three trials.

| Scenario                                     |   Before |    After | Speedup | Peak RSS before → after |
| -------------------------------------------- | -------: | -------: | ------: | ----------------------: |
| Node cache, 512 writes/8 concurrent requests | 12.464 s |  1.099 s |  11.34× |       93.89 → 91.38 MiB |
| CLI, 10,000 one-byte inputs                  | 54.732 s | 31.769 s |   1.72× |       12.84 → 15.94 MiB |

All CLI scale and real-font batch output digests match before and after,
including cold and warm cache runs with one and four processing workers.
The cache stage compares index byte counts because timestamps vary between
runs; lock and cache correctness are covered by separate regression tests.
These local observations do not change the pinned Ubuntu budgets. Repeat the
scenarios through `production-performance-worker.mjs` using the stage names
recorded in the report after rebuilding the release CLI, binding, and package.

## Current candidate baseline

The committed 1.0.2-rc.1 release-profile baseline records the representative
fontmin-rs pipeline at 0.1829 times the classic Fontmin mean, or roughly 5.47
times faster. `subsetTtf text` measures 1.5135 ms versus 1.0912 ms in the
historical beta.3 snapshot. A second three-trial report reproduced the change.
Targeted measurement attributes it to the new subset engine and the corrected
`keepLayout: "conservative"` remapping; the historical path silently discarded
layout tables.

Absolute timings for subset, WOFF, WOFF2, SVG, and the modern-web pipeline stay
in the report for diagnosis. Hosted-runner absolute timings are evidence, not a
hard gate, because CPU allocation can change between jobs.

For a coarse CPU profile of the representative pipeline, run
`pnpm run bench:profile`. It executes 2,500 release-binding iterations and
writes an ignored `.cpuprofile` under `benchmarks/`. The beta.3 profile confirms
that glyph subsetting is the largest named block; JavaScript pipeline
orchestration is not a material hotspot. The 1.0.2-rc.1 baseline explicitly
accepts the measured cost of retaining and remapping supported layout data.
