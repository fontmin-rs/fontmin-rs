# Playground File-Desk Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the VitePress font Playground as a full-width, theme-aware, single-column file tool with browser-wide file drop and individual output downloads.

**Architecture:** The VitePress page mounts a client-only composition component on a header-width content rail. A small form component emits user intent, the existing playground composable owns processing state, and a result-ledger component emits both archive and single-asset download requests. UnoCSS generates scoped utility classes while VitePress CSS variables supply all light/dark colors.

**Tech Stack:** VitePress 2, Vue 3, TypeScript, UnoCSS with Wind3 and Iconify Lucide icons, VueUse (`useFileDialog`, `useDropZone`), `fflate`, `tinysaver`, Vitest, Vue Test Utils, Playwright CLI.

## Global Constraints

- Keep all font processing in the browser; no file upload or server API.
- Accept exactly one TTF, WOFF, WOFF2, EOT, OTF, or SVG Font input.
- Keep character text mandatory and reject CSS as the only selected output.
- Default selected outputs remain WOFF2, WOFF, and CSS.
- Retain `/playground` and `/zh/playground`, with localized visible copy and accessible icon labels.
- Use the existing VitePress theme switch; do not add a Playground theme control or hard-coded light/dark palette.
- Align Playground content with `--vp-layout-max-width`; preserve the frontmatter that hides sidebar, aside, and outline.
- Use `fflate` only for ZIP creation and `tinysaver` for all download blobs.
- Preserve unrelated working-tree changes in `.oxlintrc.jsonc` and `.vscode/settings.json`.
- Prefix shell commands with `rtk` except `pnpm typecheck`.

---

## File structure

- Create: `docs/uno.config.ts` — Wind3 and Lucide Iconify presets only.
- Create: `docs/.vitepress/theme/index.ts` — extends VitePress default theme and imports generated UnoCSS.
- Modify: `docs/package.json`, `pnpm-lock.yaml`, `docs/.vitepress/config.ts` — install and register the docs-only visual toolchain.
- Modify: `docs/.vitepress/playground/archive.ts` — ZIP and individual asset download adapters.
- Modify: `docs/.vitepress/playground/archive.test.ts` — verifies both download adapters.
- Modify: `docs/.vitepress/playground/font.ts` — exposes supported-file validation shared by form selection.
- Modify: `docs/.vitepress/playground/useFontPlayground.ts` — keeps selection validation and exposes per-asset download.
- Create: `docs/.vitepress/playground/useFontPlayground.test.ts` — validates retained selection and download intent.
- Create: `docs/.vitepress/components/PlaygroundForm.vue` — four sequential form rows.
- Rewrite: `docs/.vitepress/components/PlaygroundResults.vue` — output ledger and download controls.
- Rewrite: `docs/.vitepress/components/FontPlayground.vue` — localized composition, `useFileDialog`, browser-wide `useDropZone`, and drop overlay.
- Modify: `docs/.vitepress/components/FontPlayground.test.ts` — interaction, failure, individual download, and drop/picker coverage.
- Remove: `docs/.vitepress/components/PlaygroundUpload.vue`, `docs/.vitepress/components/PlaygroundOptions.vue` — replaced by the sequential form component.
- Modify: `docs/playground.md`, `docs/zh/playground.md` — page shells retain frontmatter and only mount the component.

### Task 1: Add docs-only UnoCSS, Iconify, and VueUse integration

**Files:**
- Modify: `docs/package.json`
- Modify: `pnpm-lock.yaml`
- Create: `docs/uno.config.ts`
- Create: `docs/.vitepress/theme/index.ts`
- Modify: `docs/.vitepress/config.ts`

**Interfaces:**
- Produces `virtual:uno.css` for every documentation route.
- Makes static utility classes such as `i-lucide-download`, `bg-[var(--vp-c-bg)]`, and responsive Wind3 utilities available in Vue SFCs.

- [ ] **Step 1: Add a failing theme entry that requires generated UnoCSS**

Create `docs/.vitepress/theme/index.ts`:

```ts
import DefaultTheme from 'vitepress/theme'
import 'virtual:uno.css'

export default DefaultTheme
```

Run: `rtk pnpm -C docs run build`

Expected: FAIL with a Vite resolution error for `virtual:uno.css`, proving the stylesheet is not silently missing.

- [ ] **Step 2: Add the required packages**

Run:

```sh
rtk pnpm -C docs add @vueuse/core
rtk pnpm -C docs add -D unocss @unocss/preset-icons @unocss/preset-wind3 @iconify-json/lucide
```

Expected: `docs/package.json` records `@vueuse/core` under dependencies and the UnoCSS/Iconify packages under devDependencies; `pnpm-lock.yaml` changes only for these packages and their transitive dependencies.

- [ ] **Step 3: Configure UnoCSS and VitePress**

Create `docs/uno.config.ts`:

```ts
import { defineConfig, presetIcons, presetWind3 } from 'unocss'

export default defineConfig({
  preflights: [],
  presets: [
    presetWind3(),
    presetIcons({
      collections: {
        lucide: () => import('@iconify-json/lucide/icons.json').then(i => i.default),
      },
    }),
  ],
})
```

Update the existing `vite` object in `docs/.vitepress/config.ts`:

```ts
import UnoCSS from 'unocss/vite'

vite: {
  plugins: [UnoCSS()],
  resolve: {
    // retain the existing @fontmin-rs/wasm alias unchanged
  },
},
```

Do not add a global CSS theme or a `dark:` palette. Components use VitePress variables through static Uno arbitrary-value classes.

- [ ] **Step 4: Verify production CSS generation**

Run: `rtk pnpm -C docs run build`

Expected: PASS; the generated docs bundle contains the UnoCSS virtual stylesheet and no unresolved Iconify or virtual-module imports.

- [ ] **Step 5: Commit the toolchain setup**

```sh
rtk git add docs/package.json pnpm-lock.yaml docs/uno.config.ts docs/.vitepress/theme/index.ts docs/.vitepress/config.ts
rtk git commit -m "feat: add playground styling toolchain"
```

### Task 2: Add individual asset downloads and selection guards

**Files:**
- Modify: `docs/.vitepress/playground/archive.ts`
- Modify: `docs/.vitepress/playground/archive.test.ts`
- Modify: `docs/.vitepress/playground/font.ts`
- Modify: `docs/.vitepress/playground/font.test.ts`
- Modify: `docs/.vitepress/playground/useFontPlayground.ts`
- Create: `docs/.vitepress/playground/useFontPlayground.test.ts`

**Interfaces:**
- Produces `downloadAsset(asset: PlaygroundAsset): void` from `archive.ts`.
- Produces `isSupportedInputFile(fileName: string): boolean` from `font.ts`.
- `useFontPlayground()` exposes `downloadAsset(asset)`, and `selectFile(file)` retains a previously valid file when the new file is unsupported.

- [ ] **Step 1: Write failing download and selection tests**

Add this case to `docs/.vitepress/playground/archive.test.ts`:

```ts
it('downloads one asset with its original name', () => {
  downloadAsset(assets[0])

  expect(saveAs).toHaveBeenCalledWith(expect.any(Blob), 'demo.woff2')
})
```

Create `docs/.vitepress/playground/useFontPlayground.test.ts` with a mocked `processFont` and archive helpers:

```ts
it('keeps a valid selection when an unsupported file is chosen', () => {
  const playground = useFontPlayground()
  const valid = new File([new Uint8Array([1])], 'demo.ttf')

  playground.selectFile(valid)
  playground.selectFile(new File([new Uint8Array([1])], 'demo.bin'))

  expect(playground.selectedFile.value).toBe(valid)
  expect(playground.error.value).toBe('Unsupported input format: .bin.')
})

it('delegates one generated asset to the archive helper', () => {
  const playground = useFontPlayground()
  const asset = { contents: new Uint8Array([1]), fileName: 'demo.woff2', format: 'woff2' as const }

  playground.downloadAsset(asset)
  expect(downloadAsset).toHaveBeenCalledWith(asset)
})
```

Run: `rtk pnpm -C docs test -- archive.test.ts useFontPlayground.test.ts`

Expected: FAIL because `downloadAsset`, `isSupportedInputFile`, and the composable action do not exist.

- [ ] **Step 2: Implement the narrow archive and validation APIs**

Add to `docs/.vitepress/playground/archive.ts`:

```ts
export function downloadAsset(asset: PlaygroundAsset): void {
  saveAs(
    new Blob([asset.contents], { type: asset.format === 'css' ? 'text/css;charset=utf-8' : 'font/*' }),
    asset.fileName,
  )
}
```

Add to `docs/.vitepress/playground/font.ts` beside `detectInputFormat`:

```ts
export function isSupportedInputFile(fileName: string): boolean {
  try {
    detectInputFormat(fileName)
    return true
  } catch {
    return false
  }
}
```

In `useFontPlayground.ts`, import both helpers. Change selection and add the
action:

```ts
function selectFile(file: File): void {
  if (!isSupportedInputFile(file.name)) {
    error.value = `Unsupported input format: .${file.name.split('.').pop() ?? ''}.`
    return
  }
  selectedFile.value = file
  error.value = ''
}

function downloadAsset(asset: PlaygroundAsset): void {
  downloadPlaygroundAsset(asset)
}
```

Alias the imported archive helper as `downloadPlaygroundAsset` to avoid a local
name collision. Return `downloadAsset` from the composable.

- [ ] **Step 3: Run focused tests**

Run: `rtk pnpm -C docs test -- archive.test.ts font.test.ts useFontPlayground.test.ts`

Expected: PASS; existing archive/processor tests and the new guard/download cases pass.

- [ ] **Step 4: Commit behavior primitives**

```sh
rtk git add docs/.vitepress/playground/archive.ts docs/.vitepress/playground/archive.test.ts docs/.vitepress/playground/font.ts docs/.vitepress/playground/font.test.ts docs/.vitepress/playground/useFontPlayground.ts docs/.vitepress/playground/useFontPlayground.test.ts
rtk git commit -m "feat: add playground asset downloads"
```

### Task 3: Build the sequential form and output ledger components

**Files:**
- Create: `docs/.vitepress/components/PlaygroundForm.vue`
- Modify: `docs/.vitepress/components/PlaygroundResults.vue`
- Modify: `docs/.vitepress/playground/types.ts`
- Test: `docs/.vitepress/components/FontPlayground.test.ts`

**Interfaces:**
- `PlaygroundForm` consumes `PlaygroundCopy`, selected file, character text,
  formats, count, and generation state; emits `open-file-dialog`,
  `update-characters`, `update-format`, and `generate`.
- `PlaygroundResults` consumes assets/status/copy and emits `download-archive`
  and `download-asset` with a `PlaygroundAsset` payload.

- [ ] **Step 1: Expand the localized copy contract and add failing UI assertions**

Add these fields to `PlaygroundCopy` in `types.ts`:

```ts
changeFile: string
downloadAsset: string
downloadZip: string
fontFile: string
dropFile: string
```

Update `FontPlayground.test.ts` so the expected generated result verifies both
download paths:

```ts
expect(wrapper.get('[data-testid="download-asset-demo.woff2"]').attributes('aria-label')).toContain('demo.woff2')
await wrapper.get('[data-testid="download-asset-demo.woff2"]').trigger('click')
expect(downloadAsset).toHaveBeenCalledWith(expect.objectContaining({ fileName: 'demo.woff2' }))

await wrapper.get('[data-testid="download-archive"]').trigger('click')
expect(downloadArchive).toHaveBeenCalledWith(expect.any(Array), 'demo-fontmin.zip')
```

Add form assertions for exactly four `data-testid="playground-row"` rows,
including a textarea and checkbox group. Run:

`rtk pnpm -C docs test -- FontPlayground.test.ts`

Expected: FAIL because the old card components do not expose the linear form or
individual download action.

- [ ] **Step 2: Implement `PlaygroundForm.vue` with static Uno utilities**

Use typed props/emits and this row skeleton; repeat it for file, characters,
formats, and actions:

```vue
<div
  data-testid="playground-row"
  class="grid gap-2 border-b border-[var(--vp-c-border)] px-4 py-4 md:grid-cols-[9rem_minmax(0,1fr)] md:items-start"
>
  <label class="pt-2 text-xs font-semibold tracking-wide text-[var(--vp-c-text-2)] uppercase">
    {{ copy.fontFile }}
  </label>
  <!-- row control -->
</div>
```

The file control is a button emitting `open-file-dialog`, with
`i-lucide-file-type-2` and a `i-lucide-refresh-cw` change indicator. The
textarea emits the input string. Render checkbox labels from a local typed
format list. The action row emits `generate`, disables from the `canGenerate`
prop, and uses `i-lucide-sparkles` only as a decorative icon. Reuse VitePress
variables for every color, surface, focus ring, and disabled state.

- [ ] **Step 3: Rewrite the results component as a ledger**

Render status/errors above the list. For every asset use this button shape:

```vue
<button
  :aria-label="`${copy.downloadAsset}: ${asset.fileName}`"
  :data-testid="`download-asset-${asset.fileName}`"
  class="grid size-8 place-items-center rounded-md text-[var(--vp-c-brand-1)] hover:bg-[var(--vp-c-default-soft)] focus-visible:outline-2 focus-visible:outline-[var(--vp-c-brand-1)]"
  type="button"
  @click="emit('download-asset', asset)"
>
  <span class="i-lucide-download size-4" aria-hidden="true" />
</button>
```

Use a matching `i-lucide-package-down` ZIP button with
`data-testid="download-archive"`. Rows have columns for format, flexible
ellipsis filename, tabular byte size, and action. Keep `role="status"` and
`role="alert"` behavior intact.

- [ ] **Step 4: Run form and ledger tests while legacy imports remain intact**

Keep `PlaygroundUpload.vue` and `PlaygroundOptions.vue` until Task 4 replaces
their imports in the composition component. Run:

`rtk pnpm -C docs test -- FontPlayground.test.ts`

Expected: PASS; the component observes four sequential rows and both download
actions.

- [ ] **Step 5: Commit form and ledger components**

```sh
rtk git add docs/.vitepress/components/PlaygroundForm.vue docs/.vitepress/components/PlaygroundResults.vue docs/.vitepress/components/FontPlayground.test.ts docs/.vitepress/playground/types.ts
rtk git commit -m "feat: redesign playground form"
```

### Task 4: Compose picker, browser-wide drop, localized copy, and header-width layout

**Files:**
- Modify: `docs/.vitepress/components/FontPlayground.vue`
- Modify: `docs/.vitepress/components/FontPlayground.test.ts`
- Modify: `docs/playground.md`
- Modify: `docs/zh/playground.md`
- Remove: `docs/.vitepress/components/PlaygroundUpload.vue`
- Remove: `docs/.vitepress/components/PlaygroundOptions.vue`

**Interfaces:**
- Uses `useFileDialog` with the six supported extensions and a single file.
- Uses `useDropZone(document, ...)` and passes valid dropped files through the
  same `selectFile` state action as the picker.
- Composes `PlaygroundForm` and `PlaygroundResults` with props-down/events-up.

- [ ] **Step 1: Write failing picker and document-drop tests**

Mock VueUse in `FontPlayground.test.ts`:

```ts
const { open, onChange, onDrop } = vi.hoisted(() => ({
  onChange: vi.fn(),
  onDrop: vi.fn(),
  open: vi.fn(),
}))

vi.mock('@vueuse/core', () => ({
  useDropZone: (_target: Document, options: { onDrop: typeof onDrop }) => {
    onDrop.mockImplementation(options.onDrop)
    return { isOverDropZone: ref(false) }
  },
  useFileDialog: () => ({ files: ref(null), onChange, open, reset: vi.fn() }),
}))
```

Assert the file row calls `open`, and invoke the captured drop handler with
`[new File([new Uint8Array([1])], 'dropped.woff2')]`; then assert the file name
appears in the form. Assert `data-testid="drop-overlay"` is absent without an
active drag and shown when the mocked `isOverDropZone` is set to true.

Run: `rtk pnpm -C docs test -- FontPlayground.test.ts`

Expected: FAIL because the current component has no VueUse picker, global drop
handler, overlay, or rewritten event wiring.

- [ ] **Step 2: Replace the composition component**

Create the picker once in `FontPlayground.vue`:

```ts
const acceptedFonts = '.ttf,.woff,.woff2,.eot,.otf,.svg'
const { onChange, open } = useFileDialog({
  accept: acceptedFonts,
  multiple: false,
  reset: true,
})

onChange(files => {
  const file = files?.item(0)
  if (file) playground.selectFile(file)
})

const { isOverDropZone } = useDropZone(document, {
  multiple: false,
  preventDefaultForUnhandled: true,
  onDrop(files) {
    const file = files?.[0]
    if (file) playground.selectFile(file)
  },
})
```

Use the composable’s existing phase/error state, English and Chinese copies,
and no duplicated state. Add the full-width outer rail:

```vue
<section class="mx-auto w-full max-w-[var(--vp-layout-max-width)] px-5 py-10 sm:px-8 lg:px-12">
  <!-- File desk header, one form surface, and result ledger -->
</section>
```

Render a fixed `v-if="isOverDropZone"` overlay with
`data-testid="drop-overlay"`, `pointer-events-none`, `i-lucide-file-down`, and
localized `copy.dropFile`. It is visual feedback only. Use `open()` for the
form event; wire result archive/asset events to the composable’s two download
methods.

- [ ] **Step 3: Simplify both route shells**

Leave existing `layout: page`, `outline: false`, `aside: false`, `sidebar:
false`, and `pageClass: page-playground` frontmatter untouched. Remove each
Markdown title and introductory paragraph so only the `<script setup>` import
and `<ClientOnly><FontPlayground /></ClientOnly>` remain. This avoids a second,
narrow Markdown rail above the component.

After `FontPlayground.vue` imports `PlaygroundForm.vue` instead of the two
legacy components, remove the obsolete files:

```sh
rtk git rm docs/.vitepress/components/PlaygroundUpload.vue docs/.vitepress/components/PlaygroundOptions.vue
```

- [ ] **Step 4: Run component tests and docs build**

Run:

```sh
rtk pnpm -C docs test
rtk pnpm -C docs run build
```

Expected: PASS; all Playground tests pass and both routes render with the
custom theme entry and no unresolved VueUse, UnoCSS, or Iconify modules.

- [ ] **Step 5: Commit interaction and route integration**

```sh
rtk git add docs/.vitepress/components/FontPlayground.vue docs/.vitepress/components/FontPlayground.test.ts docs/playground.md docs/zh/playground.md
rtk git add -u docs/.vitepress/components
rtk git commit -m "feat: add playground file interactions"
```

### Task 5: Verify the complete visual and download workflow

**Files:**
- Verify only; do not add generated browser artifacts to Git.

**Interfaces:**
- Proves the live VitePress page executes the browser WASM flow and all user
  download paths after the redesign.

- [ ] **Step 1: Run fresh project checks**

Run:

```sh
rtk pnpm -C docs test
rtk pnpm -C docs run build
rtk pnpm run check
rtk git diff --check
```

Expected: every command exits 0. Existing non-fatal workspace lint warnings may
remain, but no command may produce an error.

- [ ] **Step 2: Verify desktop light and dark layouts in a real browser**

Start VitePress:

```sh
rtk pnpm -C docs dev --host 127.0.0.1
```

Use `/Users/ntnyq/.codex/skills/playwright/scripts/playwright_cli.sh` to open
`/playground` with a desktop viewport. Snapshot the page and confirm there is no
sidebar, outline, or pager; the main rail is as wide as the nav content; form
rows are sequential; and the header’s VitePress theme control changes the
Playground’s background, borders, and text tokens when toggled to dark.

- [ ] **Step 3: Verify file flow, individual download, ZIP, and drag overlay**

With the same browser session, upload
`fixtures/fonts/ttf/roboto-regular.ttf`, fill `Hello 浏览器`, and generate the
default outputs. Assert WOFF2, WOFF, and CSS result rows display nonzero sizes.
Download one WOFF2 asset and the ZIP, then inspect the ZIP with:

```sh
rtk unzip -l <downloaded-zip-path>
```

Expected: the archive lists the same generated assets. Trigger a document drag
with a font `DataTransfer` in the browser and confirm the drop overlay becomes
visible, then drop the font and confirm the selected filename updates. Repeat a
snapshot at a narrow viewport to confirm labels stack above controls and all
download controls remain visible.

- [ ] **Step 4: Clean verification artifacts and commit any intentional final fixes**

Stop the dev server. Remove only tool-created `.playwright-cli/` and
`output/playwright/` artifacts if created. Re-run `rtk git status --short`; do
not alter `.oxlintrc.jsonc` or `.vscode/settings.json`. If visual verification
requires no source change, make no extra commit; otherwise commit the minimal
fix with `fix: polish playground file desk` and repeat Steps 1–3.
