# Runtime-neutral optimizer context

This directory owns optimizer behavior that must be identical in the Node and
browser packages.

## Owned here

- Rust diagnostic normalization.
- Custom transform return semantics.
- Sequential asset flat-mapping.
- Missing-glyph warning formatting.
- Unicode delivery-slice validation.

## Not owned here

- Filesystem paths, globs, text-file reads, cache persistence, and output
  writes belong to `../workspace-io.ts` or `../optimize-storage.ts`.
- Native binding selection belongs to `../optimize-runtime.ts`.
- WASM initialization belongs to `wasm/fontmin`.
- Format conversion belongs to Rust crates and their runtime adapters.

Keep this directory importable from browser builds. A new shared policy should
be added here only when its inputs and outputs are runtime-independent.
