# Roadmap to 1.0

fontmin-rs `0.1.0-beta.3` is published across the CLI, Node.js package,
browser WASM package, native binding, and eight platform packages. The public
surface is broad enough for real projects; the remaining work is about proving
stability, correctness, and operability before declaring the contracts final.

This roadmap uses exit criteria instead of calendar promises. A milestone is
complete only when its checks are repeatable on `main` and in the release
workflow.

## Current baseline

- One release version is validated across all 11 npm packages, Cargo metadata,
  embedded runtime versions, the changelog, and the release tag.
- CI covers formatting, warning-free Rust and TypeScript linting, Node.js
  22/24/26, WASM, browser loading, the documentation playground, native package
  smoke tests, release readiness, and benchmarks.
- The release gate rejects high or critical dependency advisories, requires at
  least 80% Rust line coverage, inspects packed npm contents, and runs consumer
  smoke tests.
- The shared binary fixture inventory and its checksums are now validated by
  `pnpm run fixtures:check`.
- Local development and every GitHub workflow use the same pinned Rust 1.97.1
  toolchain; upgrades are explicit repository changes.
- The [performance policy](./benchmarks.md) builds release-profile bindings on
  a pinned CI software runner, aggregates three trials, and gates the paired
  compatibility pipeline while retaining absolute timings for diagnosis.
- The licensed fixture corpus includes Latin, compact CJK, icon-font, CFF,
  CFF2, variable-font, and malformed inputs with reproducible provenance.
- Native and WASM run one semantic conformance matrix across every built-in
  transform, preset, output metadata contract, and malformed diagnostic.
- A bounded AddressSanitizer cargo-fuzz target runs on relevant changes and a
  weekly schedule; minimized crashes become permanent malformed fixtures.
- Rust 1.88.0 is the separately declared and tested MSRV. The pinned toolchain
  and upgrade cadence are defined in the [support policy](./support.md).
- The release-profile `glyph + ttf2woff` baseline is about 6.73 times faster
  than classic Fontmin on the recorded machine; the former debug-profile
  measurement has been retired.
- The [deprecation policy](./deprecation.md),
  [troubleshooting guide](./troubleshooting.md),
  [security policy](https://github.com/fontmin-rs/fontmin-rs/security/policy), migration guide, and release rollback
  procedure define the maintenance path from prerelease through 1.0.
- Rust advisory checks have no accepted exceptions; current npm audit findings
  are resolved by scoped, lockfile-tested overrides.

## Beta hardening

The next beta should reduce unknowns rather than add a large new API surface.

- Continue growing the permanent malformed corpus from minimized fuzz
  discoveries and real-world failures.
- Publish beta.4 from the same hardened gate without release-time metadata
  repair or platform-package rollback.

Exit criterion: two consecutive beta releases pass the complete release gate
without manual metadata repair or platform-package rollback.

## Release candidate

The release candidate freezes user-facing contracts and changes the focus to
compatibility evidence.

- Freeze the CLI flags and exit codes, configuration schema, Node/WASM exports,
  plugin lifecycle, diagnostic codes, and generated file naming rules.
- Publish a support matrix for Node.js versions, operating systems, CPU/libc
  targets, browser WASM capabilities, and the Rust MSRV for library consumers.
- Compare representative Fontmin pipelines for glyph coverage, parsed output,
  CSS semantics, and file naming; byte-for-byte equality is not required.
- Exercise install, CLI, ESM, browser, native fallback, and forced-WASM paths
  from packed tarballs rather than the workspace.

Exit criterion: the frozen contract and support matrix survive one release
candidate cycle with no unresolved P0/P1 correctness, security, or packaging
issue.

## 1.0 release gate

1.0 is ready when all of the following are true:

- Public API and configuration contracts are documented and covered by
  compatibility tests.
- Every supported font path either produces parseable output with the requested
  coverage or returns a stable, actionable diagnostic; malformed input never
  panics across the public boundary.
- Native packages and the WASM fallback pass the same conformance corpus on all
  advertised targets.
- Rust line coverage remains at least 80%, lint is warning-free, packed-package
  smoke tests pass, and no high or critical dependency advisory is accepted.
- Release-profile performance is at least at parity with classic Fontmin for
  the representative compatibility pipeline and remains inside the agreed
  regression budget for native subset and web-font conversion.
- The release workflow can publish every package, create the GitHub release,
  and verify npm dist-tags from a clean tag without local intervention.

Work not required for 1.0—such as every historical Fontmin plugin, every font
format edge case, or distributed caching—should remain explicitly documented
as post-1.0 scope instead of delaying stable contracts indefinitely.
