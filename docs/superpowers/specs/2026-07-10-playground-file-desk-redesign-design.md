# Playground File-Desk Redesign

## Goal

Turn the browser font playground into a calm, full-width file tool: one linear
form, clear output records, per-file downloads, ZIP download, and browser-wide
font-file dropping. The existing VitePress theme switch controls all light and
dark appearance.

## Scope and constraints

- The Playground route remains client-only and processes all font bytes in the
  browser. No upload or server API is added.
- Support one input file in the existing six formats: TTF, WOFF, WOFF2, EOT,
  OTF, and SVG Font.
- Keep the current required-character rule and output-format validation.
- Keep the existing WOFF2, WOFF, and CSS default selection.
- Retain both `/playground` and `/zh/playground`; all new visible copy is
  localized.
- Preserve the user’s unrelated pending changes in `.oxlintrc.jsonc` and
  `.vscode/settings.json`.

## Visual direction

Use the selected **File desk** direction: an understated, single-column browser
tool rather than a dashboard. The page owns one wide content rail aligned to
VitePress’s header content width (`--vp-layout-max-width`) with responsive
inline padding. The Markdown route files contain only the client component so
the component’s heading and form share that rail.

The interface has a narrow metadata header and one bordered form surface. Thin
horizontal separators establish the input sequence; there are no competing
cards or two-column panels. The only strong accent is the primary Generate
button. Generated assets become compact ledger rows with tabular sizes and
small, explicit icon buttons.

All colors, borders, backgrounds, and text use VitePress CSS variables. This
makes the existing VitePress theme control immediately update the complete
Playground; no second theme toggle is created. Icons are monochrome Iconify
Lucide icons and inherit the surrounding VitePress token color.

## Page structure

1. A small `fontmin-rs / WASM` label, title, and local-processing statement.
2. **Font file** row: font-file icon, current file name and size, and a Change
   button. Before selection it is an inviting Choose font file action.
3. **Characters** row: a single, compact textarea with the unique code-point
   count as help text.
4. **Formats** row: horizontal checkbox group for WOFF2, WOFF, TTF, EOT, SVG,
   and CSS; it wraps at smaller widths.
5. Action row: the Generate subset button plus concise validation/progress text.
6. Results section: a heading, a Download ZIP action when outputs exist, and
   one row per output showing format, filename, byte size, and an accessible
   download icon button.

On narrow screens, form row labels move above their controls and actions remain
large enough to touch. On desktop, labels form a consistent left column. When a
file is dragged anywhere over the document, a subtle fixed drop overlay states
that the file will replace the selected font; dropping one supported font selects
it and closes the overlay.

## Technical design

### Tooling

- Add `unocss`, `@unocss/preset-wind3`, and `@unocss/preset-icons` to the docs
  toolchain, plus the Iconify JSON collection needed by the selected Lucide
  icons.
- Add `@vueuse/core` to docs dependencies.
- Add `docs/uno.config.ts` with `presetWind3()` and `presetIcons()`. Utilities
  use classes only; no Attributify mode or global preflight is enabled.
- Register `UnoCSS()` in `docs/.vitepress/config.ts` and add
  `docs/.vitepress/theme/index.ts` to extend VitePress’s default theme while
  importing `virtual:uno.css`.
- Use VitePress variables inside Uno arbitrary values (for example,
  `bg-[var(--vp-c-bg)]`), not hard-coded light/dark palettes or `dark:`
  overrides.

### Components and data flow

- `FontPlayground.vue` becomes a thin localized composition surface. It owns the
  page-wide drop target and displays the drag overlay.
- `PlaygroundForm.vue` renders the four form rows and emits only typed user
  intents: select a file, update text, toggle a format, and generate.
- `PlaygroundResults.vue` renders status and output ledger rows. It emits a ZIP
  download request and the selected output asset for individual download.
- `useFontPlayground.ts` remains the sole mutable state owner. It gains an
  explicit `downloadAsset(asset)` action and a supported-file guard that reports
  unsupported selections before generation.
- `useFileDialog({ accept, multiple: false, reset: true })` opens the native
  picker and forwards the first chosen file to the composable.
- `useDropZone(document, { multiple: false, preventDefaultForUnhandled: true,
  onDrop })` handles browser-wide drops. MIME filtering is deliberately not used
  because font MIME types are inconsistent; the shared extension guard validates
  the selected file. Safari’s drag-over limitation therefore affects only the
  overlay hint, not drop validation.
- `archive.ts` gains a single-asset download helper using `tinysaver`; ZIP
  packaging remains `fflate` plus `tinysaver`.

The obsolete drop-zone-specific upload card and multi-card option layout are
removed. State remains props-down/events-up between the composition surface and
presentational components.

## Error handling and accessibility

- The Generate button stays disabled until input, non-whitespace characters,
  and a valid non-CSS-only output selection exist.
- An unsupported file reports its extension and leaves the previous valid
  selection intact.
- File picker cancel leaves state unchanged.
- Progress text uses `role="status"`; failures use `role="alert"`.
- Every icon-only button has a localized `aria-label` and visible keyboard focus
  using VitePress brand tokens.
- The drag overlay is presentation-only and does not trap keyboard focus.

## Verification

- Extend component and composable tests for per-asset download, picker/drop
  selection, unsupported files, and checkbox validation.
- Extend archive tests to verify the single-asset `tinysaver` call.
- Run `pnpm -C docs test`, `pnpm -C docs run build`, and `pnpm run check`.
- Use a real browser to verify desktop and narrow layouts in both VitePress
  themes, no sidebar/outline, picker trigger, drag overlay, generated output
  rows, individual download, and ZIP download.
