# Compatibility Evidence

The `1.0` contract is validated from standalone consumer projects in addition
to unit, integration, conformance, and package-content tests. These projects
install packed artifacts into temporary directories and use only published
entry points.

## Consumer projects

| Project                    | Boundary exercised                                                                   |
| -------------------------- | ------------------------------------------------------------------------------------ |
| Standalone CLI and Node.js | Packed executable, native API, native pipeline, automatic WASM fallback, forced WASM |
| Browser font loading       | Packed Node pipeline, generated CSS/WOFF/WOFF2, `FontFaceSet` loading                |
| Standalone browser WASM    | Packed `@fontmin-rs/wasm`, in-memory inspection and optimization, `FontFace` loading |

Run the workspace-tarball report after installing Chromium:

```sh
pnpm --filter fontmin-rs exec playwright install chromium
pnpm run compatibility:check
```

The report records the package source, exact version, Node.js version,
operating system, architecture, browser, exercised interfaces, and result for
each project. CI uploads `compatibility/current.json` as the
`compatibility-report` artifact, and any failed project blocks release.

## Published release candidates

The same projects can install an exact version from npm:

```sh
node scripts/compatibility-report.mjs \
  --registry-version 1.0.0-rc.1 \
  --output compatibility/1.0.0-rc.1.json
```

This registry mode verifies the package metadata, optional native dependency,
WASM dependency, executable, and browser assets that users actually receive.
A reviewed RC report is committed before stable promotion.

The report freezes semantics, diagnostics, generated names, and browser
loadability. Encoder byte identity is not a compatibility promise.
