# Node Async WASM Fallback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a native-first asynchronous WOFF2 helper that falls back to the packaged WASM runtime when N-API cannot load.

**Architecture:** Move the N-API lookup behind a cached `createRequire()` loader so importing the root package does not require a native artifact. Add a separate Node-only WASM loader that dynamically imports `@fontmin-rs/wasm`, reads its adjacent binary, initializes it once, and returns WOFF2 bytes to `ttfToWoff2Async()`.

**Tech Stack:** TypeScript ESM, Node.js `createRequire`, dynamic import, `@fontmin-rs/wasm`, Vitest, pnpm 11.

## Global Constraints

- Keep every existing synchronous helper and `optimize()` behavior native-only and synchronous.
- Expose only `ttfToWoff2Async()`; do not add asynchronous variants for unrelated transforms.
- `auto` falls back only after a native binding-load failure, never after a conversion error.
- `wasm` initializes from the installed package's `.wasm` asset without caller setup.
- `js` remains an explicit unavailable fallback.
- Do not change versions, release workflows, publish configuration, or run publication commands.

---

### Task 1: Defer Native Binding Loading Without Changing Sync Behavior

**Files:**
- Create: `packages/fontmin/src/native-loader.ts`
- Modify: `packages/fontmin/src/native.ts`
- Test: `packages/fontmin/test/api.test.ts`

**Interfaces:**

```ts
export class NativeBindingLoadError extends Error {}
export function loadNativeBinding(): typeof import('@fontmin-rs/binding')
```

- [ ] **Step 1: Write a failing import-safety test**

Add a Vitest module-isolation test that imports the native loader with a
throwing binding resolver and asserts it returns `NativeBindingLoadError` only
when a transformation is called, not when the package module is evaluated.

- [ ] **Step 2: Run the focused test to verify RED**

Run: `rtk pnpm --filter fontmin-rs test -- native-loader`

Expected: FAIL because the current static package import loads the binding
immediately.

- [ ] **Step 3: Add the lazy loader and route sync helpers through it**

```ts
const require = createRequire(import.meta.url)
let binding: NativeBinding | undefined

export function loadNativeBinding(): NativeBinding {
  try {
    return (binding ??= require('@fontmin-rs/binding') as NativeBinding)
  } catch (cause) {
    throw new NativeBindingLoadError('fontmin-rs native binding is unavailable', { cause })
  }
}
```

Replace every former top-level native binding call in `native.ts` with a call
to `loadNativeBinding()` at operation time. Preserve all option mapping and
return types.

- [ ] **Step 4: Run sync regression tests to verify GREEN**

Run: `rtk pnpm --filter fontmin-rs test -- api.test.ts`

Expected: PASS, including existing subset, conversion, CSS, and pipeline tests.

- [ ] **Step 5: Commit**

```bash
rtk git add packages/fontmin/src/native-loader.ts packages/fontmin/src/native.ts packages/fontmin/test/api.test.ts
rtk git commit -m "refactor: defer native binding loading"
```

### Task 2: Add the Packaged WASM Loader and Async WOFF2 API

**Files:**
- Create: `packages/fontmin/src/wasm-fallback.ts`
- Modify: `packages/fontmin/src/native.ts`, `packages/fontmin/src/index.ts`, `packages/fontmin/package.json`, `packages/fontmin/test/api.test.ts`

**Interfaces:**

```ts
export async function loadWasmWoff2Encoder(): Promise<{
  ttfToWoff2(input: Uint8Array, options: { quality?: number }): Promise<Uint8Array>
}>

export async function ttfToWoff2Async(
  input: Uint8Array,
  options?: Ttf2Woff2Options,
): Promise<Buffer>
```

- [ ] **Step 1: Write failing async fallback tests**

Add tests that:

```ts
const output = await ttfToWoff2Async(input, { fallback: 'wasm' })
expect(output.subarray(0, 4).toString('ascii')).toBe('wOF2')
await expect(ttfToWoff2Async(input, { fallback: 'js' })).rejects.toThrow(
  'WOFF2 fallback `js` is not available',
)
```

Also retain the existing assertion that the synchronous helper rejects
`fallback: 'wasm'`.

- [ ] **Step 2: Run focused tests to verify RED**

Run: `rtk pnpm --filter fontmin-rs test -- api.test.ts`

Expected: FAIL because `ttfToWoff2Async` is not exported.

- [ ] **Step 3: Implement WASM package initialization**

Use `createRequire().resolve('@fontmin-rs/wasm')` to locate the installed ESM
entry, `readFile()` the sibling `fontmin_wasm_core_bg.wasm`, dynamically import
`@fontmin-rs/wasm`, and memoize `initWasm(bytes)` plus the encoder module.
Declare `@fontmin-rs/wasm` in `packages/fontmin/package.json` dependencies.

- [ ] **Step 4: Implement fallback selection**

```ts
if (options.fallback === 'wasm') return encodeWithWasm(input, options)
if (options.fallback === 'js') throw unavailableFallback('js')
try {
  return ttfToWoff2(input, { ...options, fallback: 'native' })
} catch (error) {
  if (options.fallback !== 'auto' || !(error instanceof NativeBindingLoadError)) throw error
  return encodeWithWasm(input, options)
}
```

Convert the WASM result to `Buffer`, forward only `quality`, and wrap only
WASM loading or conversion failures with `WOFF2 WASM fallback failed`.

- [ ] **Step 5: Run focused verification**

Run: `rtk pnpm --filter fontmin-rs test -- api.test.ts && pnpm --filter fontmin-rs typecheck`

Expected: PASS with a real WOFF2 produced through WASM and no type errors.

- [ ] **Step 6: Commit**

```bash
rtk git add packages/fontmin/package.json packages/fontmin/src/wasm-fallback.ts packages/fontmin/src/native.ts packages/fontmin/src/index.ts packages/fontmin/test/api.test.ts pnpm-lock.yaml
rtk git commit -m "feat: add async wasm woff2 fallback"
```

### Task 3: Document the Explicit Async Boundary and Verify Packaging

**Files:**
- Modify: `docs/api/node.md`, `docs/guide/migration.md`
- Test: `packages/fontmin/test/api.test.ts`, `wasm/fontmin/test/runtime.test.ts`

- [ ] **Step 1: Write documentation assertions or update existing API examples**

Document this exact usage:

```ts
const output = await ttfToWoff2Async(input, { fallback: 'auto' })
```

State that synchronous APIs and `optimize()` remain native-only, that `wasm`
is only available on the new async helper, and that `js` is unsupported.

- [ ] **Step 2: Verify package build and runtime behavior**

Run: `rtk pnpm --filter fontmin-rs build && rtk pnpm --filter fontmin-rs test -- api.test.ts && rtk pnpm -C wasm/fontmin test`

Expected: package build and both test suites exit `0`.

- [ ] **Step 3: Run repository non-publishing verification**

Run: `rtk pnpm run check`

Expected: exit `0`; existing lint warnings may remain, but there are no errors.

- [ ] **Step 4: Commit**

```bash
rtk git add docs/api/node.md docs/guide/migration.md
rtk git commit -m "docs: document async wasm woff2 fallback"
```
