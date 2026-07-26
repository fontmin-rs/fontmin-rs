# Changelog

All notable changes to fontmin-rs are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.0-rc.1]: https://github.com/fontmin-rs/fontmin-rs/compare/v0.1.0-beta.4...v0.1.0-rc.1
[0.1.0-beta.4]: https://github.com/fontmin-rs/fontmin-rs/compare/v0.1.0-beta.3...v0.1.0-beta.4
[0.1.0-beta.3]: https://github.com/fontmin-rs/fontmin-rs/compare/v0.1.0-beta.2...v0.1.0-beta.3
[0.1.0-beta.2]: https://github.com/fontmin-rs/fontmin-rs/compare/v0.1.0-beta.1...v0.1.0-beta.2
[0.1.0-beta.1]: https://github.com/fontmin-rs/fontmin-rs/compare/dba7532...v0.1.0-beta.1
