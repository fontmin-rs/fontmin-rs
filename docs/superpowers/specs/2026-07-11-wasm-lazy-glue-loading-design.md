# WASM Lazy Glue Loading Design

## Goal

Load the generated WASM glue only through `initWasm()` so the published
`@fontmin-rs/wasm` entry bundle does not statically include it. Keep every
public API and its explicit-initialization contract unchanged.

## Design

`runtime.ts` owns a single promise for importing and initializing the generated
WASM glue. `initWasm(input?)` creates that promise once, invokes the generated
initializer, verifies the runtime name, and marks the runtime initialized only
after success.

`runtime.ts` also exposes an internal asynchronous accessor. It first enforces
the existing initialization error, then resolves to the cached generated module.
The accessor is private to the package implementation and is not re-exported.

`native.ts` removes its static imports from `generated/fontmin_wasm_core`.
Each public conversion wrapper already returns a promise, so its shared helpers
will await the internal accessor immediately before calling a generated export.
No public function signature, output conversion, structured error, or exact
pre-initialization error message changes.

## Error Handling

- Calling a transform before successful `initWasm()` continues to throw
  `fontmin-rs WASM runtime is not initialized; call initWasm() first`.
- A failed initialization keeps the runtime uninitialized. Repeated calls reuse
  the same rejected initialization promise, matching the current single-flight
  behavior.
- The runtime-name validation remains part of initialization.

## Verification

1. Add a focused runtime unit test that proves the accessor remains unavailable
   before initialization and resolves the loaded module afterwards.
2. Run that test red before implementation, then green after it.
3. Run the WASM package tests and build.
4. Build the documentation site and confirm the previous ineffective dynamic
   import warning is absent.

## Scope

This change only affects module loading and bundling. It does not change the
WASM ABI, transform dispatch, browser API surface, or documentation UI.
