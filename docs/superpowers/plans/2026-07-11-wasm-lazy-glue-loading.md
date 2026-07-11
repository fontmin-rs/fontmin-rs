# WASM Lazy Glue Loading Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Load generated WASM glue only after `initWasm()` and retain the current asynchronous browser API.

**Architecture:** `runtime.ts` owns the one dynamic-import-and-initialize promise and exposes an internal `getWasmModule()` accessor. `native.ts` obtains the generated module through that accessor immediately before invoking an export, so it has no static import of the glue module.

**Tech Stack:** TypeScript ESM, wasm-bindgen generated glue, tsdown, Vitest, VitePress.

## Global Constraints

- Preserve every export from `wasm/fontmin/src/index.ts` and every function signature in `native.ts`.
- Preserve the exact pre-initialization error: `fontmin-rs WASM runtime is not initialized; call initWasm() first`.
- Keep `initWasm(input?)` single-flight and retain its `runtime_name()` validation.
- Do not change Rust crates, generated glue files, WASM ABI, or public documentation.
- Shell commands in this repository use `rtk`, except `pnpm typecheck`.

---

### Task 1: Test the internal lazy module accessor

**Files:**
- Modify: `wasm/fontmin/test/runtime.test.ts`

**Interfaces:**
- Consumes: `initWasm(input?: InitInput)` from `wasm/fontmin/src/runtime.ts`.
- Produces: focused coverage for the internal `getWasmModule(): Promise<typeof import('../src/generated/fontmin_wasm_core')>` contract.

- [ ] **Step 1: Replace the current initialization test with isolated runtime-module tests**

Use `vi.resetModules()` before each dynamic runtime import so module-level initialization state cannot leak between tests. Add `getWasmModule` to the runtime imports and test its unavailable and initialized states:

```ts
import { readFile } from 'node:fs/promises'
import { beforeEach, expect, it, vi } from 'vitest'

const wasm = new URL(
  '../src/generated/fontmin_wasm_core_bg.wasm',
  import.meta.url,
)

beforeEach(() => {
  vi.resetModules()
})

async function loadRuntime() {
  return import('../src/runtime')
}

it('rejects module access before WASM initialization', async () => {
  const { getWasmModule } = await loadRuntime()

  await expect(getWasmModule()).rejects.toThrow(
    'fontmin-rs WASM runtime is not initialized; call initWasm() first',
  )
})

it('returns the initialized generated module', async () => {
  const { getWasmModule, initWasm, isWasmInitialized } = await loadRuntime()

  await initWasm(await readFile(wasm))

  expect(isWasmInitialized()).toBe(true)
  expect((await getWasmModule()).runtime_name()).toBe('fontmin-rs')
})
```

- [ ] **Step 2: Run the focused test to verify RED**

Run:

```bash
rtk pnpm --filter @fontmin-rs/wasm test -- runtime.test.ts
```

Expected: FAIL because `getWasmModule` is not exported by `src/runtime.ts`.

- [ ] **Step 3: Commit the failing test**

```bash
rtk git add wasm/fontmin/test/runtime.test.ts
rtk git commit -m "test: cover lazy wasm module access"
```

### Task 2: Route all native calls through the lazy accessor

**Files:**
- Modify: `wasm/fontmin/src/runtime.ts`
- Modify: `wasm/fontmin/src/native.ts`
- Test: `wasm/fontmin/test/runtime.test.ts`
- Test: `wasm/fontmin/test/api.test.ts`
- Test: `wasm/fontmin/test/optimize.test.ts`

**Interfaces:**
- Consumes: the generated module's `default`, `generate_css`, `runtime_name`, `transform`, `transform_icons`, and `transform_text` exports.
- Produces: `getWasmModule()` for package-internal consumers and unchanged public conversion helpers.

- [ ] **Step 1: Add the minimal runtime accessor**

Replace the type-only import with a module type alias and add the accessor after `assertWasmInitialized`:

```ts
type WasmModule = typeof import('./generated/fontmin_wasm_core')

let initialization: Promise<WasmModule> | undefined
let initialized = false

export async function getWasmModule(): Promise<WasmModule> {
  assertWasmInitialized()
  return initialization!
}
```

Update `initWasm` so the cached promise resolves to the imported module rather than `void`:

```ts
export async function initWasm(input?: InitInput): Promise<void> {
  initialization ??= import('./generated/fontmin_wasm_core').then(
    async module => {
      await module.default(
        input === undefined ? undefined : { module_or_path: input },
      )

      if (module.runtime_name() !== 'fontmin-rs') {
        throw new Error('fontmin-rs WASM runtime did not initialize correctly')
      }

      initialized = true
      return module
    },
  )

  await initialization
}
```

- [ ] **Step 2: Remove the static generated-module import from `native.ts`**

Replace its import block with:

```ts
import { getWasmModule } from './runtime'
```

Update the shared binary helper to access `transform` from the initialized dynamic module:

```ts
async function binary(
  operation: string,
  input: Uint8Array,
  options: Options = {},
): Promise<Uint8Array> {
  const wasm = await getWasmModule()
  return bytes(wasm.transform(operation, input, options))
}
```

For each remaining direct generated call, obtain the module and call its matching export:

```ts
export async function validateWoff2(input: Uint8Array): Promise<void> {
  ;(await getWasmModule()).transform('validateWoff2', input, {})
}

export async function ttfToSvg(
  input: Uint8Array,
  options: Options = {},
): Promise<string> {
  return (await getWasmModule()).transform('ttfToSvg', input, options) as string
}

export async function svgFontToTtf(
  input: string,
  options: Options = {},
): Promise<Uint8Array> {
  return bytes((await getWasmModule()).transform_text('svgFontToTtf', input, options))
}

export async function svgsToTtf(
  inputs: SvgIcon[],
  options: Options = {},
): Promise<Uint8Array> {
  return bytes((await getWasmModule()).transform_icons(inputs, options))
}

export async function inspect(input: Uint8Array): Promise<FontInfo> {
  return (await getWasmModule()).transform('inspect', input, {}) as FontInfo
}

export async function generateFontFaceCss(
  sources: CssFontSource[],
  options: Options = {},
): Promise<string> {
  return (await getWasmModule()).generate_css(sources, options) as string
}
```

Remove every `assertWasmInitialized` use from `native.ts`; `getWasmModule()` is the sole guard.

- [ ] **Step 3: Run the focused runtime test to verify GREEN**

Run:

```bash
rtk pnpm --filter @fontmin-rs/wasm test -- runtime.test.ts
```

Expected: PASS with two runtime tests.

- [ ] **Step 4: Run the complete WASM test suite**

Run:

```bash
rtk pnpm -C wasm/fontmin test
```

Expected: PASS; direct conversion and in-memory plugin tests confirm the unchanged public APIs use the dynamically loaded module.

- [ ] **Step 5: Commit the runtime refactor**

```bash
rtk git add wasm/fontmin/src/runtime.ts wasm/fontmin/src/native.ts wasm/fontmin/test/runtime.test.ts
rtk git commit -m "fix: load wasm glue lazily"
```

### Task 3: Verify production bundling and documentation integration

**Files:**
- Test only: `wasm/fontmin/package.json`
- Test only: `docs/package.json`

**Interfaces:**
- Consumes: `@fontmin-rs/wasm` ESM build and the VitePress Playground import.
- Produces: evidence that production consumers build without the ineffective-dynamic-import diagnostic.

- [ ] **Step 1: Build the WASM package**

Run:

```bash
rtk pnpm -C wasm/fontmin build
```

Expected: PASS and emit the ESM runtime plus its WASM asset.

- [ ] **Step 2: Build the documentation site**

Run:

```bash
rtk pnpm -C docs build
```

Expected: PASS without `[INEFFECTIVE_DYNAMIC_IMPORT]` for `fontmin_wasm_core.js`.

- [ ] **Step 3: Run browser and workspace regression checks**

Run:

```bash
rtk pnpm --filter @fontmin-rs/wasm test:browser
rtk pnpm run check
```

Expected: PASS; the browser flow loads the lazy glue, and the full repository quality gate remains green.

- [ ] **Step 4: Commit verification-only generated artifacts if the package build changes tracked output**

Run:

```bash
rtk git status --short
```

Expected: no changes. If only `wasm/fontmin/dist/**` changed, do not commit it because the package manifest publishes it as build output rather than source. If a tracked source file changed unexpectedly, stop and inspect before continuing.

## Plan Self-Review

- Spec coverage: Task 1 verifies the unavailable and initialized accessor states; Task 2 preserves the public asynchronous API while removing static glue imports; Task 3 verifies both package and documentation bundles.
- Placeholder scan: no TODO/TBD/deferred items are present.
- Type consistency: `getWasmModule()` is typed from the generated ESM module and is the only internal module accessor used by `native.ts`.
