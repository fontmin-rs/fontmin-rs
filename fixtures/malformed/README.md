# Malformed font corpus

These deliberately invalid inputs are shared by public-boundary regression
tests and bounded fuzz targets. Each case is intentionally minimal so a failure
points to one parser boundary rather than an unrelated table.

Binary fixtures may use a `.hex` suffix when a compact, reviewable synthetic
font is clearer than an opaque binary diff. Corpus and runtime tests decode
these files before invoking the public API.

The manifest records SHA-256, immutable origin, license disposition, generator,
public operation, and the stable diagnostic expected from every runtime.
Synthetic cases authored by fontmin-rs are covered by the repository MIT
license; third-party reproducers must retain their original license metadata.
Each fixture also has a companion `.sha256` file.

New parser failures discovered by fuzzing must be minimized, added here, and
covered by a deterministic regression test. Run `pnpm run fixtures:check`
after any corpus change; undeclared, missing, reordered, or corrupted fixtures
fail the check.
