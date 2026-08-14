# Dependency and artifact audit

The release policy keeps every duplicated Rust dependency and local crate
override explicit. CI also records size budgets for the three executable
delivery surfaces. The machine-readable source of truth is
[`audits/release-policy.json`](../audits/release-policy.json).

## Duplicate dependency decisions

The original 2026-07-28 audit found five duplicate groups. The variable-font
reduction runtime added one reviewed group on 2026-08-14:

| Dependency            | Versions        | Decision | Replacement condition                                                         |
| --------------------- | --------------- | -------- | ----------------------------------------------------------------------------- |
| `brotli`              | 7.0.0 / 8.0.4   | Retain   | Remove v7 with the patched WOFF2 decoder chain.                               |
| `brotli-decompressor` | 4.0.3 / 5.0.3   | Retain   | Remove v4 with Brotli v7.                                                     |
| `hashbrown`           | 0.15.5 / 0.17.1 | Retain   | Remove v0.15 when the `wasmi`/`string-interner` and `indexmap` chains align.  |
| `thiserror`           | 1.0.69 / 2.0.18 | Retain   | Remove v1 with the two WOFF2 compatibility crates.                            |
| `thiserror-impl`      | 1.0.69 / 2.0.18 | Retain   | Follows the reviewed `thiserror` versions.                                    |
| `unicode-width`       | 0.1.14 / 0.2.2  | Retain   | Wait for the `miette`/`textwrap` chain to unify without changing diagnostics. |

The owner for every decision is the fontmin-rs maintainers. The dependency
gate fails if a new duplicate appears, a recorded version changes, or a
duplicate disappears without its retained decision being removed.

## Vendored patch decisions

| Crate                  | Upstream                                                              | Decision and exit                                                                                                                                          |
| ---------------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `allsorts` 0.17.0      | [yeslogic/allsorts](https://github.com/yeslogic/allsorts)             | Retain the CFF INDEX and `endchar` corrections until an upstream release contains equivalent behavior and the permanent regression corpus passes.          |
| `oxifont-core` 0.2.2   | [cool-japan/oxifont](https://github.com/cool-japan/oxifont)           | Retain the MSRV metadata-only patch until upstream declares Rust 1.88 support, or this project raises its MSRV in a SemVer-minor release.                  |
| `oxifont-subset` 0.2.2 | [cool-japan/oxifont](https://github.com/cool-japan/oxifont)           | Retain the manifest patch until upstream supports Rust 1.88 and removes its unused production parser dependencies, or the audited exit conditions are met. |
| `safer-bytes` 0.2.0    | [danieleades/safer-bytes](https://github.com/danieleades/safer-bytes) | Retain the stable-Rust compatibility copy until the selected WOFF2 decoder no longer requires it.                                                          |
| `woff2-patched` 0.4.0  | [zimond/woff2-rs](https://github.com/zimond/woff2-rs)                 | Retain explicit coordinate wrapping until an upstream release or owned decoder passes every WOFF2 regression.                                              |

Each override has patch notes beside its source. The audit verifies that the
root Cargo patch, notes, owner, upstream, decision, and removal condition remain
present together. The two oxifont copies retain the published 0.2.2 Rust
sources. `oxifont-core` only lowers its manifest-declared Rust version from 1.89
to 1.88. `oxifont-subset` also removes unused production dependencies on
`oxifont-parser` and the unmaintained `ttf-parser`; neither is referenced by its
`src/` tree. The complete workspace is compiled on Rust 1.88 in CI.

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
