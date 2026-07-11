# Static CFF OTF To TTF Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert a static OpenType CFF font into a valid static TrueType `glyf`
font while preserving source glyph IDs, metrics, names, cmap mappings, and the
approved OpenType layout tables.

**Architecture:** `fontmin_otf` will parse the existing sfnt directory, use
`allsorts` only to interpret CFF Type 2 outlines, and own a small conversion
pipeline for cubic subdivision, quadratic `glyf` serialization, and sfnt table
selection. `fontmin_ttf::write_ttf` remains the only sfnt directory and checksum
writer. The existing `fontmin`, CLI, N-API, and TypeScript calls already route
default OTF-to-TTF conversion through this crate, so they need fixture-backed
regression tests rather than new public conversion entry points.

**Tech Stack:** Rust 2024, `allsorts` 0.17.0 with `outline` and `flate2_rust`,
`ttf-parser` 0.25, existing `fontmin_ttf` writer, napi-rs 3, TypeScript ESM,
Vitest, and pnpm 11.

## Global Constraints

- This plan implements only Stage 1 from `docs/superpowers/specs/2026-07-10-cff-otf-to-ttf-design.md`: static CFF OTF to static TTF. CFF2 axes, `--variation`, and `variationCoordinates` are a separate Stage 2 plan.
- Accept only `OTTO` input containing exactly one `CFF ` table and no `CFF2`, `glyf`, or `loca` outline table.
- Preserve every source glyph ID. Do not subset, renumber, composite, or synthesize glyphs.
- Retain source table bytes for `cmap`, `name`, `OS/2`, `post`, `kern`, `GDEF`, `GSUB`, `GPOS`, `BASE`, and `JSTF`; rebuild `head`, `maxp`, `hhea`, `hmtx`, `glyf`, and `loca`; drop every other optional table, including `CFF ` and `VORG`.
- Reject source fonts containing `CFF2`, `COLR`, `CPAL`, `CBDT`, `CBLC`, `sbix`, or `SVG ` with `UnsupportedFormat`.
- Type 2 stem, mask, and hint operators are intentionally ignored; `preserve_hinting` remains accepted but does not change CFF output.
- Approximate every cubic with recursive De Casteljau subdivision until the analytic unrounded error bound `sqrt(3) * |P0 - 3P1 + 3P2 - P3| / 36` is at most `1.0 - sqrt(0.5)` font units. The final integer coordinate rounding can move a point by at most `sqrt(0.5)`, so the combined error stays at or below one source-font unit.
- Convert parser/outline/serialization failures to `InvalidFont`, unsupported source content to `UnsupportedFormat`, and failed output validation to `ConvertFailed`.
- No library conversion path may return or write partial output.
- Shell commands in this repository use `rtk`, except `pnpm typecheck`.
- Every commit stages all local changes with `rtk git add -A`, per the user instruction.

---

## File Structure

- Modify: `Cargo.toml` to expose the shared `allsorts` workspace dependency.
- Modify: `crates/fontmin_otf/Cargo.toml` to use `allsorts` and `ttf-parser` directly.
- Create: `fixtures/fonts/otf/source-sans-3-regular.otf` and its SHA-256 companion as the licensed real static CFF fixture.
- Modify: `crates/fontmin_testing/src/lib.rs` to expose the static CFF fixture to Rust tests.
- Modify: `crates/fontmin_diagnostics/src/lib.rs` to provide a constructor for `ConvertFailed` errors.
- Create: `crates/fontmin_otf/src/sfnt.rs` for validated OTF directory access, table selection, and rebuilt source table bytes.
- Create: `crates/fontmin_otf/src/outline.rs` for an `allsorts::outline::OutlineSink` recorder and the bounded cubic-to-quadratic converter.
- Create: `crates/fontmin_otf/src/glyf.rs` for simple-glyph serialization and replacement `head`, `maxp`, `hhea`, `hmtx`, `glyf`, and `loca` tables.
- Modify: `crates/fontmin_otf/src/lib.rs` to orchestrate CFF parsing, outline extraction, TrueType rebuilding, `fontmin_ttf::write_ttf`, and output validation while preserving the glyf-backed wrapper behavior.
- Modify: `apps/fontmin/tests/cli.rs`, `napi/fontmin/test/api.test.ts`, and `packages/fontmin/test/api.test.ts` with real CFF conversion assertions.
- Modify: `README.md`, `docs/api/node.md`, `docs/guide/cli.md`, and the matching `docs/zh/` pages so the documented conversion support matches the implementation.

## Shared Internal Interfaces

The new modules use these crate-private interfaces. Keep them private unless
another crate genuinely needs them.

```rust
// crates/fontmin_otf/src/outline.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Segment {
    Line(Point),
    Quadratic { control: Point, to: Point },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Contour {
    pub start: Point,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct GlyphPath {
    pub contours: Vec<Contour>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct QuadraticPiece {
    pub from: Point,
    pub control: Point,
    pub to: Point,
    pub t_start: f64,
    pub t_end: f64,
}

pub(crate) fn cubic_to_quadratics(
    from: Point,
    control1: Point,
    control2: Point,
    to: Point,
 ) -> Result<Vec<QuadraticPiece>>;
```

```rust
// crates/fontmin_otf/src/glyf.rs
pub(crate) struct TrueTypeOutlineTables {
    pub head: Vec<u8>,
    pub hhea: Vec<u8>,
    pub hmtx: Vec<u8>,
    pub maxp: Vec<u8>,
    pub glyf: Vec<u8>,
    pub loca: Vec<u8>,
}

pub(crate) struct EncodedGlyf {
    pub glyf: Vec<u8>,
    pub loca: Vec<u8>,
    pub max_points: u16,
    pub max_contours: u16,
}

pub(crate) fn encode_glyf_and_loca(glyphs: &[GlyphPath]) -> Result<EncodedGlyf>;

pub(crate) fn build_truetype_outline_tables(
    source: &sfnt::StaticCffSource<'_>,
    glyphs: &[GlyphPath],
) -> Result<TrueTypeOutlineTables>;
```

```rust
// crates/fontmin_otf/src/sfnt.rs
pub(crate) struct StaticCffSource<'a> {
    pub tables: BTreeMap<String, &'a [u8]>,
    pub num_glyphs: u16,
    pub metrics: Vec<HorizontalMetric>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct HorizontalMetric {
    pub advance_width: u16,
    pub left_side_bearing: i16,
}

pub(crate) fn read_static_cff_source(input: &[u8]) -> Result<StaticCffSource<'_>>;
pub(crate) fn output_tables(
    source: &StaticCffSource<'_>,
    outlines: TrueTypeOutlineTables,
) -> Vec<fontmin_ttf::OwnedSfntTable>;
```

### Task 1: Add the Static CFF Fixture and Conversion Dependencies

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/fontmin_otf/Cargo.toml`
- Create: `fixtures/fonts/otf/source-sans-3-regular.otf`
- Create: `fixtures/fonts/otf/source-sans-3-regular.otf.sha256`
- Modify: `crates/fontmin_testing/src/lib.rs`

**Interfaces:**

- Consumes: current workspace dependency catalog and shared fixture crate.
- Produces: `fontmin_testing::SOURCE_SANS_3_REGULAR_CFF`, a static `OTTO` font with a `CFF ` table, and direct access to `allsorts` outline APIs from `fontmin_otf`.

- [x] **Step 1: Add failing fixture-shape tests**

Add this test to `crates/fontmin_testing/src/lib.rs` before introducing the
fixture constant:

```rust
#[test]
fn exposes_static_cff_fixture() {
    assert!(SOURCE_SANS_3_REGULAR_CFF.starts_with(b"OTTO"));
    assert!(SOURCE_SANS_3_REGULAR_CFF.windows(4).any(|tag| tag == b"CFF "));
}
```

- [x] **Step 2: Run the fixture test and verify the expected compile failure**

Run:

```bash
rtk cargo test -p fontmin_testing exposes_static_cff_fixture
```

Expected: FAIL because `SOURCE_SANS_3_REGULAR_CFF` is not defined.

- [x] **Step 3: Add the verified binary fixture and dependency declarations**

Fetch the fixture from the fixed upstream branch, keep the binary under the
repository, and write the displayed digest into the companion file:

```bash
rtk mkdir -p fixtures/fonts/otf
rtk curl --fail --location --output fixtures/fonts/otf/source-sans-3-regular.otf \
  https://raw.githubusercontent.com/adobe-fonts/source-sans/release/OTF/SourceSans3-Regular.otf
rtk shasum -a 256 fixtures/fonts/otf/source-sans-3-regular.otf
```

The SHA-256 must be exactly:

```text
08df266400933d3178d081a45f94a08814c3e55b4b7dd2e0ff69cb1329f13ab6
```

Write `fixtures/fonts/otf/source-sans-3-regular.otf.sha256` as:

```text
08df266400933d3178d081a45f94a08814c3e55b4b7dd2e0ff69cb1329f13ab6  fixtures/fonts/otf/source-sans-3-regular.otf
```

Add the shared dependency and direct crate dependencies:

```toml
# Cargo.toml [workspace.dependencies]
allsorts = { version = "0.17.0", default-features = false, features = ["flate2_rust", "outline"] }
```

```toml
# crates/fontmin_otf/Cargo.toml [dependencies]
allsorts = { workspace = true }
ttf-parser = { workspace = true }
```

Expose the fixture next to `ROBOTO`:

```rust
pub const SOURCE_SANS_3_REGULAR_CFF: &[u8] =
    include_bytes!("../../../fixtures/fonts/otf/source-sans-3-regular.otf");
```

- [x] **Step 4: Run fixture and dependency checks**

Run:

```bash
rtk shasum -a 256 -c fixtures/fonts/otf/source-sans-3-regular.otf.sha256
rtk cargo test -p fontmin_testing exposes_static_cff_fixture
rtk cargo check -p fontmin_otf
```

Expected: all commands exit `0`; the fixture test proves `OTTO` and `CFF ` are
present, and Cargo resolves `allsorts` 0.17.0.

- [x] **Step 5: Commit all local changes**

```bash
rtk git add -A
rtk git commit -m "test: add static cff fixture"
```

### Task 2: Record CFF Paths and Serialize Bounded Quadratic `glyf` Outlines

**Files:**

- Create: `crates/fontmin_otf/src/outline.rs`
- Create: `crates/fontmin_otf/src/glyf.rs`
- Modify: `crates/fontmin_otf/src/lib.rs`

**Interfaces:**

- Consumes: `allsorts::outline::{OutlineBuilder, OutlineSink}` and `GlyphPath` from this task.
- Produces: complete simple-glyph bytes, long-format `loca`, and replacement outline tables for one glyph path per source glyph ID.

- [x] **Step 1: Write failing geometry and glyph-encoding tests**

Add `mod glyf;`, `mod outline;`, and `mod sfnt;` at module scope in
`crates/fontmin_otf/src/lib.rs`. Add these unit tests in the respective modules:

```rust
#[test]
fn cubic_subdivision_keeps_the_analytic_error_within_rounding_budget() {
    let pieces = cubic_to_quadratics(
        Point { x: 0.0, y: 0.0 },
        Point { x: 0.0, y: 1000.0 },
        Point { x: 1000.0, y: 1000.0 },
        Point { x: 1000.0, y: 0.0 },
    );

    assert!(pieces.len() > 1);
    assert!(pieces.iter().all(|piece| {
        piece.control.x.is_finite() && piece.to.y.is_finite()
    }));
    assert!(max_sampled_deviation(
        Point { x: 0.0, y: 0.0 },
        Point { x: 0.0, y: 1000.0 },
        Point { x: 1000.0, y: 1000.0 },
        Point { x: 1000.0, y: 0.0 },
        &pieces,
        4096,
    ) <= 1.0);
}
```

```rust
#[test]
fn serializes_simple_contours_as_long_loca_glyphs() {
    let glyphs = vec![GlyphPath {
        contours: vec![Contour {
            start: Point { x: 0.0, y: 0.0 },
            segments: vec![
                Segment::Line(Point { x: 100.0, y: 0.0 }),
                Segment::Quadratic {
                    control: Point { x: 100.0, y: 100.0 },
                    to: Point { x: 0.0, y: 100.0 },
                },
            ],
        }],
    }];
    let encoded = encode_glyf_and_loca(&glyphs).unwrap();

    assert_eq!(
        u32::from_be_bytes(encoded.loca[4..8].try_into().unwrap()),
        encoded.glyf.len() as u32,
    );
    assert_eq!(encoded.max_contours, 1);
    assert_eq!(encoded.max_points, 4);
}
```

- [x] **Step 2: Run focused tests to verify they fail**

Run:

```bash
rtk cargo test -p fontmin_otf cubic_subdivision_keeps_the_analytic_error_within_rounding_budget
rtk cargo test -p fontmin_otf serializes_simple_contours_as_long_loca_glyphs
```

Expected: FAIL because the new modules and helpers do not yet exist.

- [x] **Step 3: Implement the CFF recording sink and recursive approximation**

In `outline.rs`, implement `OutlineSink` for a recorder that:

```rust
fn move_to(&mut self, to: Vector2F) {
    self.finish_contour();
    self.current = Some(Contour { start: point(to), segments: Vec::new() });
}

fn line_to(&mut self, to: Vector2F) {
    self.push_line(point(to));
}

fn quadratic_curve_to(&mut self, control: Vector2F, to: Vector2F) {
    self.push_quadratic(Point {
        x: f64::from(control.x()),
        y: f64::from(control.y()),
    }, Point {
        x: f64::from(to.x()),
        y: f64::from(to.y()),
    });
}

fn cubic_curve_to(&mut self, controls: LineSegment2F, to: Vector2F) {
    let control1 = Point {
        x: f64::from(controls.from_x()),
        y: f64::from(controls.from_y()),
    };
    let control2 = Point {
        x: f64::from(controls.to_x()),
        y: f64::from(controls.to_y()),
    };
    let to = Point {
        x: f64::from(to.x()),
        y: f64::from(to.y()),
    };
    self.push_cubic(control1, control2, to);
}

fn push_quadratic(&mut self, control: Point, to: Point) {
    let Some(contour) = self.current.as_mut() else {
        self.error = Some(FontminError::invalid_font("CFF path segment precedes moveTo"));
        return;
    };
    contour.segments.push(Segment::Quadratic {
        control,
        to,
    });
}

fn push_cubic(&mut self, control1: Point, control2: Point, to: Point) {
    let Some(from) = self.current_point() else {
        self.error = Some(FontminError::invalid_font("CFF path segment precedes moveTo"));
        return;
    };
    let pieces = cubic_to_quadratics(from, control1, control2, to);
    let Some(contour) = self.current.as_mut() else {
        return;
    };
    contour.segments.extend(
        pieces.into_iter().map(|piece| Segment::Quadratic {
            control: piece.control,
            to: piece.to,
        }),
    );
}
```

`move_to`, `line_to`, `quadratic_curve_to`, `cubic_curve_to`, and `close` are
the infallible `OutlineSink` callbacks. The recorder stores a first
`Option<FontminError>` and `finish(self) -> Result<GlyphPath>` returns it after
`CFFOutlines::visit` succeeds.

Use De Casteljau splitting at `t = 0.5`. For one candidate quadratic compute:

```rust
const APPROXIMATION_LIMIT: f64 = 0.292_893_218_813_452_4; // 1 - sqrt(0.5)

let control = Point {
    x: (-from.x + 3.0 * control1.x + 3.0 * control2.x - to.x) / 4.0,
    y: (-from.y + 3.0 * control1.y + 3.0 * control2.y - to.y) / 4.0,
};

let d = Point {
    x: from.x - 3.0 * control1.x + 3.0 * control2.x - to.x,
    y: from.y - 3.0 * control1.y + 3.0 * control2.y - to.y,
};
let error = 3.0_f64.sqrt() * d.length() / 36.0;
```

Emit a single quadratic only when `error <= APPROXIMATION_LIMIT`; otherwise
split the cubic and recurse. Carry each segment's `t_start` and `t_end`; give
the left and right children the intervals `[t_start, middle_t]` and
`[middle_t, t_end]`, where `middle_t = (t_start + t_end) / 2.0`. Use the exact
point midpoint sequence below before recursing on `(from, p01, p012, middle)`
and `(middle, p123, p23, to)`:

```rust
let p01 = midpoint(from, control1);
let p12 = midpoint(control1, control2);
let p23 = midpoint(control2, to);
let p012 = midpoint(p01, p12);
let p123 = midpoint(p12, p23);
let middle = midpoint(p012, p123);
```

Reject non-finite coordinates and recursion deeper than 32 with
`FontminError::invalid_font`. The test-only `max_sampled_deviation` evaluates
the original cubic and its corresponding quadratic piece at 4,097 evenly spaced
values of `t`, after applying the same integer rounding used by the serializer.

- [x] **Step 4: Implement TrueType simple-glyph encoding**

In `glyf.rs`, convert each contour into an ordered point list: append the
on-curve start point, append each line endpoint as on-curve, append each
quadratic control as off-curve and its endpoint as on-curve. Reject a contour
with more than `u16::MAX` points or any rounded coordinate outside `i16`.

Encode each simple glyph exactly in this order:

```text
i16 numberOfContours
i16 xMin, yMin, xMax, yMax
u16 endPtsOfContours[numberOfContours]
u16 instructionLength = 0
u8 flags[pointCount]             // only bit 0 is used for on-curve points
i16 xDelta[pointCount]
i16 yDelta[pointCount]
```

Pad each glyph byte vector to an even length, append its starting byte offset
to `loca`, and use long `loca` offsets (`u32`, `head.indexToLocFormat = 1`).
Build `maxp` version 1.0 with the actual maximum simple point and contour
counts, zero instructions/storage/function values, `maxZones = 1`, and zero
composite values. Parse each source `hmtx` metric, preserve its advance width
and left-side bearing exactly, emit one `{ advanceWidth, lsb }` record per
source glyph, and set `hhea.numberOfHMetrics = numGlyphs`; calculate
`advanceWidthMax`, `minLeftSideBearing`, `minRightSideBearing`, and
`xMaxExtent` from the converted bounds and metrics. Update `head` global bounds
and clear `checkSumAdjustment` for `fontmin_ttf::write_ttf` to fill.

- [x] **Step 5: Run focused tests and formatting**

Run:

```bash
rtk cargo test -p fontmin_otf cubic_subdivision_keeps_the_analytic_error_within_rounding_budget
rtk cargo test -p fontmin_otf serializes_simple_contours_as_long_loca_glyphs
rtk cargo fmt --all --check
```

Expected: all commands exit `0`.

- [x] **Step 6: Commit all local changes**

```bash
rtk git add -A
rtk git commit -m "feat: encode cff outlines as glyf"
```

### Task 3: Validate Static CFF Sources and Complete Core Conversion

**Files:**

- Modify: `crates/fontmin_diagnostics/src/lib.rs`
- Create: `crates/fontmin_otf/src/sfnt.rs`
- Modify: `crates/fontmin_otf/src/lib.rs`
- Modify: `crates/fontmin_otf/Cargo.toml`

**Interfaces:**

- Consumes: `read_sfnt_table_directory`, `CFFOutlines`, `GlyphPath`, and `build_truetype_outline_tables`.
- Produces: `otf_to_ttf(input, options)` support for both the existing glyf-backed wrapper and real static CFF sources.

- [x] **Step 1: Write failing real-conversion and rejection tests**

Replace the current single CFF rejection expectation in
`crates/fontmin_otf/src/lib.rs` with these tests:

```rust
#[test]
fn converts_static_cff_otf_to_valid_ttf() {
    let input = fontmin_testing::SOURCE_SANS_3_REGULAR_CFF;
    let output = otf_to_ttf(input, &Otf2TtfOptions::default()).unwrap();
    let source = inspect_otf(input).unwrap();
    let converted = fontmin_ttf::inspect_ttf(&output).unwrap();
    let face = ttf_parser::Face::parse(&output, 0).unwrap();

    assert_eq!(&output[..4], &[0, 1, 0, 0]);
    assert_eq!(converted.glyph_count, source.glyph_count);
    assert_eq!(converted.family_name, source.family_name);
    assert!(face.tables().glyf.is_some());
    assert!(!converted.tables.iter().any(|tag| tag == "CFF "));
    assert!(converted.tables.iter().any(|tag| tag == "GSUB"));
    assert!(converted.tables.iter().any(|tag| tag == "GPOS"));
}

#[test]
fn rejects_color_table_static_cff_sources() {
    let mut input = fontmin_testing::SOURCE_SANS_3_REGULAR_CFF.to_vec();
    input[12..16].copy_from_slice(b"COLR");

    let error = otf_to_ttf(&input, &Otf2TtfOptions::default()).unwrap_err();
    assert_eq!(error.kind(), FontminErrorKind::UnsupportedFormat);
}
```

Add an output checksum assertion with the existing writer API:

```rust
assert_eq!(fontmin_ttf::calculate_table_checksum(&output), 0xB1B0_AFBA);
```

- [x] **Step 2: Run the tests to verify the current CFF behavior fails**

Run:

```bash
rtk cargo test -p fontmin_otf converts_static_cff_otf_to_valid_ttf
rtk cargo test -p fontmin_otf rejects_color_table_static_cff_sources
```

Expected: the conversion test FAILS with `unsupported font format: otf to ttf`.

- [x] **Step 3: Implement explicit source validation and table policy**

Implement `read_static_cff_source` in `sfnt.rs` using
`fontmin_ttf::read_sfnt_table_directory` and a `BTreeMap<&str, &[u8]>`.
Require `cmap`, `head`, `hhea`, `hmtx`, `maxp`, `name`, `OS/2`, `post`, and
`CFF `; reject duplicate/missing/truncated data using `InvalidFont`. In addition
to the shared directory reader, reject a table whose offset is not four-byte
aligned, starts inside the sfnt directory, or overlaps another non-empty table.

Use this exact table policy:

```rust
const PRESERVED_TABLES: &[&str] = &[
    "cmap", "name", "OS/2", "post", "kern", "GDEF", "GSUB", "GPOS", "BASE", "JSTF",
];
const REJECTED_TABLES: &[&str] = &[
    "CFF2", "COLR", "CPAL", "CBDT", "CBLC", "sbix", "SVG ",
];
```

Reject `glyf` and `loca` together with any `CFF2` table, because this path is
for CFF source outlines only. `output_tables` must insert the six rebuilt tables
and copy only the names in `PRESERVED_TABLES` that exist in the source. It must
not copy `CFF `, `VORG`, `DSIG`, `FFTM`, `META`, or any variation table.

Add a small constructor to diagnostics:

```rust
pub fn convert_failed(message: impl Into<String>) -> Self {
    Self::ConvertFailed {
        message: message.into(),
    }
}
```

- [x] **Step 4: Orchestrate CFF interpretation and validate output**

In `otf_to_ttf`, retain the existing fast path for `OTTO` wrappers containing
both `glyf` and `loca` and no CFF table. Otherwise:

```rust
let source = sfnt::read_static_cff_source(input)?;
let cff_data = source.tables["CFF "];
let cff = ReadScope::new(cff_data)
    .read::<CFF<'_>>()
    .map_err(|error| FontminError::invalid_font(format!("invalid CFF table: {error}"))?;
if cff.fonts.len() != 1 {
    return Err(FontminError::invalid_font("OpenType CFF must contain exactly one font"));
}
let font = cff.fonts.first().unwrap();

if font.char_strings_index.len() != usize::from(source.num_glyphs) {
    return Err(FontminError::invalid_font("CFF glyph count does not match maxp"));
}

let mut outlines = CFFOutlines { table: &cff };
let glyphs = (0..source.num_glyphs)
    .map(|glyph_id| record_glyph(&mut outlines, glyph_id))
    .collect::<Result<Vec<_>>>()?;
let rebuilt = glyf::build_truetype_outline_tables(&source, &glyphs)?;
let output = fontmin_ttf::write_ttf(&OwnedTtfFont {
    tables: sfnt::output_tables(&source, rebuilt),
})?;
validate_output(&output)?;
Ok(output)
```

`record_glyph` calls `CFFOutlines::visit(glyph_id, None, &mut recorder)` and
maps its parser errors to `InvalidFont`. `validate_output` requires
`fontmin_ttf::read_ttf(output)`, `fontmin_ttf::inspect_ttf(output)`, a total
checksum of `0xB1B0_AFBA`, and `ttf_parser::Face::parse(output, 0)`; map any
failure to `ConvertFailed`.

The conversion test must also compare default and hint-preservation options:

```rust
assert_eq!(
    output,
    otf_to_ttf(input, &Otf2TtfOptions { preserve_hinting: true }).unwrap(),
);
```

- [x] **Step 5: Run the core crate test suite**

Run:

```bash
rtk cargo test -p fontmin_otf
rtk cargo test -p fontmin_ttf
rtk cargo clippy -p fontmin_otf --all-targets --all-features
```

Expected: all tests and Clippy exit `0`; the real CFF fixture is emitted as a
checksum-valid TrueType font with preserved glyph count, names, GSUB, and GPOS.

- [x] **Step 6: Commit all local changes**

```bash
rtk git add -A
rtk git commit -m "feat: convert static cff otf to ttf"
```

### Task 4: Prove Existing CLI, N-API, and TypeScript Entrypoints Convert Real CFF

**Files:**

- Modify: `apps/fontmin/tests/cli.rs`
- Modify: `napi/fontmin/test/api.test.ts`
- Modify: `packages/fontmin/test/api.test.ts`
- Modify: `README.md`
- Modify: `docs/api/node.md`
- Modify: `docs/guide/cli.md`
- Modify: `docs/zh/api/node.md`
- Modify: `docs/zh/guide/cli.md`

**Interfaces:**

- Consumes: the static CFF fixture and unchanged `fontmin::convert`, `otfToTtf`, and `fontmin-rs convert` public APIs.
- Produces: end-to-end evidence that every current user-facing entry point writes or returns the static CFF conversion result.

- [x] **Step 1: Add failing end-to-end tests**

Declare the fixture beside existing TTF fixture paths in both Vitest files:

```ts
const cffFixture = resolve(
  currentDir,
  '../../../fixtures/fonts/otf/source-sans-3-regular.otf',
)
```

Add this identical behavioral test in both files, calling the locally imported
`otfToTtf` function:

```ts
it('converts a real static CFF OTF to TTF', () => {
  const output = otfToTtf(readFileSync(cffFixture))
  const info = inspectFont(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Source Sans 3')
  expect(info.metadata.tables).not.toContain('CFF ')
  expect(info.metadata.tables).toContain('glyf')
  expect(info.metadata.tables).toContain('GSUB')
  expect(info.metadata.tables).toContain('GPOS')
})
```

Add a CLI test modeled on the existing `convert_command_writes_requested_format`
test. Copy the CFF fixture into a temporary input path, invoke:

```rust
Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
    .arg("convert")
    .arg(&input)
    .arg("--format")
    .arg("ttf")
    .arg("--output")
    .arg(&output)
```

Then assert success, a TrueType flavor, `fontmin::inspect(&bytes)?.format ==
FontFormat::Ttf`, and that `fontmin_ttf::read_ttf(&bytes)` has `glyf` but no
`CFF ` table.

- [x] **Step 2: Run the new end-to-end tests against the completed core**

Run:

```bash
rtk cargo test -p fontmin_app convert_command_converts_static_cff_otf_to_ttf
rtk pnpm --filter @fontmin-rs/binding test -- api.test.ts
rtk pnpm --filter fontmin-rs test -- api.test.ts
```

Expected: all three CFF-specific tests pass because Task 3 has completed the
core conversion that these existing public wrappers call.

- [x] **Step 3: Update user-facing conversion documentation**

In both English and Chinese README/API/CLI pages, state only the delivered
behavior:

```markdown
`otfToTtf` and `fontmin-rs convert input.otf --format ttf` convert static CFF
OpenType fonts into static TrueType `glyf` fonts. Glyph IDs, cmap mappings,
metrics, names, and supported OpenType layout tables are retained. Type 2
hinting is discarded. CFF2 instance conversion is documented by the Stage 2
plan in `docs/superpowers/plans/2026-07-10-cff2-otf-to-ttf.md`.
```

Use Chinese prose in `docs/zh/` with the same limitations. Do not document
variation-coordinate options in this stage.

- [x] **Step 4: Run all user-facing regressions**

Run:

```bash
rtk pnpm run build:debug
rtk cargo test -p fontmin_app
rtk pnpm --filter @fontmin-rs/binding test
rtk pnpm --filter fontmin-rs test
pnpm typecheck
rtk cargo fmt --all --check
rtk oxfmt --check .
```

Expected: every command exits `0`; the N-API and package tests see the native
binding built from the completed core conversion.

- [x] **Step 5: Commit all local changes**

```bash
rtk git add -A
rtk git commit -m "test: cover static cff conversion APIs"
```

### Task 5: Full Workspace Verification and Completion Audit

**Files:**

- Modify only files required by fixes discovered during verification.

**Interfaces:**

- Consumes: all completed static CFF conversion code and regression tests.
- Produces: a clean, fully checked workspace at a commit containing every local change.

- [x] **Step 1: Run the full repository gate**

Run:

```bash
rtk pnpm run check
rtk git diff --check
rtk git status --short
```

Expected: `pnpm run check` and `git diff --check` exit `0`. Review every path
reported by `git status --short`; do not revert user changes.

- [x] **Step 2: Verify the concrete delivery requirements**

Run:

```bash
rtk cargo test -p fontmin_otf converts_static_cff_otf_to_valid_ttf
rtk cargo test -p fontmin_app convert_command_converts_static_cff_otf_to_ttf
rtk pnpm --filter @fontmin-rs/binding test -- api.test.ts
rtk pnpm --filter fontmin-rs test -- api.test.ts
rtk rg -n 'CFF2 variable OTF conversion is not yet available|CFF2.*not yet' README.md docs docs/zh
```

Expected: the four conversion test commands exit `0`, and documentation has no
claim that CFF2 coordinate conversion is complete.

- [x] **Step 3: No verification fixes were needed**

Run:

```bash
rtk git status --short
```

If output is non-empty after a necessary fix, run:

```bash
rtk git add -A
rtk git commit -m "fix: finalize static cff conversion"
```

If output is empty, do not create an empty commit.

## Plan Self-Review

**Spec coverage:** Stage 1 static CFF support is covered by Tasks 1 through 3.
The required TrueType flavor, `glyf`/`loca`, stable glyph IDs, preserved names,
cmap, metrics, eligible layout tables, CFF removal, bounded approximation,
checksum regeneration, input validation, and error categories all map to Tasks
2 and 3. CLI, N-API, and TypeScript behavior is covered by Task 4. The full
workspace gate and requirement-level evidence are covered by Task 5. CFF2
instantiation, coordinates, and variation-table policy are intentionally out
of this standalone Stage 1 plan and remain in the approved design for the next
plan.

**Placeholder scan:** The plan contains no unfilled code step, deferred work,
or unspecified validation instruction. Every modification has an exact path,
tests name the expected behavior, and every commit stages all local changes.

**Type consistency:** `GlyphPath` is produced by `outline.rs` and consumed by
`glyf.rs`; `TrueTypeOutlineTables` is produced by `glyf.rs` and consumed by
`sfnt::output_tables`; `StaticCffSource` is produced by `sfnt.rs` and consumed
by both modules; public `otf_to_ttf` remains unchanged.
