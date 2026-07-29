# sfnt and TrueType context

`src/sfnt.rs` is the canonical sfnt table-directory implementation for this
workspace.

Format-specific crates may parse or build their own table payloads, but they
must delegate final serialization to `write_ttf` or `write_sfnt`. This keeps
the following rules in one place:

- supported sfnt flavor signatures;
- table-tag validation and duplicate rejection;
- table ordering and four-byte alignment;
- table-directory search parameters;
- per-table checksums;
- `head.checkSumAdjustment`.

`fontmin_woff`, `fontmin_woff2`, `fontmin_otf`, and `fontmin_svg` may depend on
this crate for those primitives. Do not add another local sfnt writer to a
format crate.
