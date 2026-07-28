# fontmin-rs patch notes

This directory contains `woff2-patched` 0.4.0 from crates.io
(`cb9e0d17aedee2afac15749407e107edce8485f57b7ad9fb929f0bbdf2c28f24`).

The local patch applies transformed `glyf` triplet signs and cumulative
coordinates with explicit 16-bit wrapping. This preserves the full WOFF2
triplet magnitude range, including the valid `-32768` delta, while matching
the bit-level TrueType coordinate representation without debug-build
overflow panics.

The permanent `public_api` regression from GitHub Actions run 30243677446
covers the minimum coordinate delta through `fontmin::ttf_to_woff2` followed
by `fontmin::woff2_to_ttf`.

Remove this override after an upstream release contains equivalent explicit
wrapping semantics and the repository regression corpus passes against that
release.
