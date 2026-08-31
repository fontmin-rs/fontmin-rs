# Dependency and artifact audit

The release policy keeps every duplicated Rust dependency and local crate
override explicit. CI also records size budgets for the three executable
delivery surfaces. The machine-readable source of truth is
[`audits/release-policy.json`](../audits/release-policy.json).

## Duplicate dependency decisions

The 2026-08-30 dependency upgrade removed the duplicated Brotli and thiserror
major versions. Four reviewed groups remain:

| Dependency      | Versions        | Decision | Replacement condition                                                         |
| --------------- | --------------- | -------- | ----------------------------------------------------------------------------- |
| `hashbrown`     | 0.15.5 / 0.17.1 | Retain   | Remove v0.15 when the `wasmi`/`string-interner` and `indexmap` chains align.  |
| `miniz_oxide`   | 0.8.9 / 0.9.1   | Retain   | Wait for the `backtrace` and `flate2` chains to align.                        |
| `syn`           | 2.0.119 / 3.0.4 | Retain   | Wait for the remaining procedural-macro crates to adopt syn 3.                |
| `unicode-width` | 0.1.14 / 0.2.2  | Retain   | Wait for the `miette`/`textwrap` chain to unify without changing diagnostics. |

The owner for every decision is the fontmin-rs maintainers. The dependency
gate fails if a new duplicate appears, a recorded version changes, or a
duplicate disappears without its retained decision being removed.

## Vendored patch decisions

| Crate                  | Upstream                                                              | Decision and exit                                                                                                                                           |
| ---------------------- | --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `allsorts` 0.17.0      | [yeslogic/allsorts](https://github.com/yeslogic/allsorts)             | Retain the CFF INDEX and `endchar` corrections until an upstream release contains equivalent behavior and the permanent regression corpus passes.           |
| `oxifont-subset` 0.2.2 | [cool-japan/oxifont](https://github.com/cool-japan/oxifont)           | Retain the safe selection, CFF/CFF2, CID, COLR v1, and cmap changes until an equivalent upstream release passes the regression corpus and dependency audit. |
| `safer-bytes` 0.2.0    | [danieleades/safer-bytes](https://github.com/danieleades/safer-bytes) | Retain the stable-Rust compatibility copy until the selected WOFF2 decoder no longer requires it.                                                           |
| `woff2-patched` 0.4.0  | [zimond/woff2-rs](https://github.com/zimond/woff2-rs)                 | Retain explicit coordinate wrapping until an upstream release or owned decoder passes every WOFF2 regression.                                               |

Each override has patch notes beside its source. The audit verifies that the
root Cargo patch, notes, owner, upstream, decision, and removal condition remain
present together. The vendored `oxifont-subset` copy starts from published
version 0.2.2 and removes unused production dependencies on
`oxifont-parser` and the unmaintained `ttf-parser`, and carries the source-level
subsetting safety changes listed above. The complete workspace is compiled on
Rust 1.98 in CI. Raising the workspace MSRV satisfied the recorded exit
condition for the metadata-only `oxifont-core` patch, so the workspace now
resolves the published 0.2.2 crate directly. The oxifont-subset upstream
preparation and commit mapping are captured in
`audits/oxifont-subset-upstream-2026-08-31.md`.

## Release artifact budgets

Release builds use thin LTO, one codegen unit, and stripped symbols. The
budgets are intentionally portable across the supported CI platforms:

| Artifact            | Budget |
| ------------------- | -----: |
| Rust CLI            |  8 MiB |
| Native Node binding |  8 MiB |
| Browser WASM binary |  5 MiB |

On macOS arm64 with the current feature set, the local measurements were
7,139,040 bytes for the CLI, 5,678,928 bytes for the native binding, and
4,731,426 bytes for WASM. The WASM budget includes headroom for the browser
runtime's variable-font reduction and source-bound subset-plan support. CI
writes its platform measurements to
`audits/artifact-current.json` and uploads the report even alongside the
performance reports.

Run the policy-only check with:

```shell
pnpm run audit:dependencies
```

Build and measure all release surfaces with:

```shell
pnpm run audit:artifacts
```

Artifact budget failures persist the complete report before exiting, so the
responsible delivery surface and measured bytes remain available for review.
