# Performance baselines

[`baseline.json`](./baseline.json) is a reviewable snapshot of the Node.js
benchmark suite. It records the exact font fixture, runtime environment, mean,
p75, p99, relative margin of error, and sample count for each operation.

Collect a report with the release-profile native binding:

```sh
pnpm run bench:report
```

`bench:report` writes the ignored `benchmarks/current.json`. Compare that file
with the committed baseline only when both runs use the same OS, architecture,
Node.js major, native-binding profile, fixture checksum, and an otherwise idle
machine. To intentionally accept a new baseline, run `pnpm run bench:baseline`,
review the JSON diff, and explain material changes in the pull request.

The baseline is observational rather than a CI timing gate. Hosted runners are
too noisy for hard millisecond thresholds; CI uploads `current.json` as an
artifact so regressions can be investigated with like-for-like runs. The
committed snapshot was recorded from the exact 1.0.2-rc.1 candidate on the
fixed Apple M1 Pro runner with the release-profile native binding.

Use `pnpm run bench:profile` for a coarse Node CPU profile of 2,500
release-binding `glyph + ttf2woff` iterations. It writes an ignored
`.cpuprofile` into this directory for inspection in Chrome DevTools. Override
the bounded iteration count with `FONTMIN_PROFILE_ITERATIONS`.

The 1.0.2-rc.1 snapshot passes the compatibility gate at 0.1829 times the
classic Fontmin mean, or roughly 5.47 times faster. `subsetTtf text` measures
1.5135 ms versus 1.0912 ms in the historical beta.3 snapshot. The increase was
confirmed by a second three-trial report and isolated to the new subset engine
plus the corrected `keepLayout: "conservative"` remapping; the historical path
silently discarded layout tables.
