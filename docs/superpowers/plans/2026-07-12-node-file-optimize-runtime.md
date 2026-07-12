# Node File Optimize Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a pipeline-wide native/WASM/auto runtime to the Node file-based `optimize()` API without moving Node file and plugin behavior into the WASM package.

**Architecture:** Introduce one asynchronous runtime adapter and selector per `optimize()` call. Convert built-in transforms to await that adapter, keep custom JavaScript hooks and file operations in `optimize.ts`, and include requested plus resolved runtime identity in cache keys.

**Tech Stack:** TypeScript ESM, Node.js `Buffer` and filesystem APIs, `@fontmin-rs/wasm`, Vitest, pnpm 11.

## Global Constraints

- `FontminConfig.runtime` accepts exactly `'native'`, `'wasm'`, or `'auto'` and defaults to `'native'`.
- One `optimize()` call resolves exactly one runtime for all built-in operations.
- `auto` falls back only for `NativeBindingLoadError`; font and encoder errors never trigger a retry.
- Node retains file I/O, globbing, caching, custom plugin hooks, and output writes.
- Existing synchronous direct APIs and `ttfToWoff2Async()` remain compatible.
- An omitted config runtime may derive the pipeline runtime from a legacy WOFF2 plugin fallback; conflicting values fail.
- Unsupported WASM options fail explicitly and are never silently removed.
- Do not change package versions, release workflows, or publish anything.

---

### Task 1: Define the Runtime Contract and Deterministic Selector

**Files:**
- Create: `packages/fontmin/src/optimize-runtime.ts`
- Modify: `packages/fontmin/src/types.ts`
- Modify: `packages/fontmin/src/wasm-fallback.ts`
- Test: `packages/fontmin/test/optimize-runtime.test.ts`

**Interfaces:**
- Produces: `RuntimeMode`, `OptimizeRuntime`, `RuntimeSelector`, `createRuntimeSelector()`, and `resolvePipelineRuntimeMode()`.
- Consumes: existing direct native helpers, `loadNativeBinding()`, and the packaged WASM loader.

- [ ] **Step 1: Write selector tests before production code**

Create `packages/fontmin/test/optimize-runtime.test.ts` with real fake adapters and loaders:

```ts
import { describe, expect, it, vi } from 'vitest'
import {
  createRuntimeSelector,
  resolvePipelineRuntimeMode,
  type OptimizeRuntime,
} from '../src/optimize-runtime'
import { NativeBindingLoadError } from '../src/native-loader'

function runtime(kind: 'native' | 'wasm'): OptimizeRuntime {
  return {
    kind,
    generateFontFaceCss: vi.fn(),
    inspect: vi.fn(),
    otfToTtf: vi.fn(),
    subsetTtf: vi.fn(),
    svgFontToTtf: vi.fn(),
    svgsToTtf: vi.fn(),
    ttfToEot: vi.fn(),
    ttfToSvg: vi.fn(),
    ttfToWoff: vi.fn(),
    ttfToWoff2: vi.fn(),
  }
}

describe('optimize runtime selection', () => {
  it('memoizes one explicit WASM adapter', async () => {
    const wasm = runtime('wasm')
    const loadWasm = vi.fn(async () => wasm)
    const selector = createRuntimeSelector('wasm', {
      loadNative: vi.fn(),
      loadWasm,
    })

    expect(await selector.resolve()).toBe(wasm)
    expect(await selector.resolve()).toBe(wasm)
    expect(loadWasm).toHaveBeenCalledOnce()
  })

  it('auto falls back only when the native binding cannot load', async () => {
    const wasm = runtime('wasm')
    const selector = createRuntimeSelector('auto', {
      loadNative() {
        throw new NativeBindingLoadError(new Error('missing artifact'))
      },
      loadWasm: async () => wasm,
    })

    expect((await selector.resolve()).kind).toBe('wasm')
  })

  it('auto preserves non-load native failures', async () => {
    const failure = new Error('native setup failed')
    const selector = createRuntimeSelector('auto', {
      loadNative() {
        throw failure
      },
      loadWasm: vi.fn(),
    })

    await expect(selector.resolve()).rejects.toBe(failure)
  })

  it('does not switch runtime after a selected native operation fails', async () => {
    const native = runtime('native')
    const failure = new Error('invalid font')
    vi.mocked(native.ttfToWoff2).mockRejectedValue(failure)
    const loadWasm = vi.fn()
    const selector = createRuntimeSelector('auto', {
      loadNative: () => native,
      loadWasm,
    })

    const selected = await selector.resolve()
    await expect(selected.ttfToWoff2(new Uint8Array(), {})).rejects.toBe(failure)
    expect(loadWasm).not.toHaveBeenCalled()
  })

  it('derives a legacy pipeline mode and rejects conflicts', () => {
    expect(resolvePipelineRuntimeMode(undefined, ['wasm'])).toBe('wasm')
    expect(() => resolvePipelineRuntimeMode('native', ['wasm'])).toThrow(
      'runtime `native` conflicts with WOFF2 fallback `wasm`',
    )
    expect(() => resolvePipelineRuntimeMode(undefined, ['auto', 'wasm'])).toThrow(
      'conflicting WOFF2 fallback modes',
    )
  })
})
```

- [ ] **Step 2: Run the new test and verify RED**

Run: `rtk pnpm --filter fontmin-rs test -- optimize-runtime.test.ts`

Expected: FAIL because `../src/optimize-runtime` does not exist.

- [ ] **Step 3: Add the public mode and internal runtime interfaces**

Add to `packages/fontmin/src/types.ts`:

```ts
export type RuntimeMode = 'native' | 'wasm' | 'auto'

export interface FontminConfig {
  cwd?: string
  input?: (string | Uint8Array)[]
  outDir?: string
  clean?: boolean
  preserveOriginal?: boolean
  cache?: boolean | CacheOptions
  subset?: SubsetOptions
  outputs?: ConfigOutput[]
  css?: CssOptions
  plugins?: FontminPlugin[]
  runtime?: RuntimeMode
}
```

Create `packages/fontmin/src/optimize-runtime.ts` with these exact contracts:

```ts
export interface OptimizeRuntime {
  readonly kind: Exclude<RuntimeMode, 'auto'>
  generateFontFaceCss(sources: CssFontSource[], options: CssOptions): Promise<string>
  inspect(input: Uint8Array): Promise<FontInfo>
  otfToTtf(input: Uint8Array, options: Otf2TtfOptions): Promise<Uint8Array>
  subsetTtf(input: Uint8Array, options: SubsetOptions): Promise<Uint8Array>
  svgFontToTtf(input: string, options: Svg2TtfOptions): Promise<Uint8Array>
  svgsToTtf(inputs: SvgIcon[], options: Svgs2TtfOptions): Promise<Uint8Array>
  ttfToEot(input: Uint8Array, options: Ttf2EotOptions): Promise<Uint8Array>
  ttfToSvg(input: Uint8Array, options: Ttf2SvgOptions): Promise<string>
  ttfToWoff(input: Uint8Array, options: WoffOptions): Promise<Uint8Array>
  ttfToWoff2(input: Uint8Array, options: Ttf2Woff2Options): Promise<Uint8Array>
}

export interface RuntimeSelector {
  readonly requested: RuntimeMode
  resolve(): Promise<OptimizeRuntime>
}

interface RuntimeLoaders {
  loadNative(): OptimizeRuntime
  loadWasm(): Promise<OptimizeRuntime>
}

export function resolvePipelineRuntimeMode(
  configured: RuntimeMode | undefined,
  fallbacks: readonly NonNullable<Ttf2Woff2Options['fallback']>[],
): RuntimeMode {
  if (fallbacks.includes('js')) throw new Error('WOFF2 fallback `js` is not available in this build')
  const legacy = [...new Set(fallbacks)]
  if (legacy.length > 1) throw new Error(`conflicting WOFF2 fallback modes: ${legacy.join(', ')}`)
  const fallback = legacy[0]
  if (configured !== undefined && fallback !== undefined && configured !== fallback) {
    throw new Error(`runtime \`${configured}\` conflicts with WOFF2 fallback \`${fallback}\``)
  }
  return configured ?? fallback ?? 'native'
}

export function createRuntimeSelector(
  requested: RuntimeMode,
  loaders: RuntimeLoaders = defaultRuntimeLoaders,
): RuntimeSelector {
  let selected: Promise<OptimizeRuntime> | undefined

  return {
    requested,
    resolve() {
      selected ??= selectRuntime(requested, loaders)
      return selected
    },
  }
}

async function selectRuntime(
  requested: RuntimeMode,
  loaders: RuntimeLoaders,
): Promise<OptimizeRuntime> {
  if (requested === 'native') return loaders.loadNative()
  if (requested === 'wasm') return loaders.loadWasm()
  try {
    return loaders.loadNative()
  } catch (error) {
    if (!(error instanceof NativeBindingLoadError)) throw error
    return loaders.loadWasm()
  }
}
```

Define `defaultRuntimeLoaders` after the concrete adapters in Step 4 so the
default parameter uses production loaders while tests can inject deterministic
loaders.

- [ ] **Step 4: Expand the WASM loader and create both adapters**

Expand the internal `WasmRuntime` interface in `wasm-fallback.ts` to include
the operations already exported by `@fontmin-rs/wasm`. Change loader failures
from the WOFF2-specific prefix to:

```ts
throw new Error('fontmin-rs WASM runtime failed to initialize', { cause })
```

In `optimize-runtime.ts`, implement the native adapter by wrapping existing
sync helpers in `async` functions. Implement the WASM adapter by awaiting
`loadWasmRuntime()` and forwarding matching options. Convert no options by
omission: validate and throw `fontmin-rs WASM <operation> does not support option <name>`
before a call whenever a non-`undefined` native-only option is present.

- [ ] **Step 5: Run selector tests and typecheck to verify GREEN**

Run: `rtk pnpm --filter fontmin-rs test -- optimize-runtime.test.ts`

Expected: PASS, 5 tests.

Run: `pnpm --filter fontmin-rs typecheck`

Expected: exit `0`.

- [ ] **Step 6: Commit the runtime contract**

```bash
rtk git add packages/fontmin/src/types.ts packages/fontmin/src/wasm-fallback.ts packages/fontmin/src/optimize-runtime.ts packages/fontmin/test/optimize-runtime.test.ts
rtk git commit -m "feat: add optimize runtime selector"
```

### Task 2: Route Every Built-in Optimize Operation Through the Runtime

**Files:**
- Modify: `packages/fontmin/src/optimize.ts`
- Test: `packages/fontmin/test/api.test.ts`

**Interfaces:**
- Consumes: `createRuntimeSelector(mode)` and `OptimizeRuntime` from Task 1.
- Produces: runtime-aware asynchronous built-in transforms with unchanged asset ordering.

- [ ] **Step 1: Add a failing complete WASM pipeline test**

Add to `packages/fontmin/test/api.test.ts`:

```ts
it('runs the complete file optimize pipeline through WASM', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-wasm-optimize-'))

  try {
    const files = await optimize({
      input: [fixture],
      outDir: outputDir,
      runtime: 'wasm',
      plugins: modernWeb({
        clone: false,
        fontFamily: 'Roboto WASM',
        fontPath: './',
        text: 'Hello',
      }),
    })

    const woff = files.find(file => file.format === 'woff')
    const woff2 = files.find(file => file.format === 'woff2')
    const cssAsset = files.find(file => file.format === 'css')

    expect(Buffer.from(woff?.contents ?? []).subarray(0, 4).toString('ascii')).toBe('wOFF')
    expect(Buffer.from(woff2?.contents ?? []).subarray(0, 4).toString('ascii')).toBe('wOF2')
    expect(new TextDecoder().decode(cssAsset?.contents)).toContain("font-family: 'Roboto WASM';")
    expect(readFileSync(resolve(outputDir, 'roboto-regular.woff2')).subarray(0, 4).toString()).toBe('wOF2')
  } finally {
    rmSync(outputDir, { force: true, recursive: true })
  }
})
```

Also add a custom plugin before `modernWeb()` that records its transform and
emits a text asset, then assert both the hook and emitted asset remain present.

- [ ] **Step 2: Run the focused test and verify RED**

Run: `rtk pnpm --filter fontmin-rs test -- api.test.ts -t "complete file optimize pipeline through WASM"`

Expected: FAIL because `runtime` is not used and the synchronous built-ins
still call native helpers.

- [ ] **Step 3: Resolve one selector per optimize call**

At the start of `optimize()`:

```ts
const plugins = sortPlugins(
  await resolvePluginTextFiles(pluginsFromConfig(config), cwd),
)
const legacyFallbacks = woff2FallbacksFromPlugins(plugins)
const runtimeMode = resolvePipelineRuntimeMode(config.runtime, legacyFallbacks)
const runtime = createRuntimeSelector(runtimeMode)
```

Pass `runtime` to `transformAssets()` and `runCss()`. Do not expose the selector
through `PluginContext`.

- [ ] **Step 4: Make built-in transforms asynchronous without reordering assets**

Replace each built-in `flatMap()` path with an ordered helper:

```ts
async function flatMapAssets(
  assets: FontAsset[],
  transform: (asset: FontAsset) => Promise<FontAsset[]>,
): Promise<FontAsset[]> {
  const transformed: FontAsset[] = []

  for (const asset of assets) {
    transformed.push(...(await transform(asset)))
  }

  return transformed
}
```

Change `runGlyph`, `runUnicodeSlices`, `runOtf2Ttf`, `runTtf2Woff`,
`runTtf2Woff2`, `runTtf2Eot`, `runTtf2Svg`, `runSvg2Ttf`, and `runSvgs2Ttf`
to accept `runtime: OptimizeRuntime`, await the matching adapter method, and
wrap binary results with `Buffer.from()`. Preserve clone rules, paths, formats,
source formats, metadata, and array ordering exactly.

Change `runCss()` to await `runtime.inspect()` for function-valued font family
and `runtime.generateFontFaceCss()`. Pass a resolved string rather than the
function to the adapter.

- [ ] **Step 5: Verify the WASM pipeline and native regressions are GREEN**

Run: `rtk pnpm --filter fontmin-rs test -- api.test.ts -t "WASM|optimized pipeline|custom plugin"`

Expected: all selected tests pass.

Run: `pnpm --filter fontmin-rs typecheck`

Expected: exit `0`.

- [ ] **Step 6: Commit asynchronous built-ins**

```bash
rtk git add packages/fontmin/src/optimize.ts packages/fontmin/test/api.test.ts
rtk git commit -m "feat: run file optimize through wasm"
```

### Task 3: Make Fallback and Cache Identity Runtime-Aware

**Files:**
- Modify: `packages/fontmin/src/optimize.ts`
- Modify: `scripts/package-smoke.mjs`
- Test: `packages/fontmin/test/api.test.ts`
- Test: `scripts/package-smoke.test.mjs`

**Interfaces:**
- Consumes: resolved runtime selector from Tasks 1–2.
- Produces: cache manifests separated by requested and resolved runtime and an isolated auto-fallback package smoke path.

- [ ] **Step 1: Write failing cache and compatibility tests**

Add tests asserting:

```ts
await optimize({ cache: { dir: cacheDir, enabled: true }, input: [fixture], runtime: 'native', plugins: modernWeb() })
await optimize({ cache: { dir: cacheDir, enabled: true }, input: [fixture], runtime: 'wasm', plugins: modernWeb() })
const index = JSON.parse(readFileSync(resolve(cacheDir, 'v1', 'index.json'), 'utf8'))
expect(Object.keys(index.entries)).toHaveLength(2)
const manifests = Object.keys(index.entries).map(key => JSON.parse(readFileSync(
  resolve(cacheDir, 'v1', key.slice(0, 2), key.slice(2, 4), key, 'index.json'),
  'utf8',
)))
expect(manifests.map(manifest => manifest.runtime.resolved).sort()).toStrictEqual(['native', 'wasm'])
```

Add tests that an omitted runtime plus `ttf2woff2({ fallback: 'wasm' })`
produces `wOF2`, and that explicit `runtime: 'native'` plus the same plugin
rejects with `runtime \`native\` conflicts with WOFF2 fallback \`wasm\``.

- [ ] **Step 2: Run focused tests and verify RED**

Run: `rtk pnpm --filter fontmin-rs test -- api.test.ts -t "runtime-specific cache|conflicts with WOFF2|legacy WOFF2"`

Expected: the manifest assertion fails because cached entries do not record
their requested and resolved runtime identity.

- [ ] **Step 3: Include requested and resolved runtime in cache keys**

When caching a pipeline with built-ins, resolve the selector before computing
the key and add:

```ts
runtime: {
  requested: runtime.requested,
  resolved: (await runtime.resolve()).kind,
},
```

to the canonical cache-key object. Pipelines containing only custom plugins do
not initialize a runtime and use `{ requested, resolved: null }`.

Add the same runtime object to `CacheManifest`, write it beside the key, and
require an exact identity match when reading a cached entry. This makes the
selected runtime auditable and prevents a manually copied manifest from being
accepted under the wrong runtime.

- [ ] **Step 4: Add isolated auto-fallback package smoke coverage**

Extend `scripts/package-smoke.mjs` with a consumer installation that installs
only the main and WASM tarballs, copies the TTF fixture, and executes:

```js
import { modernWeb, optimize } from 'fontmin-rs'
const assets = await optimize({
  input: ['./roboto.ttf'],
  runtime: 'auto',
  plugins: modernWeb({ clone: false, text: 'Hello' }),
})
if (!assets.some(asset => Buffer.from(asset.contents).subarray(0, 4).toString('ascii') === 'wOF2')) {
  throw new Error('auto optimize did not use WASM without a native artifact')
}
```

Do not install the binding tarball in this consumer. Update the script unit
test to assert that the isolated source contains `runtime: 'auto'` and omits
the native tarball.

- [ ] **Step 5: Verify focused and package smoke tests are GREEN**

Run: `rtk pnpm --filter fontmin-rs test -- api.test.ts`

Expected: PASS.

Run: `rtk node --test scripts/package-smoke.test.mjs`

Expected: PASS.

Run: `rtk pnpm run package:smoke`

Expected: exit `0`, including the no-native auto consumer.

- [ ] **Step 6: Commit fallback and cache behavior**

```bash
rtk git add packages/fontmin/src/optimize.ts packages/fontmin/test/api.test.ts scripts/package-smoke.mjs scripts/package-smoke.test.mjs
rtk git commit -m "test: cover optimize runtime fallback"
```

### Task 4: Document the Pipeline Runtime and Verify the Package

**Files:**
- Modify: `docs/api/node.md`
- Modify: `docs/guide/config.md`
- Modify: `docs/guide/migration.md`
- Modify: `docs/zh/api/node.md`
- Modify: `docs/zh/guide/config.md`
- Modify: `docs/zh/guide/migration.md`

**Interfaces:**
- Documents: exact runtime modes, legacy fallback rules, Node/WASM boundary, and error behavior.

- [ ] **Step 1: Replace the native-only documentation contract**

Document this exact example in both API languages:

```ts
await optimize({
  input: ['fonts/*.ttf'],
  outDir: 'build',
  runtime: 'auto',
  plugins: modernWeb({ text: 'Hello' }),
})
```

State that `native` is the default, `wasm` forces every built-in operation to
WASM, `auto` selects one runtime and falls back only when the native binding
cannot load, custom JavaScript plugins remain in Node, and conversion errors
never trigger fallback. Document the exact compatibility matrix for plugin
`fallback` values.

- [ ] **Step 2: Run documentation and package verification**

Run: `rtk pnpm --filter fontmin-rs test`

Expected: PASS.

Run: `pnpm --filter fontmin-rs typecheck`

Expected: exit `0`.

Run: `rtk pnpm run docs:check`

Expected: exit `0`.

- [ ] **Step 3: Commit the runtime documentation**

```bash
rtk git add docs/api/node.md docs/guide/config.md docs/guide/migration.md docs/zh/api/node.md docs/zh/guide/config.md docs/zh/guide/migration.md
rtk git commit -m "docs: document optimize runtime selection"
```

- [ ] **Step 4: Run the feature-wide non-publishing gate**

Run: `rtk pnpm run check`

Expected: exit `0` with no test, type, lint, format, or docs failures.
