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
artifact so regressions can be investigated with like-for-like runs. Before
1.0, the release benchmark should use a fixed runner and release-profile native
binding, then enforce a sustained regression budget against the release
candidate baseline.

Use `pnpm run bench:profile` for a coarse Node CPU profile of 2,500
release-binding `glyph + ttf2woff` iterations. It writes an ignored
`.cpuprofile` into this directory for inspection in Chrome DevTools. Override
the bounded iteration count with `FONTMIN_PROFILE_ITERATIONS`.

The beta.3 release-profile snapshot passes the compatibility gate at 0.1485
times the classic Fontmin mean, or roughly 6.73 times faster, on the recorded
Apple M1 Pro runner. No correctness tradeoff was required.
