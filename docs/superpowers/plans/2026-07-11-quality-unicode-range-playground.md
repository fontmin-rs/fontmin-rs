# Quality Gates, Unicode Range, and Playground Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add non-publishing quality gates, safe `@font-face` Unicode-range support, and a localized Playground interface for that feature.

**Architecture:** Quality gates remain scripts and CI jobs rather than release steps. `fontmin_css` owns parsing and canonical CSS serialization; all higher layers transport `unicodeRanges` as strings. The Playground performs lightweight client-side validation for feedback, while WASM remains authoritative for CSS generation.

**Tech Stack:** Rust 2024, serde, napi-rs, TypeScript ESM, Vitest, VitePress, Vue 3, Playwright, pnpm 11.

## Global Constraints

- Do not run a publish command, modify a package version, add an npm registry dependency, or alter the release workflow.
- Accept only `U+HEX` and `U+HEX-HEX`, with 1–6 ASCII hexadecimal digits per endpoint.
- Reject Unicode values above `10FFFF`, inverted ranges, whitespace inside descriptors, wildcard syntax, and non-ASCII input.
- Canonical output uses uppercase hexadecimal digits with at least four digits and joins descriptors with `, `.
- Preserve every existing API when `unicodeRanges` is omitted.
- Shell commands in this repository use `rtk`, except `pnpm typecheck`.

---

### Task 1: Make docs tests and builds first-class quality gates

**Files:**
- Modify: `package.json`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Produces root scripts `docs:test` and `docs:check`.
- Extends `pnpm run check` to cover the VitePress package without publishing it.

- [ ] **Step 1: Add the failing root-script expectation**

Run:

```bash
rtk pnpm run docs:test
```

Expected: FAIL because the root script does not exist.

- [ ] **Step 2: Add docs scripts and chain them into `check`**

In root `package.json`, add:

```json
"docs:test": "pnpm -C docs run test",
"docs:check": "pnpm run docs:test && pnpm run docs:build"
```

Change `check` to append `&& pnpm run docs:check` after the existing test
command. This keeps root tests, docs unit tests, and VitePress rendering in one
local gate.

- [ ] **Step 3: Extend CI without duplicating platform matrices**

Add `pnpm run docs:check` to the existing Ubuntu `check` job after type
checking. It runs once per change and does not add docs work to each of the
nine native test matrix entries.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
rtk pnpm run docs:check
rtk pnpm run check
```

Expected: both pass; docs unit tests and VitePress build are present in root
output.

- [ ] **Step 5: Commit**

```bash
rtk git add package.json .github/workflows/ci.yml
rtk git commit -m "ci: gate docs tests and build"
```

### Task 2: Verify local npm tarballs without publishing

**Files:**
- Create: `scripts/package-smoke.mjs`
- Modify: `package.json`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Produces `pnpm run package:smoke`.
- Consumes tarballs from `pnpm pack` and tests the ESM package entry points in
  disposable directories.

- [ ] **Step 1: Write the failing smoke script test**

Create a script that exits nonzero until `fontmin-rs` and `@fontmin-rs/wasm`
tarballs are available. Its intended consumer commands are:

```js
await execFile('npm', ['install', '--ignore-scripts', tarball])
await execFile('node', ['--input-type=module', '--eval', source])
```

The native source imports `fontmin-rs` and asserts that its exported
`inspect` is a function. The WASM source imports `@fontmin-rs/wasm` and
asserts that `initWasm` is a function. Both execute from a temporary directory
created with `mkdtemp` and are removed in `finally`.

- [ ] **Step 2: Verify RED**

Run:

```bash
rtk node scripts/package-smoke.mjs
```

Expected: FAIL because the script does not exist.

- [ ] **Step 3: Implement isolated packing and installation**

Implement `scripts/package-smoke.mjs` with `node:child_process/promises`,
`node:fs/promises`, `node:os`, and `node:path`. It must:

1. call `pnpm pack` in `packages/fontmin` and `wasm/fontmin`;
2. parse the final nonempty stdout line as each tarball path;
3. install each tarball in its own temporary package with npm;
4. run the ESM import assertion;
5. remove temporary directories and tarballs even after a failed assertion.

Add this root script:

```json
"package:smoke": "pnpm --filter @fontmin-rs/binding build:debug && pnpm --filter fontmin-rs build && pnpm -C wasm/fontmin build && node scripts/package-smoke.mjs"
```

Use the rustup toolchain in CI's existing `wasm-browser` job to run
`pnpm run package:smoke` after its WASM build. Do not add this command to the
release workflow.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
rtk sh -c 'PATH="$HOME/.cargo/bin:$PATH"; pnpm run package:smoke'
```

Expected: PASS and leave no `.tgz` or temporary-consumer directory tracked.

- [ ] **Step 5: Commit**

```bash
rtk git add scripts/package-smoke.mjs package.json .github/workflows/ci.yml
rtk git commit -m "test: smoke test packed packages"
```

### Task 3: Add an actual Playground browser acceptance flow

**Files:**
- Create: `docs/test/playground-browser.mjs`
- Modify: `docs/package.json`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Produces `pnpm -C docs test:browser`.
- Uses the static VitePress output and a fixture upload instead of mocking
  browser WASM or download helpers.

- [ ] **Step 1: Write the failing acceptance script**

The script imports `chromium` from Playwright, serves `docs/.vitepress/dist`,
and requires the page to:

1. open `/playground`;
2. set `fixtures/fonts/ttf/roboto-regular.ttf` on the file input;
3. enter `Hello` into `#playground-characters`;
4. click `[data-testid="generate"]`;
5. wait for `roboto.woff2` and `roboto.css` result rows;
6. assert the individual WOFF2 download and ZIP download events each produce
   the expected filename.

- [ ] **Step 2: Verify RED**

Run:

```bash
rtk pnpm -C docs test:browser
```

Expected: FAIL because the command and script do not exist.

- [ ] **Step 3: Implement the static-server harness**

Add Playwright to docs dev dependencies. Implement a `node:http` static
server that maps `/` to `index.html`, rejects traversal, and sends
`application/wasm` for `.wasm`. Build docs before launching it. Add:

```json
"test:browser": "pnpm run build && node test/playground-browser.mjs"
```

The test waits for the result section rather than arbitrary sleeps and closes
server and browser in nested `finally` blocks.

- [ ] **Step 4: Verify GREEN and wire CI**

Run:

```bash
rtk pnpm --filter @fontmin-rs/wasm exec playwright install --with-deps chromium
rtk pnpm -C docs test:browser
```

Add a `playground-browser` CI job that needs `wasm-browser`, uses Chromium,
installs the WASM target and wasm-pack, then runs the docs browser command.

- [ ] **Step 5: Commit**

```bash
rtk git add docs/package.json docs/test/playground-browser.mjs pnpm-lock.yaml .github/workflows/ci.yml
rtk git commit -m "test: verify playground in chromium"
```

### Task 4: Model and emit canonical Unicode ranges in Rust CSS

**Files:**
- Modify: `crates/fontmin_css/src/lib.rs`
- Modify: `crates/fontmin/src/lib.rs`
- Test: `crates/fontmin_css/src/lib.rs`

**Interfaces:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnicodeRange { pub start: u32, pub end: u32 }

impl std::str::FromStr for UnicodeRange {
    type Err = FontminError;
}
```

`CssOptions` gains `pub unicode_ranges: Vec<UnicodeRange>`, serialized as
`unicodeRanges`. `fontmin` re-exports `UnicodeRange`.

- [ ] **Step 1: Write failing parser and emission tests**

Add tests that assert:

```rust
assert_eq!("u+2a".parse::<UnicodeRange>().unwrap().to_string(), "U+002A");
assert_eq!("U+4e00-9fff".parse::<UnicodeRange>().unwrap().to_string(), "U+4E00-9FFF");
for value in ["U+", "U+110000", "U+FF-00", "U+4??", "U+0041; color:red"] {
    assert!(value.parse::<UnicodeRange>().is_err());
}
```

Add a CSS assertion for `unicode-range: U+0020-007E, U+4E00-9FFF;` and one
assertion that the property is omitted for the default empty list.

- [ ] **Step 2: Verify RED**

Run:

```bash
rtk cargo test -p fontmin_css unicode_range
```

Expected: FAIL because `UnicodeRange` and `CssOptions::unicode_ranges` do not
exist.

- [ ] **Step 3: Implement the bounded parser**

Use `strip_prefix("U+")`, split once on `-`, and parse endpoints with a helper
that checks 1–6 ASCII hex digits before `u32::from_str_radix`. Return
`FontminError::config` for every invalid descriptor. Implement `Display` with
`format!("U+{:04X}")` for a singleton and `format!("U+{:04X}-{:04X}")` for a
range. In `generate_font_face_css`, append this exact line after `font-style`:

```rust
if !options.unicode_ranges.is_empty() {
    writeln!(css, "  unicode-range: {};", options.unicode_ranges.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "))
        .expect("writing to string should not fail");
}
```

- [ ] **Step 4: Verify GREEN**

Run:

```bash
rtk cargo test -p fontmin_css
rtk cargo test -p fontmin
```

Expected: PASS; Rust public facade exposes the same option model.

- [ ] **Step 5: Commit**

```bash
rtk git add crates/fontmin_css/src/lib.rs crates/fontmin/src/lib.rs
rtk git commit -m "feat: support css unicode ranges"
```

### Task 5: Forward Unicode ranges through config, CLI, N-API, Node, and WASM

**Files:**
- Modify: `crates/fontmin_config/src/config.rs`
- Modify: `apps/fontmin/src/cli.rs`, `apps/fontmin/src/commands/build.rs`, `apps/fontmin/tests/cli.rs`
- Modify: `napi/fontmin/src/lib.rs`, `napi/fontmin/test/api.test.ts`
- Modify: `packages/fontmin/src/types.ts`, `packages/fontmin/src/native.ts`, `packages/fontmin/src/optimize.ts`, `packages/fontmin/test/api.test.ts`
- Modify: `wasm/fontmin/src/native.ts`, `wasm/fontmin/test/api.test.ts`

**Interfaces:**
- Config and JS use `unicodeRanges?: string[]`.
- Build CLI accepts repeated `--css-unicode-range RANGE`.

- [ ] **Step 1: Add cross-boundary failing tests**

Add one assertion per boundary that generated CSS contains
`unicode-range: U+0020-007E;`. Add invalid input assertions that each public
boundary reports `U+4??` as invalid and never returns CSS. Add a CLI fixture
case with two repeatable flags and a config case using:

```json
{ "css": { "unicodeRanges": ["U+0020-007E"] } }
```

- [ ] **Step 2: Verify RED**

Run:

```bash
rtk cargo test -p fontmin_app css_unicode_range
rtk pnpm --filter @fontmin-rs/binding test -- unicode
rtk pnpm --filter fontmin-rs test -- unicode
rtk pnpm -C wasm/fontmin test -- unicode
```

Expected: FAIL because options are absent or ignored.

- [ ] **Step 3: Implement only forwarding and conversion**

Add `unicode_ranges: Vec<String>` with `#[serde(default)]` to `CssConfig` and
map each item with `UnicodeRange::from_str`, retaining the config error. Add a
repeatable bpaf build option and make nonempty CLI values replace config
values. Add optional `Vec<String>` to `JsCssOptions`; parse it in
`css_options_from_js`. Add `unicodeRanges?: string[]` to TypeScript and assign
it in both direct native and plugin option records. The WASM facade already
deserializes camel-case JSON through `CssOptions`; add only type forwarding and
tests there.

- [ ] **Step 4: Regenerate N-API declarations and verify GREEN**

Run:

```bash
rtk pnpm run build:debug
rtk cargo test -p fontmin_app
rtk pnpm --filter @fontmin-rs/binding test
rtk pnpm --filter fontmin-rs test
rtk pnpm -C wasm/fontmin test
```

Expected: PASS; generated binding declarations include `unicodeRanges`.

- [ ] **Step 5: Commit**

```bash
rtk git add crates/fontmin_config apps/fontmin napi/fontmin packages/fontmin wasm/fontmin
rtk git commit -m "feat: expose css unicode ranges"
```

### Task 6: Add a localized Unicode-range control to the Playground

**Files:**
- Modify: `docs/.vitepress/playground/types.ts`, `font.ts`, `useFontPlayground.ts`
- Modify: `docs/.vitepress/components/FontPlayground.vue`, `PlaygroundForm.vue`, `FontPlayground.test.ts`
- Modify: `docs/.vitepress/playground/font.test.ts`, `useFontPlayground.test.ts`
- Modify: `docs/playground.md`, `docs/zh/playground.md`

**Interfaces:**
- `ProcessFontRequest` gains `unicodeRanges?: string[]`.
- `useFontPlayground()` exposes `unicodeRanges` and `setUnicodeRanges(value)`.

- [ ] **Step 1: Write failing request and component tests**

Cover valid comma-separated input `u+20-7e, U+4e00-9fff`, forwarding as
`['U+0020-007E', 'U+4E00-9FFF']`, and malformed `U+4??` which disables
generation and shows an error. Assert the field is visible only while CSS is
selected and that English and Chinese helper text distinguishes character
subsetting from browser matching.

- [ ] **Step 2: Verify RED**

Run:

```bash
rtk pnpm -C docs test -- font.test.ts useFontPlayground.test.ts FontPlayground.test.ts
```

Expected: FAIL because request state and the form control do not exist.

- [ ] **Step 3: Implement shared client validation and form wiring**

In `font.ts`, export `parseUnicodeRanges(value: string): string[]`, using the
same `U+HEX`/`U+HEX-HEX` grammar and canonical uppercase formatter as Rust.
Pass its result only to `generateFontFaceCss` as `{ unicodeRanges }`; it must
not alter `subsetTtf` options. Add a localized text field to `PlaygroundForm`
when CSS is selected and bind it through `FontPlayground` to the composable.
Store validation errors before generation; keep its text state when CSS is
unchecked.

- [ ] **Step 4: Add user documentation**

Add one concise English and Chinese paragraph plus a `U+0020-007E,
U+4E00-9FFF` example. State explicitly that ranges do not add glyphs to the
subset; they only affect browser matching.

- [ ] **Step 5: Verify GREEN**

Run:

```bash
rtk pnpm -C docs test
rtk pnpm -C docs build
rtk pnpm -C docs test:browser
```

Expected: PASS; generated CSS in the actual browser flow contains the canonical
descriptor list.

- [ ] **Step 6: Commit**

```bash
rtk git add docs
rtk git commit -m "feat: configure unicode ranges in playground"
```

### Task 7: Complete the non-publishing delivery audit

**Files:**
- Modify: `README.md`, `docs/api/node.md`, `docs/api/wasm.md`, matching `docs/zh/` API pages
- Test only: root workspace and CI workflow

- [ ] **Step 1: Document all public option names**

Add a Node and WASM `generateFontFaceCss` example using
`unicodeRanges: ['U+0020-007E']`, then update README feature wording to state
that CSS Unicode ranges are supported.

- [ ] **Step 2: Verify every quality gate without publishing**

Run:

```bash
rtk sh -c 'PATH="$HOME/.cargo/bin:$PATH"; pnpm run package:smoke'
rtk pnpm run check
rtk pnpm -C docs test:browser
rtk git diff --check
rtk git status --short
```

Expected: all checks pass; `git status` lists only intended source and
documentation changes before the final commit.

- [ ] **Step 3: Commit and confirm no publication occurred**

```bash
rtk git add README.md docs
rtk git commit -m "docs: document css unicode ranges"
rtk git log --oneline -8
```

Expected: local commits only; no `pnpm publish`, tag, version change, or
release-workflow invocation appears in the command history.

## Plan Self-Review

- Spec coverage: Tasks 1–3 deliver non-publishing verification; Tasks 4–5 add
  bounded, canonical Unicode ranges to every core boundary; Task 6 exposes it
  in the localized browser product; Task 7 documents and audits it.
- Placeholder scan: no deferred implementation or ambiguous acceptance step is
  present.
- Type consistency: every external boundary names the field `unicodeRanges`,
  while Rust uses `unicode_ranges` through serde camel-case conversion.
