# Compatibility evidence

`scripts/compatibility-report.mjs` runs three standalone consumer projects:

- packed CLI and Node.js native/WASM workflows;
- browser loading of generated CSS, WOFF, and WOFF2;
- direct browser use of the packed `@fontmin-rs/wasm` package.

`pnpm run compatibility:check` builds candidate packages, installs them into
temporary projects, and writes `compatibility/current.json`. CI uploads that
ignored file as the `compatibility-report` artifact.

An already published release can be checked without using workspace artifacts:

```sh
node scripts/compatibility-report.mjs \
  --registry-version 1.0.0 \
  --output compatibility/1.0.0.json
```

Versioned registry reports may be committed as release evidence after their
environment and results have been reviewed.

The reviewed `1.0.0`, `1.0.0-rc.1`, and `1.0.2-rc.1` reports passed all three
standalone consumers. The `1.0.2-rc.1` report verifies the exact registry
packages on Node.js 24 and Chromium; its source version and environment are
recorded in `compatibility/1.0.2-rc.1.json`. The `1.0.0` promotion decision is
recorded in `audits/1.0.0-readiness.json`.
