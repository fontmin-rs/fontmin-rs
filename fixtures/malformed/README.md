# Malformed font corpus

These deliberately invalid inputs are shared by public-boundary regression
tests and bounded fuzz targets. Each case is intentionally minimal so a failure
points to one parser boundary rather than an unrelated table.

Binary fixtures may use a `.hex` suffix when a compact, reviewable synthetic
font is clearer than an opaque binary diff. Corpus and runtime tests decode
these files before invoking the public API.

The manifest records the public operation and stable diagnostic expected from
both the native and WASM runtimes. New parser failures discovered by fuzzing
must be minimized, added here, and covered by a deterministic regression test.
