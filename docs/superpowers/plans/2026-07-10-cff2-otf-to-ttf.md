# CFF2 OTF To TTF Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert a CFF2 variable OpenType font into a static TrueType `glyf`
font at its default instance or at caller-provided variation coordinates, with
the same validation, glyph preservation, and public entry points as static CFF
conversion.

**Architecture:** Use allsorts 0.17's `font_data::FontData` and
`variations::instance` to apply CFF2 charstring blends, HVAR, MVAR, and avar
normalization into an intermediate static CFF2 sfnt. Feed that intermediate
font through the existing `fontmin_otf` CFF outline recorder and quadratic
`glyf` writer. Preserve the original non-variation metadata and layout table
bytes while taking instance-specific metrics from the intermediate font, then
remove CFF2 and every variation table in the final writer.

**Tech Stack:** Rust 2024, allsorts 0.17.0 (`font_data`, `cff::cff2`,
`cff::outline`, `tables::variable_fonts::fvar`, `variations`), existing
`fontmin_ttf` writer, bpaf, napi-rs 3, TypeScript ESM, Vitest, and pnpm 11.

## Global Constraints

- This plan implements Stage 2 from `docs/superpowers/specs/2026-07-10-cff-otf-to-ttf-design.md`; static CFF behavior from the previous plan must remain unchanged.
- Accept CFF2 input with `OTTO`, exactly one CFF2 outline table, valid `fvar`, and all required sfnt tables; reject mixed CFF/CFF2/glyf/loca outlines.
- With no supplied coordinates, use every `fvar` axis `default_value`; do not silently use zero when the font default is nonzero.
- Accept `variationCoordinates` / `--variation TAG=VALUE` as user-space coordinates keyed by four-character ASCII axis tags. Reject unknown tags, non-finite values, and values outside each axis's declared min/max range with `InvalidFont`.
- Use allsorts `variations::instance` so CFF2 blends, `avar`, HVAR, MVAR, and instance metrics use one consistent implementation. Do not hand-implement a second variation evaluator.
- Preserve the original source bytes for `cmap`, `name`, `OS/2`, `post`, `kern`, `GDEF`, `GSUB`, `GPOS`, `BASE`, and `JSTF` when they are static. If GDEF contains an ItemVariationStore, drop GDEF as a unit; reject non-removable GSUB/GPOS variation layout. Take instance-specific `head`, `hhea`, and `hmtx` values from the allsorts intermediate source before rebuilding outline tables.
- Drop `CFF2`, `fvar`, `avar`, `STAT`, `HVAR`, `VVAR`, `MVAR`, `gvar`, `cvar`, `cvt `, color, bitmap, SVG, and every other non-approved optional table from the final TTF.
- Preserve every source glyph ID and glyph count; do not subset, renumber, or create `gvar` data.
- Keep Type 2 hinting discarded, as in Stage 1. `preserve_hinting` remains accepted and has no effect on CFF2 output.
- Existing glyf-backed OTF wrapper conversion and static CFF conversion must keep their current output and error behavior.
- Shell commands in this repository use `rtk`, except `pnpm typecheck`.
- Every commit stages all local changes with `rtk git add -A`, per the user instruction.

## File Structure

- Create: `fixtures/fonts/otf/source-serif-4-variable-roman.otf` and its SHA-256 companion as the real CFF2 fixture.
- Modify: `crates/fontmin_testing/src/lib.rs` to expose the CFF2 fixture.
- Modify: `crates/fontmin_otf/src/sfnt.rs` to accept either CFF or CFF2 intermediate outlines, validate variation tables, and preserve original approved tables.
- Modify: `crates/fontmin_otf/src/lib.rs` to resolve coordinates, invoke allsorts instancing, record CFF2 outlines, and expose the expanded Rust options.
- Modify: `crates/fontmin/src/lib.rs` to expose an options-aware conversion helper while keeping `convert` backward compatible.
- Modify: `apps/fontmin/src/cli.rs`, `apps/fontmin/src/commands/mod.rs`, and `apps/fontmin/src/commands/convert.rs` to parse repeated `--variation TAG=VALUE` arguments and pass them to native conversion.
- Modify: `napi/fontmin/src/lib.rs`; regenerate `napi/fontmin/src-js/index.d.ts` and `napi/fontmin/src-js/index.js` through the existing debug build.
- Modify: `packages/fontmin/src/types.ts`, `packages/fontmin/src/native.ts`, `packages/fontmin/src/optimize.ts`, and `packages/fontmin/src/plugins.ts` to pass `variationCoordinates` through direct and builtin plugin APIs.
- Modify: `crates/fontmin_otf/src/lib.rs`, `apps/fontmin/tests/cli.rs`, `napi/fontmin/tests/api.test.ts`, and `packages/fontmin/tests/api.test.ts` with default, explicit-coordinate, and invalid-coordinate tests.
- Modify: `README.md`, `docs/architecture.md`, `docs/api/node.md`, `docs/guide/cli.md`, `docs/guide/getting-started.md`, `docs/guide/migration.md`, and matching `docs/zh/` pages to document CFF2 instancing and its static output limits.

## Shared Internal Interfaces

```rust
// crates/fontmin_otf/src/lib.rs
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Otf2TtfOptions {
    pub preserve_hinting: bool,
    pub variation_coordinates: BTreeMap<String, f32>,
}

pub fn otf_to_ttf(input: &[u8], options: &Otf2TtfOptions) -> Result<Vec<u8>>;
```

```rust
// crates/fontmin/src/lib.rs
pub fn convert_with_options(
    input: &[u8],
    target: OutputFormat,
    otf_options: &Otf2TtfOptions,
) -> Result<Vec<u8>>;

pub fn convert(input: &[u8], target: OutputFormat) -> Result<Vec<u8>> {
    convert_with_options(input, target, &Otf2TtfOptions::default())
}
```

```rust
// apps/fontmin/src/commands/convert.rs
fn parse_variations(values: &[String]) -> miette::Result<BTreeMap<String, f32>>;
```

```ts
// packages/fontmin/src/types.ts
export interface Otf2TtfOptions {
  clone?: boolean
  preserveHinting?: boolean
  variationCoordinates?: Record<string, number>
}
```

### Task 1: Add and Validate a CFF2 Fixture

**Files:**

- Create: `fixtures/fonts/otf/source-serif-4-variable-roman.otf`
- Create: `fixtures/fonts/otf/source-serif-4-variable-roman.otf.sha256`
- Modify: `crates/fontmin_testing/src/lib.rs`

**Interfaces:**

- Consumes: the existing fixture crate and fixed upstream release URL.
- Produces: `fontmin_testing::SOURCE_SERIF_4_VARIABLE_CFF2`, a valid `OTTO`
  font containing CFF2, fvar, HVAR, GPOS, and GSUB.

- [x] **Step 1: Add the fixture-shape test**

Add this test beside the static CFF fixture test:

```rust
#[test]
fn exposes_cff2_variable_fixture() {
    assert!(SOURCE_SERIF_4_VARIABLE_CFF2.starts_with(b"OTTO"));
    assert!(SOURCE_SERIF_4_VARIABLE_CFF2.windows(4).any(|tag| tag == b"CFF2"));
    assert!(SOURCE_SERIF_4_VARIABLE_CFF2.windows(4).any(|tag| tag == b"fvar"));
}
```

Run `rtk cargo test -p fontmin_testing exposes_cff2_variable_fixture` and
observe the missing-constant compile failure before adding the fixture.

- [x] **Step 2: Add the fixed fixture and checksum**

Run:

```bash
rtk curl --fail --location --output fixtures/fonts/otf/source-serif-4-variable-roman.otf \
  https://raw.githubusercontent.com/adobe-fonts/source-serif/release/VAR/SourceSerif4Variable-Roman.otf
rtk shasum -a 256 fixtures/fonts/otf/source-serif-4-variable-roman.otf
```

Require this digest:

```text
867b73c6a954a4a64616906d179f94572a748790a1d022ebeeff07f56ea0221a  fixtures/fonts/otf/source-serif-4-variable-roman.otf
```

Expose it with:

```rust
pub const SOURCE_SERIF_4_VARIABLE_CFF2: &[u8] =
    include_bytes!("../../../fixtures/fonts/otf/source-serif-4-variable-roman.otf");
```

- [x] **Step 3: Verify the fixture**

Run:

```bash
rtk shasum -a 256 -c fixtures/fonts/otf/source-serif-4-variable-roman.otf.sha256
rtk cargo test -p fontmin_testing exposes_cff2_variable_fixture
```

Expected: both commands exit `0`.

- [x] **Step 4: Commit all local changes**

```bash
rtk git add -A
rtk git commit -m "test: add cff2 variable fixture"
```

### Task 2: Resolve CFF2 Instances and Reuse the Static CFF Pipeline

**Files:**

- Modify: `crates/fontmin_otf/src/sfnt.rs`
- Modify: `crates/fontmin_otf/src/lib.rs`
- Modify: `crates/fontmin_otf/src/glyf.rs`

**Interfaces:**

- Consumes: `BTreeMap<String, f32>`, allsorts `FontData`, `FvarTable`, and
  `variations::instance`.
- Produces: one static intermediate CFF2 sfnt with instance-specific metrics,
  then one static TTF through the existing writer.

- [x] **Step 1: Write failing CFF2 conversion and coordinate tests**

Add these tests in `crates/fontmin_otf/src/lib.rs`:

```rust
#[test]
fn converts_default_cff2_instance_to_valid_ttf() {
    let output = otf_to_ttf(
        SOURCE_SERIF_4_VARIABLE_CFF2,
        &Otf2TtfOptions::default(),
    )
    .unwrap();
    let info = fontmin_ttf::inspect_ttf(&output).unwrap();

    assert_eq!(info.family_name.as_deref(), Some("Source Serif 4 Variable"));
    assert!(info.tables.iter().any(|tag| tag == "glyf"));
    assert!(!info.tables.iter().any(|tag| tag == "CFF2"));
    assert!(!info.tables.iter().any(|tag| tag == "fvar"));
    assert!(!info.tables.iter().any(|tag| tag == "HVAR"));
    assert_eq!(fontmin_ttf::calculate_table_checksum(&output), 0xB1B0_AFBA);
}

#[test]
fn explicit_cff2_coordinates_change_the_static_instance() {
    let default_output = otf_to_ttf(
        SOURCE_SERIF_4_VARIABLE_CFF2,
        &Otf2TtfOptions::default(),
    )
    .unwrap();
    let mut coordinates = BTreeMap::new();
    coordinates.insert("wght".to_owned(), 700.0);
    coordinates.insert("opsz".to_owned(), 14.0);
    let explicit_output = otf_to_ttf(
        SOURCE_SERIF_4_VARIABLE_CFF2,
        &Otf2TtfOptions {
            preserve_hinting: false,
            variation_coordinates: coordinates,
        },
    )
    .unwrap();

    assert_ne!(default_output, explicit_output);
}

#[test]
fn rejects_unknown_and_out_of_range_cff2_coordinates() {
    for (tag, value) in [("XXXX", 1.0), ("wght", 10000.0)] {
        let mut variation_coordinates = BTreeMap::new();
        variation_coordinates.insert(tag.to_owned(), value);
        let error = otf_to_ttf(
            SOURCE_SERIF_4_VARIABLE_CFF2,
            &Otf2TtfOptions {
                preserve_hinting: false,
                variation_coordinates,
            },
        )
        .unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::InvalidFont);
    }
}
```

- [x] **Step 2: Run the new tests and verify CFF2 is rejected before support**

Run:

```bash
rtk cargo test -p fontmin_otf converts_default_cff2_instance_to_valid_ttf
rtk cargo test -p fontmin_otf explicit_cff2_coordinates_change_the_static_instance
```

Expected: the tests fail because the current source validator rejects CFF2.

- [x] **Step 3: Generalize source validation for CFF2 intermediates**

In `sfnt.rs`, replace the CFF-only required-table check with an outline tag
selection that accepts exactly one of `CFF ` or `CFF2`. Keep the existing
required tables and overlap checks. Add `outline_tag: String` to
`StaticCffSource` and make `table("CFF ")` / `table("CFF2")` use the selected
tag. Remove CFF2 from the unconditional rejection list, but reject the
variation and color tags listed in the global constraints when they appear in
the intermediate source. Add:

```rust
pub(crate) fn approved_tables_from<'a, 'b>(
    source: &StaticCffSource<'a>,
    original: Option<&StaticCffSource<'b>>,
) -> Vec<OwnedSfntTable>;
```

For each approved tag, use `original` bytes when present and fall back to
`source` bytes. Always use rebuilt outline tables for `head`, `hhea`, `hmtx`,
`maxp`, `glyf`, and `loca`; never copy CFF/CFF2 or variation tags.

- [x] **Step 4: Implement coordinate validation and allsorts instancing**

Add a helper with this behavior:

```rust
fn instance_cff2(
    input: &[u8],
    coordinates: &BTreeMap<String, f32>,
) -> Result<Vec<u8>>;
```

Read `FontData<'_>` through `ReadScope`, obtain provider `0`, parse `fvar`,
build one `Fixed` value per axis using the caller's value or
`axis.default_value`, and call `allsorts::variations::instance`. Before the
call, reject any map key that is not an exact four-byte ASCII axis tag, any
unknown axis, non-finite value, or value outside `[min_value, max_value]`.
Map allsorts parse/variation/write failures to `InvalidFont` with the axis or
operation in the message. The call shape is:

```rust
let user_instance = fvar
    .axes()
    .map(|axis| Fixed::from(coordinate_for_axis(axis)))
    .collect::<Vec<_>>();
let (instanced, _) = allsorts::variations::instance(&provider, &user_instance)
    .map_err(|error| FontminError::invalid_font(error.to_string()))?;
```

- [x] **Step 5: Route CFF2 through the existing outline recorder**

In `otf_to_ttf`, detect the original `CFF2` outline, read the original source
for approved table bytes, call `instance_cff2`, validate the returned static
CFF2 source, parse `allsorts::cff::cff2::CFF2`, and visit every glyph through
`CFF2Outlines` with `None` for the tuple. Use the intermediate source metrics
when rebuilding `hhea` and `hmtx`, pass the original source to
`approved_tables_from`, write with `fontmin_ttf::write_ttf`, and run the same
TTF validation as static CFF. Keep the existing `CFFOutlines` branch untouched
for static CFF.

- [x] **Step 6: Run focused Rust verification**

Run:

```bash
rtk cargo fmt --all --check
rtk cargo test -p fontmin_otf
rtk cargo clippy -p fontmin_otf --all-targets --all-features
```

Expected: all tests pass and CFF2 output has valid `glyf`/`loca`, no CFF2 or
variation tables, original names/layout tables, and a checksum of
`0xB1B0_AFBA`.

- [x] **Step 7: Commit all local changes**

```bash
rtk git add -A
rtk git commit -m "feat: instantiate cff2 fonts before conversion"
```

### Task 3: Expand Rust, CLI, N-API, and TypeScript Options

**Files:**

- Modify: `crates/fontmin/src/lib.rs`
- Modify: `apps/fontmin/src/cli.rs`
- Modify: `apps/fontmin/src/commands/mod.rs`
- Modify: `apps/fontmin/src/commands/convert.rs`
- Modify: `napi/fontmin/src/lib.rs`
- Regenerate: `napi/fontmin/src-js/index.d.ts`, `napi/fontmin/src-js/index.js`
- Modify: `packages/fontmin/src/types.ts`
- Modify: `packages/fontmin/src/native.ts`
- Modify: `packages/fontmin/src/optimize.ts`
- Modify: `packages/fontmin/src/plugins.ts`

**Interfaces:**

- Consumes: `Otf2TtfOptions.variation_coordinates` and the existing OTF
  conversion entry points.
- Produces: one coordinate map shape across Rust, CLI, N-API, direct TypeScript
  API, and builtin `otf2ttf` plugin.

- [x] **Step 1: Add a Rust options-aware conversion test**

Add a `fontmin` crate test that calls `convert_with_options` with
`OutputFormat::Ttf` and `{ "wght": 700.0, "opsz": 14.0 }`, then asserts a
TrueType flavor and no `CFF2`/`fvar` tables. Keep the existing `convert` tests
to prove the default helper remains backward compatible.

- [x] **Step 2: Add CLI variation parsing and a failing CLI test**

Add `variation: Vec<String>` to `Command::Convert` with bpaf's repeated
`--variation TAG=VALUE` argument. Implement `parse_variations` by splitting
each value once on `=`, parsing a finite `f32`, and rejecting duplicate tags,
wrong tag length/ASCII, missing values, or invalid numbers. Call
`fontmin::convert_with_options` from `convert::run` with the parsed map. Add a
CLI fixture test invoking:

```text
fontmin-rs convert input.otf --format ttf --variation wght=700 --variation opsz=14 --output output.ttf
```

and assert success, TTF flavor, and no CFF2/fvar tables.

- [x] **Step 3: Add N-API variation coordinates**

Add `variation_coordinates: Option<HashMap<String, f32>>` to
`JsOtf2TtfOptions`, convert it into `BTreeMap<String, f32>`, and pass it to
the Rust options. Run `rtk pnpm run build:debug` so the generated binding
declarations expose `variationCoordinates?: Record<string, number>`.

- [x] **Step 4: Add TypeScript direct and plugin forwarding**

Add `variationCoordinates?: Record<string, number>` to both the public and
native OTF option interfaces. In `otfToTtf`, assign the map when defined. In
`otf2TtfOptions`, copy only a plain object whose values are finite numbers,
leaving native validation to reject unknown/out-of-range axes. The builtin
`otf2ttf(options)` already forwards its option object and needs no new plugin
shape beyond the type.

- [x] **Step 5: Run public API focused tests**

Run:

```bash
rtk pnpm run build:debug
rtk cargo test -p fontmin_app convert_command_converts_cff2_coordinates
rtk pnpm --filter @fontmin-rs/binding test -- api.test.ts
rtk pnpm --filter fontmin-rs test -- api.test.ts
pnpm typecheck
```

Expected: default and explicit coordinates work through all three public
surfaces, and invalid coordinate maps throw an error mentioning the axis.

- [x] **Step 6: Commit all local changes**

```bash
rtk git add -A
rtk git commit -m "feat: expose cff2 variation coordinates"
```

### Task 4: Document and Verify the Complete CFF2 Delivery

**Files:**

- Modify: `README.md`
- Modify: `docs/architecture.md`
- Modify: `docs/api/node.md`
- Modify: `docs/guide/cli.md`
- Modify: `docs/guide/getting-started.md`
- Modify: `docs/guide/migration.md`
- Modify: `docs/zh/api/node.md`
- Modify: `docs/zh/architecture.md`
- Modify: `docs/zh/guide/cli.md`
- Modify: `docs/zh/guide/getting-started.md`
- Modify: `docs/zh/guide/migration.md`

**Interfaces:**

- Consumes: tested CFF2 behavior and public option names.
- Produces: documentation that states CFF2 is instantiated to static TTF,
  coordinates use user-space axis values, Type 2 hinting is discarded, and no
  variable tables remain in output.

- [x] **Step 1: Update documentation examples**

Add an English CLI example:

```bash
fontmin-rs convert fixtures/fonts/otf/source-serif-4-variable-roman.otf \
  --format ttf --variation wght=700 --variation opsz=14 \
  --output build/source-serif-4-700.ttf
```

Document the matching TypeScript call:

```ts
otfToTtf(input, { variationCoordinates: { wght: 700, opsz: 14 } })
```

Add equivalent Chinese wording. Explicitly state that output is static, CFF2,
fvar, avar, HVAR, MVAR, and other variation tables are removed, and Type 2
hinting is not preserved.

- [x] **Step 2: Run the full repository gate**

Run:

```bash
rtk pnpm run check
rtk git diff --check
rtk git status --short
```

Expected: all checks exit `0`, warnings remain non-fatal, and status lists only
the intended files before the final commit.

- [x] **Step 3: Run requirement-level verification**

Run:

```bash
rtk shasum -a 256 -c fixtures/fonts/otf/source-serif-4-variable-roman.otf.sha256
rtk cargo test -p fontmin_otf converts_default_cff2_instance_to_valid_ttf
rtk cargo test -p fontmin_app convert_command_converts_cff2_coordinates
rtk pnpm --filter @fontmin-rs/binding test -- api.test.ts
rtk pnpm --filter fontmin-rs test -- api.test.ts
```

Expected: every command exits `0`; all output assertions prove static TTF
flavor, glyph count, names, layout preservation, coordinate-dependent output,
and removal of CFF2/variation tables.

- [x] **Step 4: Commit all local changes**

```bash
rtk git add -A
rtk git commit -m "test: cover cff2 otf conversion"
```

## Plan Self-Review

**Spec coverage:** The plan covers CFF2 default and explicit user-space
coordinates, unknown/out-of-range validation, allsorts instancing, CFF2 to
quadratic glyf conversion, instance metrics, approved table preservation,
variation-table removal, Rust/CLI/N-API/TypeScript forwarding, documentation,
and full verification. Static CFF and glyf-backed OTF behavior are protected by
existing tests and are not reimplemented.

**Placeholder scan:** The plan contains no TODO/TBD/deferred implementation
steps. Every task names exact files, functions, commands, and expected results.

**Type consistency:** `variation_coordinates` is a `BTreeMap<String, f32>` in
Rust, `HashMap<String, f32>` only at the N-API boundary, and
`Record<string, number>` in TypeScript. CLI parsing produces the same Rust map.
`convert_with_options` preserves the existing `convert` signature, and all
public callers continue to use `otf_to_ttf` through one options shape.
