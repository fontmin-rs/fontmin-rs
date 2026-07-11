# CFF/CFF2 OTF To TTF Conversion Design

## Goal

Add a Rust-native `otfToTtf` conversion path for static CFF OTF fonts and CFF2
variable fonts instantiated at a selected design-space location. The output is
a static TrueType `glyf` font that remains loadable by the existing CLI, N-API,
and TypeScript package APIs.

This feature deliberately does not claim lossless hint conversion. Type 2 hint
operators are dropped because they have no equivalent TrueType instruction
representation. CFF2 output is always instantiated to a static font; it does
not preserve a variable `gvar` representation.

## Scope

### Supported input

- Static OpenType CFF fonts with Type 2 charstrings.
- OpenType CFF2 variable fonts at their default instance.
- OpenType CFF2 variable fonts at caller-provided axis coordinates.
- Name-keyed and CID-keyed CFF fonts, with every source glyph ID retained.

### Output guarantees

- The result has a TrueType sfnt flavor and valid `glyf` and `loca` tables.
- Glyph IDs, cmap mappings, horizontal metrics, names, and eligible layout
  tables are retained.
- Cubic CFF outlines are converted to quadratic outlines with no more than one
  source-font unit of geometric deviation, including coordinate rounding.
- Table directory order and `checkSumAdjustment` are regenerated through the
  existing `fontmin_ttf` writer.
- CFF/CFF2 source tables and CFF-specific private data are removed.
- CFF2 variation tables are removed after instantiation, producing a static
  TTF.

### Explicit non-goals

- Preserving Type 2 hints or synthesizing equivalent TrueType hint programs.
- Retaining variable-font behavior after CFF2 conversion.
- Converting color, bitmap, SVG, or other non-glyf outline inputs.
- Retaining variation-dependent layout data after CFF2 instantiation.

## Public API

Rust extends `Otf2TtfOptions` with optional design-space coordinates:

```rust
pub struct Otf2TtfOptions {
    pub preserve_hinting: bool,
    pub variation_coordinates: BTreeMap<String, f32>,
}
```

`preserve_hinting` remains accepted for compatibility, but CFF/CFF2 conversion
cannot retain Type 2 hints. Its use does not change the output and is
documented accordingly.

TypeScript exposes the matching option:

```ts
otfToTtf(input, {
  variationCoordinates: { wght: 700, wdth: 90 },
})
```

The CLI accepts repeated `--variation TAG=VALUE` arguments. With no supplied
coordinates, a CFF2 font is instantiated at its default axis location.

## Architecture

`fontmin_otf` owns the conversion orchestration. It uses `allsorts` as the
input parser, Type 2 charstring interpreter, CFF/CFF2 outline provider, and
variable-font instantiation layer. `fontmin_otf` then owns the conversion-only
work that needs stable project control:

1. Read and validate the CFF/CFF2 sfnt input.
2. Resolve the default or requested CFF2 instance.
3. Interpret glyph outlines while retaining original glyph IDs and metrics.
4. Approximate cubic segments with bounded-error quadratic segments.
5. Rebuild `glyf`, `loca`, `head`, `maxp`, `hhea`, and `hmtx`.
6. Apply the table-preservation policy for cmap, naming, and OpenType layout.
7. Write the result through `fontmin_ttf` to sort records and recalculate
   checksums.

The conversion retains `cmap`, `name`, `OS/2`, `post`, `kern`, `GDEF`, `GSUB`,
`GPOS`, `BASE`, and `JSTF` unchanged because every glyph ID is retained. For
CFF2 input, it rejects GSUB/GPOS tables containing `FeatureVariations` or a
variation-device reference. If GDEF contains an `ItemVariationStore`, the
whole GDEF table is dropped because its static mark data cannot remain valid
after instantiation. All other optional tables are dropped in both delivery
stages.

## Errors

- Unknown variation axis, invalid numeric value, or coordinate outside its
  declared range: `InvalidFont`, naming the axis and value.
- Unsupported Type 2 operation, color/bitmap/SVG outline, or non-removable
  variation-dependent layout table: `UnsupportedFormat`.
- Outline approximation beyond tolerance or invalid output verification:
  `ConvertFailed`.

No error path writes a partial output file through the library API. CLI output
is only written after conversion succeeds.

## Delivery Plan

### Stage 1: Static CFF

Support static CFF OTF to static TTF conversion with glyph outlines, cmap,
metrics, naming, eligible layout tables, sfnt validation, and Rust fixture
tests.

### Stage 2: CFF2 instances

Support default and explicit CFF2 instance coordinates, remove variation
tables from the resulting static TTF, and expose the option through the CLI,
N-API, TypeScript API, and end-to-end fixture tests.

## Verification

Fixtures must cover static CFF, default CFF2, explicitly-instantiated CFF2,
and a font with GSUB/GPOS. Every successful output is reread by the TTF parser
and checked for:

- TrueType flavor and valid sfnt checksum adjustment.
- cmap and glyph-count consistency.
- Family and PostScript naming preservation.
- Horizontal advance values at the selected instance.
- Expected preservation of eligible layout tables and removal of variation
  tables.

Negative tests cover invalid axis tags and values, unsupported CFF operations,
variation-dependent layout tables, and invalid source data. CLI, N-API, and
TypeScript tests prove that the same coordinates reach the native converter.

## Dependencies

The feature adds `allsorts` for CFF/CFF2 parsing and variable instantiation.
Existing `fontmin_ttf` writing and validation utilities remain the only sfnt
writer used by this feature.
