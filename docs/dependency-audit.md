# Dependency and artifact audit

The release policy keeps every duplicated Rust dependency and local crate
override explicit. CI also records size budgets for the three executable
delivery surfaces. The machine-readable source of truth is
[`audits/release-policy.json`](../audits/release-policy.json).

## Duplicate dependency decisions

The audit recorded on 2026-07-28 found five duplicate groups:

| Dependency            | Versions        | Decision | Replacement condition                                                         |
| --------------------- | --------------- | -------- | ----------------------------------------------------------------------------- |
| `brotli`              | 7.0.0 / 8.0.4   | Retain   | Remove v7 with the patched WOFF2 decoder chain.                               |
| `brotli-decompressor` | 4.0.3 / 5.0.3   | Retain   | Remove v4 with Brotli v7.                                                     |
| `thiserror`           | 1.0.69 / 2.0.18 | Retain   | Remove v1 with the two WOFF2 compatibility crates.                            |
| `thiserror-impl`      | 1.0.69 / 2.0.18 | Retain   | Follows the reviewed `thiserror` versions.                                    |
| `unicode-width`       | 0.1.14 / 0.2.2  | Retain   | Wait for the `miette`/`textwrap` chain to unify without changing diagnostics. |

The owner for every decision is the fontmin-rs maintainers. The dependency
gate fails if a new duplicate appears, a recorded version changes, or a
duplicate disappears without its retained decision being removed.

## Vendored patch decisions

| Crate                 | Upstream                                                              | Decision and exit                                                                                                                                 |
| --------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `allsorts` 0.17.0     | [yeslogic/allsorts](https://github.com/yeslogic/allsorts)             | Retain the CFF INDEX and `endchar` corrections until an upstream release contains equivalent behavior and the permanent regression corpus passes. |
| `safer-bytes` 0.2.0   | [danieleades/safer-bytes](https://github.com/danieleades/safer-bytes) | Retain the stable-Rust compatibility copy until the selected WOFF2 decoder no longer requires it.                                                 |
| `woff2-patched` 0.4.0 | [zimond/woff2-rs](https://github.com/zimond/woff2-rs)                 | Retain explicit coordinate wrapping until an upstream release or owned decoder passes every WOFF2 regression.                                     |

Each override has patch notes beside its source. The audit verifies that the
root Cargo patch, notes, owner, upstream, decision, and removal condition remain
present together.

## Release artifact budgets

Release builds use thin LTO, one codegen unit, and stripped symbols. The
budgets are intentionally portable across the supported CI platforms:

| Artifact            | Budget |
| ------------------- | -----: |
| Rust CLI            |  8 MiB |
| Native Node binding |  8 MiB |
| Browser WASM binary |  4 MiB |

On macOS arm64 after the release-profile change, the local measurements were
4,425,520 bytes for the CLI, 3,279,968 bytes for the native binding, and
2,964,395 bytes for WASM. CI writes its platform measurements to
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
