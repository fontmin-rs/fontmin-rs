# Rust Task 4 Report: Verify Module Config CLI Parity

## Status

Complete. End-to-end CLI coverage now proves every supported module extension,
real package preset imports, CLI override parity, config-relative paths,
discovery precedence, and Node-independent JSONC execution.

## Coverage Added

- `.ts`, `.mts`, `.mjs`, and `.cjs` configurations through the compiled Rust
  CLI, including typed TypeScript, default and named exports, synchronous
  objects, and asynchronous factories.
- A real self-import of `defineConfig` and `modernWeb` from
  `packages/fontmin`, asserting WOFF, WOFF2, CSS, and the configured family.
  The preset keeps its established sequential clone semantics; the test does
  not pass `clone: false` while expecting all three derived assets.
- Module CLI overrides for input, output directory, text, formats, cache,
  CSS family/path/glyph/ranges, delivery slices, and variation coordinates.
- Config-directory resolution for relative input, output, cache, top-level
  subset `textFile`, and every explicit glyph plugin `textFile`. The resulting
  glyph count independently proves existing `text` and file contents append.
- Automatic discovery preference for TypeScript over JSONC.
- A dedicated missing-Node diagnostic with an empty `PATH`.
- A successful JSONC build with an empty `PATH`, invoking the already-built
  CLI binary directly without relying on a shell command.

## TDD and Precondition Evidence

The requested extension precondition was green on Node v26.5.0:

```text
rtk cargo test -p fontmin_app module_config_extensions -- --nocapture
1 passed
```

The broader focused test exposed two failures before production fixes:

```text
rtk cargo test -p fontmin_app module_config_ -- --nocapture
4 passed; 2 failed
```

1. The real `modernWeb({ text, fontFamily })` import failed because preset-only
   options leaked into the WOFF descriptor and the strict bridge rejected
   `plugins[2].native.options.fontFamily`.
2. The missing-Node assertion expected an unwrapped diagnostic, while Miette
   correctly wrapped the same dedicated message for terminal display.

After scoping preset options and making the diagnostic assertion wrap-safe,
the modern preset still revealed a second real parity bug: Rust config defaults
added top-level compatibility outputs and CSS when a module supplied only
explicit plugins. That later CSS plugin overwrote the preset family.

The module bridge now preserves plugin-only intent by supplying `outputs: []`
and `css: null` only when `plugins` is present and those keys are absent.
Explicit top-level outputs/CSS remain unchanged, including documented duplicate
operation semantics.

## Production Fixes

- `modernWeb()` now sends only WOFF-supported options to `ttf2woff` and only
  WOFF2-supported options to `ttf2woff2`. A package regression test checks the
  exact descriptors.
- Module configs containing explicit plugins no longer accidentally acquire
  compatibility output/CSS defaults when those fields were omitted.

## Verification

```text
rtk cargo test -p fontmin_app -- --nocapture
84 passed across 2 suites

rtk pnpm --filter fontmin-rs test
141 passed across 3 files

rtk pnpm --filter fontmin-rs format:check
passed

rtk cargo fmt --all -- --check
passed

rtk git diff --check
passed
```

## Self-review

- Tests use only the public CLI seam and output artifacts for Rust parity.
- The TypeScript test is unconditional and therefore exposes unsupported Node
  runtimes instead of skipping.
- The modern preset test imports the real package export, not a hand-written
  descriptor approximation.
- The empty-`PATH` tests execute the absolute Cargo-provided binary path.
- Relative glyph file coverage includes multiple explicit descriptors and
  independently distinguishes appended `Hello` from file-only `ello`.
- No release, version, publication, or clone-semantics changes were made.
