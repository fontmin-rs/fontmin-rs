# Performance baselines

[`baseline.json`](./baseline.json) is a reviewable snapshot of the Node.js
benchmark suite. It records the exact font fixture, runtime environment, mean,
p75, p99, relative margin of error, and sample count for each operation.

Build the debug native binding before collecting a report:

```sh
pnpm run build:debug
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

The first beta.2 snapshot shows one immediate optimization target: the
`fontmin-rs glyph + ttf2woff` compatibility pipeline is slower than classic
Fontmin on the recorded debug environment. The 1.0 target is parity or better
on the same runner while preserving output correctness.
