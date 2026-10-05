# fontmin-rs oxifont-subset patch

This directory starts from `oxifont-subset` 0.2.2 at upstream commit
`8dca5507d127404061f54f1341ac061ff88562df` and contains fontmin-rs changes in
both the manifest and Rust sources.

The source patch adds explicit glyph/table selection controls, safe CFF1 and
CFF2 rewriting, CID-keyed CFF support, COLR v1 paint-graph traversal and GID
remapping, and stricter malformed cmap format 12 validation. Unsupported or
malformed CFF structures return typed errors instead of copying charstrings
whose GIDs no longer match the subset. The CFF structural tests live in
`src/cff/tests.rs`, and cmap parsing lives with the cmap rewriter so upstream
source files remain below its 2,000-line contribution limit.

GPOS rewriting also preserves class-pair kerning by keeping the PairPos format
2 adjustment matrix immediately after its fixed header. SinglePos and PairPos
ValueRecords relocate their Device/VariationIndex offsets with the referenced
data, including the PairSet-relative offsets used by PairPos format 1. These
changes preserve both default and variable-position kerning instead of copying
offsets that point outside a rebuilt subtable. Regression coverage includes
pixel-size Device data and GDEF VariationIndex references in all four
ValueRecord-bearing positioning formats.

Selected GSUB Single, Multiple, Alternate, Ligature, and Extension substitutions
now expand the retained glyph set before color/composite closure and GID
remapping. The pass follows feature/script/language selection, requires every
ligature component, and uses a worklist for chained/cyclic substitutions.
Shared lookup/subtable offsets are deduplicated; expansion beyond one million
rule input/output references returns an error. Contextual dispatch and
FeatureVariations remain outside this closure pass. Public fontmin_subset tests
cover Source Serif 4's unencoded ffi ligature, selection controls, substitution
chains, shared offsets, and the allocation bound.

The manifest still lowers the package `rust-version` declaration from 1.89 to
1.88 and removes the unused production `oxifont-parser` and `ttf-parser`
dependencies. Neither crate is referenced by `src/`; `ttf-parser` remains only
a development dependency in the complete upstream repository. The fontmin-rs
workspace itself currently compiles on Rust 1.98.

The upstream-ready commit mapping and validation evidence are recorded in
`audits/oxifont-subset-upstream-2026-08-31.md` at the repository root.
That historical eight-patch series predates the GSUB/GPOS maintenance fixes above;
those changes must also be included when evaluating an upstream replacement.

Remove this override after an upstream release contains equivalent selection,
CFF/CFF2, CID, COLR v1, cmap safety, GSUB closure, and GPOS positioning behavior, and only when the fontmin-rs
regression corpus passes against it. Any remaining manifest differences must
also be accepted by a dependency-graph audit.
