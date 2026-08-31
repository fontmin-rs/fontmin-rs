# oxifont-subset upstream preparation — 2026-08-31

## Baseline

- Upstream: <https://github.com/cool-japan/oxifont>
- Release: `v0.2.2`
- Base commit: `8dca5507d127404061f54f1341ac061ff88562df`
- Prepared branch: `fontmin/upstream-full-series` in a clean upstream checkout

The upstream `master` branch and latest release pointed to the same commit when
this audit was performed. The source series was applied without carrying the
fontmin-rs-only manifest changes.

The generated mail patches are checked in under
[`audits/patches/oxifont-subset-0.2.2`](patches/oxifont-subset-0.2.2). Apply
them to the recorded upstream base with:

```shell
git am /path/to/fontmin-rs/audits/patches/oxifont-subset-0.2.2/*.patch
```

## Commit series

| Upstream commit | fontmin-rs source        | Subject                                                       |
| --------------- | ------------------------ | ------------------------------------------------------------- |
| `c32bd9e`       | `2760308`                | `feat(subset): add professional selection and table controls` |
| `b19684a`       | `3e406fe`                | `fix(subset): rewrite CFF2 indexes and offsets safely`        |
| `3521d8d`       | `4c24091`                | `fix(subset): reject unsafe CFF rewrites`                     |
| `d262c1c`       | `1fa2046`                | `feat(subset): support CID CFF and COLR v1 subsetting`        |
| `b01904e`       | `087d451`                | `fix(subset): harden COLR v1 paint rewrites`                  |
| `7cde86f`       | `e8186db`                | `fix(subset): reject malformed format 12 cmap ranges`         |
| `e957bed`       | working-tree refactor    | `refactor(subset): split cmap parsing and CFF tests`          |
| `766f51f`       | working-tree test update | `test(subset): use a valid minimal CFF fixture`               |

The final two commits satisfy the upstream contribution rule that source files
remain below 2,000 lines and update a stale fixture that expected malformed CFF
data to be copied verbatim. Final source sizes are 1,829 lines for `lib.rs`,
1,998 for `cff.rs`, 690 for `cmap.rs`, and 149 for `cff/tests.rs`.

## Validation

Run from the clean upstream checkout:

```shell
cargo fmt --all --check
cargo test --workspace --lib --tests
cargo clippy -p oxifont-subset --all-targets -- -D warnings
cd crates/oxifont-subset
cargo +nightly fuzz build fuzz_subset
cargo +nightly fuzz build fuzz_subset_by_gids
```

Results: formatting passed; 1,119 library/integration tests passed with 23
ignored; the affected crate passed Clippy with warnings denied; and both fuzz
targets built successfully with the installed nightly toolchain. The checked-in
eight-patch series was also applied with `git am` to a second clean checkout and
produced a tree identical to the prepared branch.

The upstream guide asks for `cargo nextest run --workspace`; `cargo-nextest` was
not installed in this environment, so `cargo test --workspace --lib --tests`
was used to cover every library and integration-test target. A plain
`cargo test --workspace` additionally runs doctests and reaches an unrelated
existing failure in `crates/oxifont/src/lib.rs`: its example imports
`oxifont::db` without enabling the `db` feature.

The requested full-workspace Clippy command was also attempted. Rust 1.98
reports three pre-existing `chunks_exact_to_as_chunks` errors in
`oxifont-hinting` and `oxifont-webfont`; those files are unchanged by this patch
series. The affected `oxifont-subset` crate passes the same `-D warnings` gate.

## Adoption gate

Do not remove the local override merely because these commits are submitted.
Wait for an upstream release containing equivalent selection, CFF/CFF2, CID,
COLR v1, and cmap behavior; run the fontmin-rs malformed-font and color-font
regression corpus against that release; then audit any remaining manifest and
dependency-graph differences.
