# CFF/CFF2 Web Pipeline Design

## Goal

Make static CFF and variable CFF2 OpenType fonts first-class inputs to the
standard Web delivery path. `modernWeb()` and `fontmin-rs build` must normalize
an OTF input to static TrueType before subsetting and emitting Web fonts.

## Context

The OTF converter already handles static CFF and CFF2 instances, including
explicit user-space coordinates. The compatibility preset invokes that
converter, but the usual `modernWeb()` preset and the Rust build engine do not.
An OTF therefore reaches TTF-only subset/output stages without a usable asset.

## Decisions

1. The Rust build engine installs an `Otf2TtfPlugin` as a pre-transform with
   `clone: false`. It is a no-op for non-OTF inputs and turns every supported
   OTF build input into its canonical TTF asset before glyph subsetting, slices,
   and output conversion.
2. `FontminConfig` gains an `otf` section with `preserveHinting` and
   `variationCoordinates`. The pipeline maps it directly to `Otf2TtfOptions`.
3. `fontmin-rs build --variation TAG=VALUE` accepts repeated CFF2 coordinates.
   Flag values override matching `otf.variationCoordinates` entries from a
   config file; unrelated configured coordinates remain intact.
4. Node `modernWeb()` prepends `otf2ttf({ ...options, clone: false })`, and its
   options extend `Otf2TtfOptions`. Browser `modernWeb()` follows the same
   order and replacement semantics.
5. The normalized TTF remains in the in-memory output set, just as a TTF input
   does today. The source OTF is not emitted by the standard Web pipeline.

## Non-goals

- Retaining CFF/CFF2 data, variation tables, or Type 2 hinting in output.
- Supporting color-font tables that the existing converter rejects.
- Changing `fontminCompatPreset()` clone semantics.

## Verification

- CLI integration tests cover static CFF and a non-default CFF2 instance via
  `build --preset modern-web`, checking static TTF/WOFF/WOFF2 output.
- Node and browser tests cover the same `modernWeb()` conversion order and
  verify that no OTF source remains in the result.
- Config tests cover the new `otf` values and CLI override behavior.
