# Changelog

All notable changes to fontmin-rs are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-07-28

### Added

- Added a digest-pinned production corpus for a large CJK variable font and a
  bitmap color font, with cached native/WASM metadata and mixed-delivery
  conformance checks.
- Added minimized malformed table fixtures for duplicate SFNT tags and
  overlapping WOFF table data.
- Added isolated native/WASM production performance stages with median latency
  and peak-RSS budgets, plus reports that name every regressed stage.
- Added focused parser, converter, configuration, and output-naming fuzz
  targets with target-specific deterministic and permanent regression corpora.
- Added machine-checked duplicate dependency and vendored patch decisions, plus
  release artifact size budgets published by CI.

### Changed

- Enabled thin LTO, single codegen units, and symbol stripping for release
  builds, reducing the measured CLI and native binding sizes.

### Fixed

- Kept generated N-API JavaScript and declarations formatted after release
  builds so benchmark and production-conformance commands leave no source diff.
- Reclaimed cache locks after owner-process termination, released Rust cache
  locks when write tasks are cancelled, and removed incomplete temporary files
  before the next update.
- Aligned the release WASM optimizer feature set with the pinned Rust toolchain
  so CI, documentation, and release builds validate the same module features.

## [0.2.0] - 2026-07-28

### Changed

- Made the published npm executable a thin adapter and routed direct,
  configuration, and icon-font builds through the shared Node optimizer.
- Split the Node optimizer behind its unchanged `optimize()` facade into
  pipeline orchestration, transform rules, and filesystem/cache modules.
- Split the Node and Rust CLI integration suites by public API and command
  seams, with structure checks that keep individual suites bounded.
- Marked every Rust workspace package as internal-only through one inherited
  Cargo publish policy and added a metadata check that prevents drift.
- Deserialized Rust built-in plugin descriptors into typed
  `fontmin_config` variants before pipeline construction, removing duplicate
  JSON option models from `fontmin_pipeline`.
- Moved built-in Rust asset metadata for icon Unicode values, CSS glyphs, and
  CSS Unicode ranges into typed `AssetMeta` fields while retaining `custom`
  for third-party extensions.

### Fixed

- Kept the release version bump targets aligned with the refactored Node CLI
  and optimizer modules.
- Made the Rust publish-policy contract test portable across LF and CRLF
  checkouts.

## [0.1.1] - 2026-07-28

### Changed

- Aligned public installation guidance and the machine-readable contract with
  the stable `0.1` release line.
- Replaced the completed first-stable roadmap with milestone-based work toward
  `1.0`.

### Fixed

- Added a release-state contract test that rejects prerelease install tags when
  the published package version is stable.
- Made the packaged npm CLI accept and apply `--css-unicode-range` and
  `--delivery-slice`, with its complete help surface checked against the public
  CLI contract.
- Standardized an SVG icon collection without an explicit `fontName` on the
  documented `iconfont.ttf` default across Rust, Node.js, and browser pipelines.
- Prevented release version bumps from rewriting an unrelated external Cargo
  dependency that happens to share the workspace version.

## [0.1.0] - 2026-07-28

### Changed

- Promoted the release candidate to the first stable release after completing
  the full Rust, Node.js, native, and browser WASM verification matrix.
- Kept the root and Fuzz Cargo lockfiles synchronized during workspace version
  updates, with portable release checks that invoke Cargo directly.

### Fixed

- Preserved the full WOFF2 transformed-glyph coordinate range, including the
  minimum signed delta, while rejecting out-of-range cumulative coordinates.
- Accepted valid four-operand CFF `endchar` sequences when no explicit glyph
  width is present.

### Known limitations

- Rust CLI module configuration requires Node.js 22 or newer.
- Arbitrary JavaScript plugin hooks run only in the Node pipeline.
- CFF2 conversion produces a static TrueType instance and removes variation tables.
- `ttf-parser` remains unmaintained and has no safe upgrade in the current dependency graph.

## [0.1.0-rc.3] - 2026-07-26

### Changed

- Fuzz regression promotion now opens a tracking issue with a manual comparison
  link when organization policy prevents GitHub Actions from creating a pull
  request.
- Preserved the minimized malformed-TTF reproducer in the fuzz corpus and
  shared fixture manifest for Rust, CLI, Node.js, and browser WASM checks.

### Fixed

- Rejected invalid SFNT table-count search parameters before invoking the TTF
  subsetter, preventing malformed input from triggering a shift-overflow panic.

### Known limitations

- This is a release candidate; changing a frozen public contract restarts the
  RC validation cycle.
- Rust CLI module configuration requires Node.js 22 or newer.
- Arbitrary JavaScript plugin hooks run only in the Node pipeline.
- CFF2 conversion produces a static TrueType instance and removes variation tables.
- `ttf-parser` remains unmaintained and has no safe upgrade in the current dependency graph.

## [0.1.0-rc.2] - 2026-07-26

### Changed

- Aligned output overrides, clone behavior, plugin cleanup, and packaged CLI
  behavior across the Rust, Node.js, and browser WASM pipelines.
- Cache writers now use atomic writes and owner-scoped locks across the Rust
  CLI, Node.js API, and packaged CLI.
- Release-candidate installation guidance now selects the npm `rc` dist-tag
  explicitly and distinguishes the `0.1.0` stable gate from a future 1.0 cycle.
- Every GitHub Action is pinned to a full commit SHA, with repository tokens
  read-only by default and write access limited to publishing jobs.

### Fixed

- Refused unsafe output cleanup, path traversal, and symbolic-link writes while
  preserving nested output file names.
- Prevented an old cache writer from deleting a replacement lock owned by a
  successor process.
- Kept release workflow policy tests portable across LF and CRLF checkouts.

### Security

- Bounded executable config evaluation, decompression, and generated output
  paths, and hardened CSS string and option escaping.

### Known limitations

- This is a release candidate; changing a frozen public contract restarts the
  RC validation cycle.
- Rust CLI module configuration requires Node.js 22 or newer.
- Arbitrary JavaScript plugin hooks run only in the Node pipeline.
- CFF2 conversion produces a static TrueType instance and removes variation tables.
- `ttf-parser` remains unmaintained and has no safe upgrade in the current dependency graph.

## [0.1.0-rc.1] - 2026-07-26

### Added

- A machine-readable release-candidate contract that freezes CLI commands and
  flags, exit codes, Rust/Node/browser configuration boundaries, Node/WASM
  exports, plugin lifecycle hooks, diagnostic codes, and generated file names.
- English and Chinese contract documentation linked to the support,
  deprecation, security reporting, troubleshooting, and rollback policies.

### Changed

- Install, ESM, CLI, native, automatic fallback, forced-WASM, and browser
  consumer checks now install packed tarballs instead of importing workspace
  build output.
- Release verification now exercises packed Node and browser WASM packages in
  Chromium before npm publication.

### Known limitations

- This is a release candidate; changing a frozen public contract restarts the
  RC validation cycle.
- Rust CLI module configuration requires Node.js 22 or newer.
- Arbitrary JavaScript plugin hooks run only in the Node pipeline.
- CFF2 conversion produces a static TrueType instance and removes variation tables.
- `ttf-parser` remains unmaintained and has no safe upgrade in the current dependency graph.

## [0.1.0-beta.4] - 2026-07-26

### Changed

- Revalidated the unchanged beta.3 public behavior through a second complete
  beta release gate, including all 11 npm packages, native targets, browser
  runtimes, provenance, and rollback checks.

### Known limitations

- This is a prerelease; the public interface may still change before `1.0.0`.
- Rust CLI module configuration requires Node.js 22 or newer.
- Arbitrary JavaScript plugin hooks run only in the Node pipeline.
- CFF2 conversion produces a static TrueType instance and removes variation tables.
- `ttf-parser` remains unmaintained and has no safe upgrade in the current dependency graph.

## [0.1.0-beta.3] - 2026-07-26

### Added

- Reproducible CJK, icon-font, CFF/CFF2, and malformed fixture inventories with immutable source, license, and SHA-256 metadata.
- Shared Native/WASM conformance coverage for every built-in transform and preset, including stable malformed-input diagnostics.
- Bounded AddressSanitizer fuzzing with deterministic minimization and reviewable promotion into a permanent regression corpus.
- Typed `FontminDiagnosticError` codes across the Node native and browser WASM APIs.
- An explicit Node.js, native target, browser, and Rust MSRV support matrix.

### Changed

- Release performance is measured with release-profile bindings over three paired trials on a fixed software runner.
- CI now checks Node.js 22, 24, and 26, Rust 1.88.0 compatibility, all published native targets, and the current browser engine matrix.
- The pinned development and release toolchain is Rust 1.97.1; WASM publishing uses wasm-pack 0.15.0.

### Fixed

- Rejected malformed SVG outlines, invalid WOFF2 contour endpoints and coordinate overflows, and out-of-range CFF INDEX offsets without panicking.
- Preserved structured diagnostic codes through the Rust CLI, Node native binding, and browser WASM boundary.
- Prevented CLI help rendering from panicking on current command metadata.

### Security

- Removed dependency-advisory exceptions by replacing or locally hardening affected transitive crates.

### Known limitations

- This is a prerelease; the public interface may still change before `1.0.0`.
- Rust CLI module configuration requires Node.js 22 or newer.
- Arbitrary JavaScript plugin hooks run only in the Node pipeline.
- CFF2 conversion produces a static TrueType instance and removes variation tables.
- `ttf-parser` remains unmaintained and has no safe upgrade in the current dependency graph.

## [0.1.0-beta.2] - 2026-07-21

### Added

- Character coverage auditing across the Rust, CLI, Node.js, and browser WASM APIs.
- Missing-glyph policies for subset and build workflows, including warning and strict failure modes.
- Coverage reporting in the browser playground.

### Changed

- npm releases now use GitHub Actions trusted publishing with provenance.
- Documented and tested the routine release workflow.
- Updated pnpm, documentation tooling, and bundled development dependencies.

### Security

- Updated the transitive `tar` override to 7.5.19 to address high- and critical-severity advisories.

### Known limitations

- This is a prerelease; the public interface may still change before `1.0.0`.
- Rust CLI module configuration requires Node.js 22 or newer.
- Arbitrary JavaScript plugin hooks run only in the Node pipeline.
- CFF2 conversion produces a static TrueType instance and removes variation tables.
- `ttf-parser` and the transitive `paste` crate are unmaintained; neither has a safe upgrade in the current dependency graph.

## [0.1.0-beta.1] - 2026-07-16

### Added

- Rust, Node.js, and browser WASM font processing runtimes.
- TTF subsetting and TTF, OTF, WOFF, WOFF2, EOT, and SVG conversions.
- Built-in plugins, modern web and Fontmin-compatible presets, and an asset pipeline.
- Rust CLI commands for initialization, build, subset, convert, inspect, doctor, and benchmarks.
- JSON, JSONC, TypeScript, and JavaScript configuration loading in the Rust CLI.
- Cache-aware Node optimization with native, WASM, and automatic runtime selection.
- Browser playground, multilingual documentation, package smoke tests, and cross-platform CI.

### Known limitations

- This is a prerelease; the public interface may still change before `1.0.0`.
- Rust CLI module configuration requires Node.js 22 or newer.
- Arbitrary JavaScript plugin hooks run only in the Node pipeline.
- CFF2 conversion produces a static TrueType instance and removes variation tables.
- `ttf-parser` and the transitive `paste` crate are unmaintained; neither has a safe upgrade in the current dependency graph.

[0.3.0]: https://github.com/fontmin-rs/fontmin-rs/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/fontmin-rs/fontmin-rs/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/fontmin-rs/fontmin-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/fontmin-rs/fontmin-rs/compare/v0.1.0-rc.3...v0.1.0
[0.1.0-rc.3]: https://github.com/fontmin-rs/fontmin-rs/compare/v0.1.0-rc.2...v0.1.0-rc.3
[0.1.0-rc.2]: https://github.com/fontmin-rs/fontmin-rs/compare/v0.1.0-rc.1...v0.1.0-rc.2
[0.1.0-rc.1]: https://github.com/fontmin-rs/fontmin-rs/compare/v0.1.0-beta.4...v0.1.0-rc.1
[0.1.0-beta.4]: https://github.com/fontmin-rs/fontmin-rs/compare/v0.1.0-beta.3...v0.1.0-beta.4
[0.1.0-beta.3]: https://github.com/fontmin-rs/fontmin-rs/compare/v0.1.0-beta.2...v0.1.0-beta.3
[0.1.0-beta.2]: https://github.com/fontmin-rs/fontmin-rs/compare/v0.1.0-beta.1...v0.1.0-beta.2
[0.1.0-beta.1]: https://github.com/fontmin-rs/fontmin-rs/compare/dba7532...v0.1.0-beta.1
