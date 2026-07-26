# Roadmap to 1.0

fontmin-rs `0.1.0-beta.2` is published across the CLI, Node.js package,
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
- A portable benchmark snapshot lives in
  [`benchmarks/baseline.json`](../benchmarks/baseline.json); CI publishes each
  new run as an artifact without using noisy hosted-runner timings as a hard
  gate.
- The licensed fixture corpus includes Latin, compact CJK, icon-font, CFF,
  CFF2, variable-font, and malformed inputs with reproducible provenance.
- Native and WASM run one semantic conformance matrix across every built-in
  transform, preset, output metadata contract, and malformed diagnostic.
- A bounded AddressSanitizer cargo-fuzz target runs on relevant changes and a
  weekly schedule; minimized crashes become permanent malformed fixtures.
- Rust 1.88.0 is the separately declared and tested MSRV. The pinned toolchain
  and upgrade cadence are defined in the [support policy](./support.md).

## Beta hardening

The next beta should reduce unknowns rather than add a large new API surface.

- Continue growing the permanent malformed corpus from minimized fuzz
  discoveries and real-world failures.
- Profile the compatibility `glyph + ttf2woff` pipeline. The beta.2 debug
  baseline is slower than classic Fontmin on the recorded machine; reach parity
  or explain an intentional correctness tradeoff.

Exit criterion: two consecutive beta releases pass the complete release gate
without manual metadata repair or platform-package rollback.

## Release candidate

The release candidate freezes user-facing contracts and changes the focus to
compatibility evidence.

- Freeze the CLI flags and exit codes, configuration schema, Node/WASM exports,
  plugin lifecycle, diagnostic codes, and generated file naming rules.
- Publish a support matrix for Node.js versions, operating systems, CPU/libc
  targets, browser WASM capabilities, and the Rust MSRV for library consumers.
- Define a deprecation policy and require migration notes for every breaking
  change from the final beta.
- Compare representative Fontmin pipelines for glyph coverage, parsed output,
  CSS semantics, and file naming; byte-for-byte equality is not required.
- Re-record benchmarks on a fixed runner with release-profile bindings. Treat a
  sustained regression greater than 10% as release-blocking after confirming it
  in at least three like-for-like runs.
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
- Migration, troubleshooting, security reporting, release, and rollback
  documentation are complete.

Work not required for 1.0—such as every historical Fontmin plugin, every font
format edge case, or distributed caching—should remain explicitly documented
as post-1.0 scope instead of delaying stable contracts indefinitely.
