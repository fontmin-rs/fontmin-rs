# Browser WASM Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish a browser-only WASM runtime that performs every supported
font transformation and the built-in in-memory plugin pipeline without loading
the Node native binding.

**Architecture:** Keep the Rust transformation crates as the single source of
font behavior and add a `wasm-bindgen` facade that exposes bytes and JSON only.
The `@fontmin-rs/wasm` TypeScript package performs asynchronous initialization
and owns browser assets, custom plugin hooks, and the in-memory pipeline. It
never imports Node built-ins or `@fontmin-rs/binding`.

**Tech Stack:** Rust 2024, `wasm-bindgen`, `serde-wasm-bindgen`,
`wasm32-unknown-unknown`, TypeScript ESM, tsdown, Vitest, Playwright, pnpm 11.

## Global Constraints

- Implement every existing transform: subset; WOFF/WOFF2/EOT/SVG round trips;
  SVG iconfont creation; CFF/CFF2 OTF-to-TTF; inspection; and CSS generation.
- Browser input and output are `Uint8Array`, strings, and named in-memory
  assets. No filesystem, CLI, globs, disk cache, or remote service is allowed.
- Browser APIs are asynchronous. Native Node APIs remain synchronous and
  native-first.
- `fallback: 'wasm'` must fail before `initWasm()` and use the initialized
  browser runtime afterwards.
- Native and WASM tests compare semantic output contracts, not encoded bytes.
- Chromium, Firefox, and WebKit must each run the browser acceptance flow.
- Shell commands use `rtk`, except `pnpm typecheck`.

## File Structure

- Create `wasm/fontmin-core/`: Rust `cdylib` facade and WASM-only tests.
- Create `wasm/fontmin/`: published TypeScript package, generated WASM glue,
  browser asset model, runtime initialization, built-in pipeline, and tests.
- Modify `Cargo.toml`: add the WASM crate to the workspace and WASM bindings.
- Modify `package.json`, `pnpm-lock.yaml`, and CI workflows: build, test, pack,
  and publish the WASM package.
- Modify `packages/fontmin/src/native.ts` and `packages/fontmin/src/types.ts`:
  route the explicit WOFF2 WASM fallback after initialization.
- Modify docs and fixtures: document browser APIs and execute parity tests.

## Shared Interfaces

```ts
// wasm/fontmin/src/index.ts
export async function initWasm(): Promise<void>
export function isWasmInitialized(): boolean

export interface BrowserAsset {
  contents: Uint8Array
  fileName: string
  format?: AssetFormat
  meta?: Record<string, unknown>
  sourceFormat?: FontFormat
}

export interface BrowserOptimizeConfig {
  assets: BrowserAsset[]
  plugins?: FontminPlugin[]
  subset?: SubsetOptions
}

export async function optimizeBrowser(
  config: BrowserOptimizeConfig,
): Promise<BrowserAsset[]>
```

```rust
// wasm/fontmin-core/src/lib.rs
#[wasm_bindgen]
pub fn transform(operation: String, input: Vec<u8>, options: JsValue) -> Result<JsValue, JsValue>;

#[wasm_bindgen]
pub fn transform_text(operation: String, input: String, options: JsValue) -> Result<JsValue, JsValue>;
```

### Task 1: Establish the compilable WASM workspace and package boundary

**Files:**
- Modify: `Cargo.toml`, `package.json`, `pnpm-workspace.yaml`
- Create: `wasm/fontmin-core/Cargo.toml`, `wasm/fontmin-core/src/lib.rs`
- Create: `wasm/fontmin/package.json`, `wasm/fontmin/src/index.ts`, `wasm/fontmin/tests/runtime.test.ts`

- [ ] **Step 1: Write a failing package test**

```ts
import { expect, it } from 'vitest'
import { initWasm, isWasmInitialized } from '../src/index'

it('initializes without importing the native binding', async () => {
  expect(isWasmInitialized()).toBe(false)
  await initWasm()
  expect(isWasmInitialized()).toBe(true)
})
```

- [ ] **Step 2: Verify RED**

Run `rtk pnpm --filter @fontmin-rs/wasm test -- runtime.test.ts`.
Expected: failure because `@fontmin-rs/wasm` and `initWasm` do not exist.

- [ ] **Step 3: Add the minimal WASM facade and browser package**

Use a `cdylib` crate with `wasm-bindgen`, `serde-wasm-bindgen`, and the shared
`fontmin` crate. Add the workspace member, a package build script that runs
`wasm-pack build --target web`, and an ESM `initWasm()` that imports the
generated initializer exactly once.

- [ ] **Step 4: Verify GREEN**

Run `rtk rustup target add wasm32-unknown-unknown`, then
`rtk cargo check -p fontmin_wasm_core --target wasm32-unknown-unknown` and the
Task 1 Vitest command. Expected: both exit `0`.

- [ ] **Step 5: Commit**

```bash
rtk git add Cargo.toml package.json pnpm-workspace.yaml wasm/fontmin-core wasm/fontmin pnpm-lock.yaml
rtk git commit -m "feat: scaffold browser wasm runtime"
```

### Task 2: Expose every Rust transformation through the WASM facade

**Files:**
- Modify: `wasm/fontmin-core/src/lib.rs`
- Create: `wasm/fontmin-core/tests/transform.rs`

- [ ] **Step 1: Write failing transform-contract tests**

Create one test table that invokes `transform` for `subsetTtf`, `ttfToWoff`,
`woffToTtf`, `ttfToWoff2`, `woff2ToTtf`, `ttfToEot`, `eotToTtf`, `ttfToSvg`,
`svgFontToTtf`, `svgsToTtf`, `otfToTtf`, and `inspect`, using existing Roboto,
static CFF, and CFF2 fixtures. Assert output magic, output format, and error
kind for invalid bytes.

- [ ] **Step 2: Verify RED**

Run `rtk cargo test -p fontmin_wasm_core --test transform`.
Expected: each operation reports an unknown operation error.

- [ ] **Step 3: Implement operation dispatch**

Deserialize each JSON option into the existing `fontmin` public option types,
call the matching `fontmin::*` function, serialize byte outputs as
`Uint8Array`, and map `FontminError` into `{ kind, message }`. Route all
operations through the shared crate; do not duplicate conversion logic.

- [ ] **Step 4: Verify GREEN**

Run `rtk cargo test -p fontmin_wasm_core --test transform` and
`rtk cargo check -p fontmin_wasm_core --target wasm32-unknown-unknown`.
Expected: both pass.

- [ ] **Step 5: Commit**

```bash
rtk git add wasm/fontmin-core
rtk git commit -m "feat: expose font transforms to wasm"
```

### Task 3: Make codec dependencies work on `wasm32-unknown-unknown`

**Files:**
- Modify: `crates/fontmin_woff2/Cargo.toml`, `crates/fontmin_woff2/src/lib.rs`
- Modify: any shared crate whose target build fails
- Test: `wasm/fontmin-core/tests/transform.rs`

- [ ] **Step 1: Write the failing WOFF2 parity test**

Assert that `ttfToWoff2(ROBOTO)` begins with `wOF2`, decodes back to TTF, and
the decoded font reports Roboto metadata when compiled for WASM.

- [ ] **Step 2: Verify RED on WASM**

Run `rtk cargo test -p fontmin_wasm_core --target wasm32-unknown-unknown --no-run`.
Expected: compilation fails at the non-WASM-compatible codec dependency.

- [ ] **Step 3: Isolate and replace only incompatible codec calls**

Add an internal `Woff2Codec` implementation selected by target cfg. The native
implementation continues using the current codec. The WASM implementation must
use a local Rust-compatible encoder/decoder with the same `Woff2Options` and
`FontminError` contract. Keep `encode_ttf_to_woff2` and
`decode_woff2_to_ttf` public signatures unchanged.

- [ ] **Step 4: Verify GREEN**

Run the Task 3 test in browser-generated WASM and `rtk cargo test -p
fontmin_woff2`. Expected: output validates and native tests remain unchanged.

- [ ] **Step 5: Commit**

```bash
rtk git add crates/fontmin_woff2 wasm/fontmin-core pnpm-lock.yaml
rtk git commit -m "feat: support woff2 codecs in wasm"
```

### Task 4: Provide the browser API for conversions and inspection

**Files:**
- Create: `wasm/fontmin/src/runtime.ts`, `wasm/fontmin/src/native.ts`, `wasm/fontmin/src/types.ts`
- Modify: `wasm/fontmin/src/index.ts`
- Test: `wasm/fontmin/tests/api.test.ts`

- [ ] **Step 1: Write failing API tests**

```ts
it('converts every supported format after initialization', async () => {
  await initWasm()
  expect(ttfToWoff2(roboto)).toHaveProperty('byteLength')
  expect(inspect(ttfToWoff2(roboto)).format).toBe('woff2')
})
```

- [ ] **Step 2: Verify RED**

Run `rtk pnpm --filter @fontmin-rs/wasm test -- api.test.ts`.
Expected: public conversion exports are absent.

- [ ] **Step 3: Implement asynchronous browser wrappers**

Export async functions for every operation in Task 2. Convert Rust structured
errors to `FontminWasmError`, preserve input `Uint8Array` ownership, and reject
all calls before `initWasm()` with one exact initialization message.

- [ ] **Step 4: Verify GREEN**

Run the Task 4 Vitest command. Expected: every conversion fixture and invalid
input assertion passes.

- [ ] **Step 5: Commit**

```bash
rtk git add wasm/fontmin
rtk git commit -m "feat: add browser font transformation api"
```

### Task 5: Implement the in-memory built-in plugin pipeline

**Files:**
- Create: `wasm/fontmin/src/optimize.ts`, `wasm/fontmin/src/plugins.ts`
- Modify: `wasm/fontmin/src/types.ts`, `wasm/fontmin/src/index.ts`
- Test: `wasm/fontmin/tests/optimize.test.ts`

- [ ] **Step 1: Write a failing complete-pipeline test**

```ts
it('runs subset, woff, woff2, and css without filesystem access', async () => {
  await initWasm()
  const assets = await optimizeBrowser({
    assets: [{ contents: roboto, fileName: 'roboto.ttf' }],
    plugins: modernWeb({ fontFamily: 'Roboto', text: 'Hello' }),
  })
  expect(assets.map(asset => asset.fileName)).toEqual([
    'roboto.ttf', 'roboto.woff', 'roboto.woff2', 'roboto.css',
  ])
})
```

- [ ] **Step 2: Verify RED**

Run `rtk pnpm --filter @fontmin-rs/wasm test -- optimize.test.ts`.
Expected: `optimizeBrowser` is absent.

- [ ] **Step 3: Implement browser-only plugin context and transforms**

Reuse plugin descriptors from `fontmin-rs`, execute built-in plugins in the
same `pre`/normal/`post` order, and provide custom plugin `emitFile` and
`warn`. Omit `cwd`, `readFile`, `resolve`, and `writeFile`; throw the exact
unsupported-browser diagnostic if a custom hook tries to use them.

- [ ] **Step 4: Verify GREEN**

Run the Task 5 test plus tests for every built-in transform and custom plugin
diagnostics. Expected: no Node built-in imports appear in `wasm/fontmin/src`.

- [ ] **Step 5: Commit**

```bash
rtk git add wasm/fontmin
rtk git commit -m "feat: add browser fontmin pipeline"
```

### Task 6: Wire the explicit WOFF2 WASM fallback

**Files:**
- Modify: `packages/fontmin/src/native.ts`, `packages/fontmin/src/types.ts`
- Test: `packages/fontmin/tests/api.test.ts`

- [ ] **Step 1: Write failing fallback tests**

Assert `ttfToWoff2(input, { fallback: 'wasm' })` reports the initialization
error before `initWasm()` and returns WOFF2 after explicit initialization.

- [ ] **Step 2: Verify RED**

Run `rtk pnpm --filter fontmin-rs test -- api.test.ts`.
Expected: the current unavailable-fallback error is returned in both cases.

- [ ] **Step 3: Implement explicit route selection**

Load `@fontmin-rs/wasm` only for `fallback: 'wasm'`; leave `native` and `auto`
unchanged. Do not add a browser conditional export for the main package.

- [ ] **Step 4: Verify GREEN and Node compatibility**

Run the Task 6 test and `rtk pnpm run test`. Expected: WASM fallback succeeds
only after initialization; all native tests still pass.

- [ ] **Step 5: Commit**

```bash
rtk git add packages/fontmin/src/native.ts packages/fontmin/src/types.ts packages/fontmin/tests/api.test.ts pnpm-lock.yaml
rtk git commit -m "feat: route woff2 wasm fallback"
```

### Task 7: Enforce parity, browser acceptance, packaging, and documentation

**Files:**
- Create: `wasm/fontmin/tests/browser-runtime.mjs`, `wasm/fontmin/tests/parity.test.ts`
- Modify: `packages/fontmin/tests/browser-load.mjs`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`
- Modify: `README.md`, `docs/api/node.md`, `docs/guide/getting-started.md`, `docs/guide/migration.md`, and matching `docs/zh/` pages

- [ ] **Step 1: Write a failing browser acceptance test**

In each Playwright engine, import the WASM package in page JavaScript, run
TTF -> subset -> WOFF2/WOFF + CSS, create a Blob URL stylesheet, and assert
`document.fonts.check()` and `document.fonts.load()` succeed for `Hello
Browser`.

- [ ] **Step 2: Verify RED**

Run `rtk pnpm --filter @fontmin-rs/wasm test:browser`.
Expected: script or browser package does not exist.

- [ ] **Step 3: Add matrix and package gates**

Add Chromium, Firefox, and WebKit jobs after building the WASM package. Add a
packed-package smoke test that installs the tarball in a clean fixture and
performs the browser flow. Add parity tests for every operation and malformed
input listed in the design's release acceptance matrix. Document initialization,
the browser asset boundary, and unsupported filesystem hooks.

- [ ] **Step 4: Verify release gate**

Run `rtk pnpm run check`, `rtk pnpm --filter @fontmin-rs/wasm test:browser`,
and `rtk pnpm --filter @fontmin-rs/wasm pack --pack-destination ../../dist`.
Expected: all commands exit `0` and the tarball contains generated ESM, types,
and WASM assets.

- [ ] **Step 5: Commit**

```bash
rtk git add README.md docs .github package.json wasm/fontmin packages/fontmin/tests pnpm-lock.yaml
rtk git commit -m "test: gate releases on browser wasm parity"
```
