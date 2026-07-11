# Browser Font Playground Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a VitePress playground that subsets one local font in the browser and downloads selected outputs as a ZIP.

**Architecture:** A client-only Vue component delegates font normalization and transformations to a pure playground processing module backed by `@fontmin-rs/wasm`. A separate archive module creates ZIP bytes with `fflate` and triggers the download through `tinysaver`; pure modules and the component have focused Vitest coverage.

**Tech Stack:** VitePress, Vue 3, TypeScript, `@fontmin-rs/wasm`, `fflate` 0.8.3, `tinysaver` 0.1.0, Vitest 4.1.10, Vue Test Utils 2.4.11, happy-dom 20.10.6.

## Global Constraints

- Run all font transformation code in the browser; never upload file bytes.
- Accept one TTF, WOFF, WOFF2, EOT, OTF, or SVG Font input.
- Require non-empty character text and at least one non-CSS output before generation.
- CSS requires at least one selected font output.
- Default outputs are WOFF2, WOFF, and CSS.
- Use `fflate` only for ZIP creation and `tinysaver` only for file download.
- Preserve VitePress visual variables and dark mode; do not introduce a separate design system.
- Prefix shell commands with `rtk` (except `pnpm typecheck`).

---

## File structure

- `docs/.vitepress/playground/types.ts`: shared input, output, progress, and asset types.
- `docs/.vitepress/playground/font.ts`: input normalization, validation, subsetting, conversion, and CSS generation.
- `docs/.vitepress/playground/archive.ts`: ZIP creation and `tinysaver` download adapter.
- `docs/.vitepress/components/FontPlayground.vue`: client UI state and VitePress-styled interaction.
- `docs/playground.md` and `docs/zh/playground.md`: English and Chinese route shells that mount the client component.
- `docs/.vitepress/playground/*.test.ts`: focused unit tests for the processor and ZIP helper.
- `docs/.vitepress/components/FontPlayground.test.ts`: component validation and result-state tests.
- `docs/vitest.config.ts`: happy-dom Vitest configuration for the docs package.

### Task 1: Add the docs package dependencies, test runner, routes, and navigation

**Files:**
- Modify: `docs/package.json`
- Modify: `pnpm-lock.yaml`
- Modify: `docs/.vitepress/config.ts`
- Create: `docs/vitest.config.ts`
- Create: `docs/playground.md`
- Create: `docs/zh/playground.md`

**Interfaces:**
- Produces the `/playground` and `/zh/playground` VitePress routes.
- Provides `pnpm -C docs test` for later playground tests.

- [ ] **Step 1: Add the failing route-shell and navigation assertions by building docs before the routes exist**

Run: `rtk pnpm -C docs run build`

Expected: the current build passes but neither `/playground` nor `/zh/playground` is emitted; this establishes the missing route condition before adding it.

- [ ] **Step 2: Add the browser and test dependencies**

In `docs/package.json`, add these dependencies and scripts:

```json
{
  "scripts": {
    "build": "vitepress build .",
    "dev": "vitepress dev .",
    "test": "vitest --run"
  },
  "dependencies": {
    "@fontmin-rs/wasm": "workspace:*",
    "fflate": "^0.8.3",
    "tinysaver": "^0.1.0"
  },
  "devDependencies": {
    "@vue/test-utils": "^2.4.11",
    "happy-dom": "^20.10.6",
    "vitepress": "^2.0.0-alpha.18",
    "vitest": "catalog:"
  }
}
```

Run: `rtk pnpm install`

Expected: the lockfile contains the three runtime dependencies and two test-only dependencies.

- [ ] **Step 3: Add the docs Vitest configuration and page shells**

Create `docs/vitest.config.ts`:

```ts
import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    environment: 'happy-dom',
    include: ['.vitepress/**/*.test.ts'],
  },
})
```

Create each route shell with a localized title and local-processing intro copy:

```md
# Font Playground

Subset a local font and download selected web-font outputs as a ZIP. Your file is processed in this browser and is never uploaded.
```

Add `Playground` / `字体 Playground` to the respective VitePress top navigation.

- [ ] **Step 4: Build the route shells**

Run: `rtk pnpm -C docs run build`

Expected: PASS; VitePress renders both localized route shells.

- [ ] **Step 5: Commit the docs scaffold**

```bash
git add docs/package.json docs/.vitepress/config.ts docs/vitest.config.ts docs/playground.md docs/zh/playground.md pnpm-lock.yaml
git commit -m "feat: add font playground docs scaffold"
```

### Task 2: Implement and test pure font processing

**Files:**
- Create: `docs/.vitepress/playground/types.ts`
- Create: `docs/.vitepress/playground/font.ts`
- Test: `docs/.vitepress/playground/font.test.ts`

**Interfaces:**
- Consumes: `@fontmin-rs/wasm` direct helpers after `initWasm()`.
- Produces: `processFont(request): Promise<PlaygroundAsset[]>` for the Vue component.

- [ ] **Step 1: Write failing processor tests**

Create tests that inject a mocked WASM API and verify format normalization,
mandatory text, output selection, and CSS sources:

```ts
it('normalizes WOFF2, subsets it, and emits selected outputs', async () => {
  const wasm = fakeWasm()
  const outputs = await processFont({
    contents: new Uint8Array([1]),
    fileName: 'demo.woff2',
    text: 'Hello',
    formats: new Set(['woff2', 'css']),
  }, wasm)

  expect(wasm.woff2ToTtf).toHaveBeenCalledOnce()
  expect(wasm.subsetTtf).toHaveBeenCalledWith(expect.any(Uint8Array), { text: 'Hello' })
  expect(outputs.map(output => output.fileName)).toEqual(['demo.woff2', 'demo.css'])
})

it.each([
  ['', 'Enter at least one character to subset.'],
  ['   ', 'Enter at least one character to subset.'],
])('rejects missing character text', async (text, message) => {
  await expect(processFont({ contents, fileName: 'demo.ttf', text, formats: new Set(['woff2']) }, fakeWasm()))
    .rejects.toThrow(message)
})

it('rejects CSS as the only selected output', async () => {
  await expect(processFont({ contents, fileName: 'demo.ttf', text: 'A', formats: new Set(['css']) }, fakeWasm()))
    .rejects.toThrow('Select at least one font output when generating CSS.')
})
```

- [ ] **Step 2: Run the processor test to verify failure**

Run: `rtk pnpm -C docs test -- font.test.ts`

Expected: FAIL because `processFont` and its request types do not exist.

- [ ] **Step 3: Define the request and output interfaces**

Create `types.ts`:

```ts
export type PlaygroundFormat = 'css' | 'eot' | 'svg' | 'ttf' | 'woff' | 'woff2'

export interface PlaygroundAsset {
  contents: Uint8Array
  fileName: string
  format: PlaygroundFormat
}

export interface ProcessFontRequest {
  contents: Uint8Array
  fileName: string
  text: string
  formats: ReadonlySet<PlaygroundFormat>
}
```

In `font.ts`, export `detectInputFormat(fileName)`,
`validateRequest(request)`, and `processFont(request, wasm = browserWasm)`.
`detectInputFormat` accepts only `ttf`, `woff`, `woff2`, `eot`, `otf`, and
`svg`; all other extensions throw `Unsupported input format: .<extension>.`.

- [ ] **Step 4: Implement normalization and output conversion**

Use direct WASM helpers in this exact normalization switch:

```ts
const ttf = await normalizeToTtf(request.contents, detectInputFormat(request.fileName), wasm)
const subset = await wasm.subsetTtf(ttf, { text: request.text })
```

`normalizeToTtf` passes TTF through; uses `woffToTtf`, `woff2ToTtf`,
`eotToTtf`, `otfToTtf`, and `svgFontToTtf` for the other formats. Generate
selected font assets from `subset`, preserving the original stem. Generate CSS
last with `generateFontFaceCss(fontAssets, { fontFamily: stem, fontPath: './', local: false })`.

- [ ] **Step 5: Run the processor tests**

Run: `rtk pnpm -C docs test -- font.test.ts`

Expected: PASS. Tests cover all six accepted extensions, CSS-only rejection,
and CSS receiving selected generated font bytes.

- [ ] **Step 6: Commit the processor**

```bash
git add docs/.vitepress/playground/types.ts docs/.vitepress/playground/font.ts docs/.vitepress/playground/font.test.ts
git commit -m "feat: process fonts in playground"
```

### Task 3: Implement and test ZIP creation and download

**Files:**
- Create: `docs/.vitepress/playground/archive.ts`
- Test: `docs/.vitepress/playground/archive.test.ts`

**Interfaces:**
- Consumes: `PlaygroundAsset[]` from `processFont`.
- Produces: `createArchive(assets): Uint8Array` and `downloadArchive(assets, fileName): void`.

- [ ] **Step 1: Write the failing archive tests**

```ts
import { unzipSync, strFromU8 } from 'fflate'

it('creates a ZIP containing every generated asset', () => {
  const zip = createArchive([
    { contents: new Uint8Array([1]), fileName: 'demo.woff2', format: 'woff2' },
    { contents: new TextEncoder().encode('body {}'), fileName: 'demo.css', format: 'css' },
  ])

  const files = unzipSync(zip)
  expect(Object.keys(files)).toContain('demo.woff2')
  expect(strFromU8(files['demo.css'])).toBe('body {}')
})

it('passes a ZIP Blob and requested name to tinysaver', () => {
  downloadArchive(assets, 'demo-fontmin.zip')
  expect(saveAs).toHaveBeenCalledWith(expect.any(Blob), 'demo-fontmin.zip')
})
```

- [ ] **Step 2: Run the archive tests to verify failure**

Run: `rtk pnpm -C docs test -- archive.test.ts`

Expected: FAIL because the archive module does not exist.

- [ ] **Step 3: Implement ZIP and download adapters**

```ts
import { zipSync } from 'fflate'
import { saveAs } from 'tinysaver'

export function createArchive(assets: PlaygroundAsset[]): Uint8Array {
  return zipSync(Object.fromEntries(assets.map(asset => [asset.fileName, asset.contents])))
}

export function downloadArchive(assets: PlaygroundAsset[], fileName: string): void {
  saveAs(new Blob([createArchive(assets)], { type: 'application/zip' }), fileName)
}
```

- [ ] **Step 4: Run the archive tests**

Run: `rtk pnpm -C docs test -- archive.test.ts`

Expected: PASS; ZIP entry names, ZIP contents, MIME type, and `tinysaver`
call are verified.

- [ ] **Step 5: Commit the archive helper**

```bash
git add docs/.vitepress/playground/archive.ts docs/.vitepress/playground/archive.test.ts
git commit -m "feat: package playground outputs as zip"
```

### Task 4: Implement the VitePress-styled Playground component

**Files:**
- Create: `docs/.vitepress/components/FontPlayground.vue`
- Test: `docs/.vitepress/components/FontPlayground.test.ts`
- Modify: `docs/playground.md`
- Modify: `docs/zh/playground.md`

**Interfaces:**
- Consumes: `processFont`, `downloadArchive`, and `PlaygroundAsset`.
- Produces: an interactive component mounted by both localized page shells.

- [ ] **Step 1: Write failing component tests**

```ts
it('disables generation until a file, character text, and font output are selected', async () => {
  const wrapper = mount(FontPlayground)
  expect(wrapper.get('[data-testid="generate"]').attributes('disabled')).toBeDefined()

  await wrapper.get('textarea').setValue('Hello')
  expect(wrapper.get('[data-testid="generate"]').attributes('disabled')).toBeDefined()
})

it('shows generated assets and downloads their ZIP', async () => {
  const wrapper = mount(FontPlayground)
  await selectFile(wrapper, new File([new Uint8Array([1])], 'demo.ttf'))
  await wrapper.get('textarea').setValue('Hello')
  await wrapper.get('[data-testid="generate"]').trigger('click')

  expect(wrapper.text()).toContain('demo.woff2')
  expect(downloadArchive).not.toHaveBeenCalled()
  await wrapper.get('[data-testid="download"]').trigger('click')
  expect(downloadArchive).toHaveBeenCalledWith(expect.any(Array), 'demo-fontmin.zip')
})
```

- [ ] **Step 2: Run the component test to verify failure**

Run: `rtk pnpm -C docs test -- FontPlayground.test.ts`

Expected: FAIL because the component does not exist.

- [ ] **Step 3: Implement the component state and accessibility contract**

Use a hidden `<input type="file">` with a visible `<label>` drop target;
reject a second file. Add a labeled textarea, fieldset of format checkboxes,
live status text (`role="status"`), error panel (`role="alert"`), and
`data-testid` values from the test. Derive `canGenerate` from:

```ts
const canGenerate = computed(() =>
  selectedFile.value !== undefined &&
  characters.value.trim().length > 0 &&
  selectedFormats.value.size > 0 &&
  (selectedFormats.value.size !== 1 || !selectedFormats.value.has('css')) &&
  !isGenerating.value,
)
```

Set phases to `initializing`, `normalizing`, `subsetting`, `converting`,
`archiving`, `complete`, or `error`. Preserve the last successful asset list
when a later generation fails. Set `isGenerating` to `true` before WASM work
and back to `false` in `finally`, so a completed or failed run can be retried.

- [ ] **Step 4: Apply VitePress-native styling**

Use scoped CSS built on `--vp-c-bg-soft`, `--vp-c-bg-alt`, `--vp-c-border`,
`--vp-c-brand-1`, `--vp-c-text-1`, and `--vp-font-family-base`. Use a
responsive two-column form above a single-column result section; collapse to
one column below `640px`. Do not hard-code light-only colors.

- [ ] **Step 5: Add localized page copy**

Keep route shells localized: headings, local-processing copy, labels, helper
text, errors, and download button text are supplied to the component through
English and Chinese props. Both pages must say the source file is processed
locally and never uploaded.

- [ ] **Step 6: Run component tests**

Run: `rtk pnpm -C docs test -- FontPlayground.test.ts`

Expected: PASS; validation gates, success list, error rendering, and
`tinysaver` download invocation are covered.

- [ ] **Step 7: Commit the interactive UI**

```bash
git add docs/.vitepress/components/FontPlayground.vue docs/.vitepress/components/FontPlayground.test.ts docs/playground.md docs/zh/playground.md
git commit -m "feat: add browser font playground"
```

### Task 5: Verify documentation and full browser flow

**Files:**
- Modify: `docs/guide/getting-started.md`
- Modify: `docs/zh/guide/getting-started.md`

**Interfaces:**
- Consumes: completed `/playground` route and docs package test script.
- Produces: a documented, buildable, cross-browser playground entry point.

- [ ] **Step 1: Add a concise Playground link to both quick-start guides**

Add this sentence after the browser WASM example, with localized equivalent:

```md
Try the [Font Playground](/playground) to subset a local font and download selected web-font outputs as a ZIP.
```

- [ ] **Step 2: Run playground tests and docs build**

Run: `rtk pnpm -C docs test && rtk pnpm -C docs run build`

Expected: PASS; test suite verifies the processing, archive, and component
contracts, while VitePress renders both playground routes.

- [ ] **Step 3: Run the existing browser WASM acceptance matrix**

Run:

```bash
rtk pnpm --filter @fontmin-rs/wasm run build
rtk env BROWSER=chromium node wasm/fontmin/test/browser-runtime.mjs
rtk env BROWSER=firefox node wasm/fontmin/test/browser-runtime.mjs
rtk env BROWSER=webkit node wasm/fontmin/test/browser-runtime.mjs
```

Expected: PASS in all three engines; generated WOFF2 loads through
`FontFace`.

- [ ] **Step 4: Run the repository check**

Run: `rtk pnpm run check`

Expected: PASS. Existing warnings may remain, but the command exits zero.

- [ ] **Step 5: Commit verification and guide links**

```bash
git add docs/guide/getting-started.md docs/zh/guide/getting-started.md
git commit -m "docs: link font playground"
```
