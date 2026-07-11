# Unicode-Sliced Font Delivery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generate named Unicode-range font subsets and matching per-slice CSS across Rust, CLI, Node, WASM, and the Playground.

**Architecture:** `fontmin_core` owns the validated Unicode range and delivery-slice models. A pipeline slice plugin creates range-tagged TTF assets before normal format conversion; the CSS generator emits each source's own descriptor. Higher APIs transport camel-case strings and keep the current single-file behavior when no slices are configured.

**Tech Stack:** Rust 2024, serde, napi-rs, TypeScript ESM, Vue 3, Vitest, VitePress, Playwright, pnpm 11.

## Global Constraints

- Do not run publish commands, modify package versions, inspect registry credentials, or alter release workflows.
- Accept only `U+HEX` and `U+HEX-HEX`, with 1–6 ASCII hexadecimal digits per endpoint.
- Keep range expansion bounded to 65,536 scalar values per subset operation.
- Slice names use `[A-Za-z0-9_-]+`, are unique, and cannot contain a path separator.
- Preserve all existing behavior when `delivery` / `deliverySlices` is omitted.
- Shell commands use `rtk`, except `pnpm typecheck`.

---

### Task 1: Move the Unicode range domain model into the core crate

**Files:**
- Modify: `crates/fontmin_core/src/lib.rs`, `crates/fontmin_core/src/text.rs`
- Modify: `crates/fontmin_css/src/lib.rs`, `crates/fontmin/src/lib.rs`
- Modify: `crates/fontmin_css/Cargo.toml`
- Test: `crates/fontmin_core/src/text.rs`, `crates/fontmin_css/src/lib.rs`

**Interfaces:**

```rust
pub struct UnicodeRange { pub start: u32, pub end: u32 }
impl std::str::FromStr for UnicodeRange { type Err = FontminError; }
```

- [ ] Write failing core tests for canonical parsing, invalid scalars, and serde string round-tripping.
- [ ] Run `rtk cargo test -p fontmin_core unicode_range`; expect unresolved type errors.
- [ ] Move the parser, display, and serde implementation from CSS to core; re-export it from CSS and the facade.
- [ ] Run `rtk cargo test -p fontmin_core -p fontmin_css -p fontmin`; expect all tests to pass.
- [ ] Commit `refactor: share unicode range domain model`.

### Task 2: Add range-aware subsetting and delivery slice validation

**Files:**
- Modify: `crates/fontmin_core/src/lib.rs`, `crates/fontmin_core/src/text.rs`
- Modify: `crates/fontmin_subset/src/lib.rs`
- Test: `crates/fontmin_core/src/text.rs`, `crates/fontmin_subset/src/lib.rs`

**Interfaces:**

```rust
pub struct FontDeliverySlice {
    pub name: String,
    pub unicode_ranges: Vec<UnicodeRange>,
}

pub struct SubsetOptions {
    pub unicode_ranges: Vec<UnicodeRange>,
    // existing fields unchanged
}
```

- [ ] Write failing tests for invalid slice names, duplicates, empty range lists, bounded expansion, and a range-only subset.
- [ ] Run `rtk cargo test -p fontmin_core -p fontmin_subset`; expect the new API tests to fail.
- [ ] Add slice validation and union explicit range values with text, `unicodes`, and basic text; return a configuration diagnostic when expansion exceeds 65,536 scalars.
- [ ] Run `rtk cargo test -p fontmin_core -p fontmin_subset`; expect pass.
- [ ] Commit `feat: add unicode delivery slice models`.

### Task 3: Produce range-tagged assets and per-source CSS

**Files:**
- Modify: `crates/fontmin_config/src/config.rs`
- Modify: `crates/fontmin_pipeline/src/lib.rs`
- Modify: `crates/fontmin_plugins/src/lib.rs`
- Modify: `crates/fontmin_css/src/lib.rs`
- Test: `crates/fontmin_pipeline/src/lib.rs`, `crates/fontmin_plugins/src/lib.rs`, `crates/fontmin_css/src/lib.rs`

**Interfaces:**

```rust
pub struct DeliveryConfig { pub slices: Vec<FontDeliverySlice> }
pub struct CssFontSource { pub unicode_ranges: Vec<UnicodeRange> }
pub struct SlicePlugin { pub slices: Vec<FontDeliverySlice> }
```

- [ ] Write failing tests that configure `latin` and `cjk` slices and expect `roboto-latin.woff2`, `roboto-cjk.woff2`, and two CSS descriptors.
- [ ] Run `rtk cargo test -p fontmin_pipeline -p fontmin_plugins -p fontmin_css`; expect missing delivery types/plugin behavior.
- [ ] Add `DeliveryConfig`, insert `SlicePlugin` before format conversion, preserve metadata through conversions, and let CSS prefer source ranges over global CSS options.
- [ ] Run the same command and `rtk cargo test -p fontmin`; expect pass.
- [ ] Commit `feat: generate unicode sliced font assets`.

### Task 4: Expose slices through config and CLI

**Files:**
- Modify: `apps/fontmin/src/cli.rs`, `apps/fontmin/src/commands/build.rs`, `apps/fontmin/src/commands/mod.rs`
- Modify: `apps/fontmin/tests/cli.rs`
- Modify: `docs/guide/cli.md`, `docs/guide/config.md`, `docs/zh/guide/cli.md`, `docs/zh/guide/config.md`

**Interfaces:**

```text
--delivery-slice NAME:RANGE[,RANGE...]
delivery.slices[].unicodeRanges
```

- [ ] Write failing CLI tests for repeated slice flags, config slices, invalid names, and CLI-over-config replacement.
- [ ] Run `rtk cargo test -p fontmin_app delivery_slice`; expect failures.
- [ ] Parse the repeatable flags, validate names and ranges through the core model, and document the config/CLI forms in both languages.
- [ ] Run `rtk cargo test -p fontmin_app` and `rtk pnpm run docs:check`; expect pass.
- [ ] Commit `feat: configure unicode delivery slices`.

### Task 5: Expose range subsetting and slices in N-API and Node

**Files:**
- Modify: `napi/fontmin/src/lib.rs`, `napi/fontmin/test/api.test.ts`, `napi/fontmin/src-js/index.d.ts`
- Modify: `packages/fontmin/src/types.ts`, `native.ts`, `plugins.ts`, `optimize.ts`, `presets.ts`
- Modify: `packages/fontmin/test/api.test.ts`

**Interfaces:**

```ts
interface SubsetOptions { unicodeRanges?: string[] }
interface DeliverySlice { name: string; unicodeRanges: string[] }
function deliverySlices(slices: DeliverySlice[]): FontminPlugin
```

- [ ] Write failing N-API and package tests that generate two WOFF2 files and reject `../../escape` slice names.
- [ ] Run `rtk pnpm --filter @fontmin-rs/binding test` and `rtk pnpm --filter fontmin-rs test -- delivery`; expect failures.
- [ ] Forward range strings through bindings, introduce the Node slice plugin, and preserve per-asset range metadata in the JavaScript optimizer.
- [ ] Run `rtk pnpm run build:debug`, both test commands, and package typecheck; expect pass.
- [ ] Commit `feat: expose unicode delivery slices in node`.

### Task 6: Add WASM and Playground delivery controls

**Files:**
- Modify: `wasm/fontmin-core/src/lib.rs`, `wasm/fontmin/src/native.ts`, `optimize.ts`, `plugins.ts`, `types.ts`
- Modify: `wasm/fontmin/test/api.test.ts`, `wasm/fontmin/test/optimize.test.ts`
- Modify: `docs/.vitepress/playground/types.ts`, `font.ts`, `useFontPlayground.ts`
- Modify: `docs/.vitepress/components/PlaygroundForm.vue`, `FontPlayground.vue`, and tests
- Modify: `docs/playground.md`, `docs/zh/playground.md`, `docs/test/playground-browser.mjs`

**Interfaces:**

```ts
type PlaygroundDeliveryPreset = 'latin' | 'cjk' | 'custom'
interface BrowserDeliverySlice { name: string; unicodeRanges: string[] }
```

- [ ] Write failing WASM tests for two slice assets and Playground tests for preset/custom forwarding and malformed input.
- [ ] Run `rtk pnpm -C wasm/fontmin test` and `rtk pnpm -C docs test`; expect failures.
- [ ] Add typed browser slice plugins and a CSS-only-visible Playground delivery section; preserve no-slice behavior and show malformed descriptors before generation.
- [ ] Extend Chromium acceptance to assert both downloads and exact per-slice CSS descriptors.
- [ ] Run `rtk pnpm -C wasm/fontmin test`, `rtk pnpm -C docs test`, and `rtk pnpm -C docs run test:browser`; expect pass.
- [ ] Commit `feat: add playground unicode delivery slices`.

### Task 7: Run non-publishing release-quality verification

**Files:**
- Modify only files required by formatter or generated declarations/artifacts.

- [ ] Run `rtk pnpm run check`.
- [ ] Run `rtk sh -c 'PATH="$HOME/.cargo/bin:$PATH"; pnpm run package:smoke'` with the repository's compatible wasm-pack tool available.
- [ ] Run `rtk pnpm -C docs run test:browser`.
- [ ] Verify `rtk git status --short` is clean and no publish/version/release file changed.
- [ ] Commit `chore: verify unicode sliced delivery artifacts` only if generated tracked files changed.
