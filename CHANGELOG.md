# Changelog

All notable changes to fontmin-rs are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-beta.1] - 2026-07-13

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

[0.1.0-beta.1]: https://github.com/fontmin-rs/fontmin-rs/compare/dba7532...v0.1.0-beta.1
