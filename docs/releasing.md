# Release preparation

Releases are prepared as a candidate first. The preparation commands do not
create a Git tag, push commits, publish npm packages, or dispatch the Release
workflow.

## Automated gate

Install the tools used by CI, then run the complete release gate:

```shell
rustup component add llvm-tools-preview
cargo install cargo-deny cargo-llvm-cov
pnpm run release:check
```

The gate verifies:

- one non-placeholder version across the Rust workspace and all 11 published
  npm packages;
- a matching changelog heading;
- formatting, linting, typechecking, Rust/Node/WASM tests, and documentation;
- Rust advisory/source policy and high-severity npm advisories;
- a minimum 80% Rust line-coverage baseline (81.94% when established);
- dry-run package contents and installable Node/WASM/native package tarballs.

The advisory policy records two temporary maintenance exceptions:
`RUSTSEC-2024-0436` (`paste`, transitive through `woff2-patched`) and
`RUSTSEC-2026-0192` (`ttf-parser`). Neither advisory reports a vulnerability or
offers a safe upgrade. All vulnerability and unsoundness advisories still fail
the gate; remove each exception when its dependency path is replaced.

To validate a prospective tag without creating it:

```shell
pnpm run release:metadata -- --tag v0.1.0-beta.1
```

## Manual checklist

- [ ] `pnpm install --frozen-lockfile` succeeds from a clean checkout.
- [ ] `pnpm run release:check` passes locally.
- [ ] The main-branch CI matrix passes on Linux, macOS, and Windows.
- [ ] Node 22, 24, and 26 tests pass.
- [ ] Chromium, Firefox, and WebKit WASM tests pass.
- [ ] All eight native binding artifacts are present.
- [ ] `CHANGELOG.md` matches the candidate behavior and known limitations.
- [ ] The candidate version and prospective `v<version>` tag agree.
- [ ] Package tarballs contain no fixture, test, cache, or development files.

Stop after this checklist when preparing a release. Creating the tag and
running the Release workflow are separate, explicitly authorized actions.
