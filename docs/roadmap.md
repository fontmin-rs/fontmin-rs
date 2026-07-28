# Roadmap to 1.0

fontmin-rs `0.1.0` is the first stable release across the CLI, Node.js
package, browser WASM package, native binding, and eight platform packages.
The next milestones deepen the implementation behind that public contract
before a separately validated `1.0.0` contract is finalized.

This roadmap uses exit criteria instead of calendar promises. A milestone is
complete only when its checks are repeatable on `main`, packed-package smoke
tests cover the affected public paths, and the release workflow remains
reproducible.

## Stable baseline

- One release version is validated across all 11 npm packages, Cargo metadata,
  embedded runtime versions, the changelog, and the release tag.
- CI covers formatting, warning-free Rust and TypeScript linting, Node.js
  22/24/26, WASM, browser loading, documentation, native package smoke tests,
  release readiness, and benchmarks.
- The release gate rejects high or critical dependency advisories, requires at
  least 80% Rust line coverage, inspects packed npm contents, and runs consumer
  smoke tests.
- Native and WASM share a semantic conformance corpus across built-in
  transforms, presets, output metadata, and malformed diagnostics.
- A bounded AddressSanitizer cargo-fuzz target runs on relevant changes and a
  weekly schedule; minimized crashes become permanent malformed fixtures.
- Rust `1.88.0` is the tested MSRV. Development and release automation use the
  pinned toolchain declared by the repository.

## 0.1.1 — contract correction

The first patch release removes drift discovered immediately after `0.1.0`
without expanding the public API.

- Align README files, installation guides, navigation labels, and the
  machine-readable inventory with the stable release.
- Make the packaged npm CLI accept every frozen Rust CLI flag and verify both
  executables from the same contract.
- Make the default SVG icon-font stem consistently `iconfont` across Rust,
  Node.js, and browser pipelines.
- Add semantic checks that prevent stable package versions from publishing
  prerelease installation guidance.

Exit criterion: the full release gate passes, packed npm and Rust CLI behavior
matches the public inventory, and `0.1.1` is published to `latest`.

## 0.2 — consolidate pipeline boundaries

The `0.2` line reduces duplicate policy while preserving the stable public
entry points.

- Turn the npm executable into a thin adapter over shared command parsing and
  pipeline behavior instead of maintaining a second independent CLI.
- Normalize built-in plugin configuration into typed domain values in
  `fontmin_config`; remove repeated JSON option decoding from
  `fontmin_pipeline`.
- Replace well-known `AssetMeta.custom` keys with typed metadata while keeping
  an extension map for third-party plugins.
- Split the Node optimizer by pipeline execution, transform rules, and
  filesystem/cache ownership while retaining the current `optimize()` facade.
- Split oversized CLI and Node integration tests by public command or API seam.
- Decide whether Rust workspace crates are internal-only or independently
  publishable, then make Cargo manifests enforce that decision.
- Keep the pre-`0.1.0` design proposal clearly historical; current
  architecture and contract documents remain authoritative.

Exit criterion: each public entry point retains its contract, duplicate CLI
and configuration rules have one source of truth, and the full conformance and
package gates pass without compatibility exceptions.

## 0.3 — real-world resilience and performance

The `0.3` line builds evidence from production-sized inputs and reduces
remaining operational risk.

- Expand conformance fixtures for large CJK fonts, variable fonts, color
  fonts, malformed tables, and mixed delivery slices.
- Add bounded memory and latency budgets for native and WASM processing, with
  regression reports that identify the responsible stage.
- Grow fuzz corpora from real failures and run focused targets for parsers,
  converters, configuration loading, and output naming.
- Audit duplicate compression and error-handling dependencies, vendored
  patches, binary size, and upstream replacement paths.
- Validate cache concurrency, cancellation, and cleanup under interrupted
  multi-process builds.

Exit criterion: representative large fonts stay within documented performance
budgets, every supported format path has regression fixtures, and known
vendored/dependency risks have an owner and replacement decision.

## 1.0 — independently validated contract

`1.0.0` is not a rename of the `0.1` contract. It begins with its own release
candidate after evidence from real projects has shaped the final surface.

- Collect compatibility reports from CLI, Node.js, and browser consumers.
- Publish migration guidance for every intentional contract change.
- Complete the announced deprecation windows and remove only APIs eligible
  under the deprecation policy.
- Reconfirm supported runtimes, native targets, browser capabilities, Rust
  MSRV, diagnostics, and generated naming rules.
- Pass at least one release-candidate cycle with no unresolved P0/P1
  correctness, security, performance, or packaging issue.

Exit criterion: the independently versioned RC contract survives the complete
release gate and real-project validation, then the same commit is promoted to
`1.0.0` without release-time repair.
