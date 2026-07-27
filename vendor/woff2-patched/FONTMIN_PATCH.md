# fontmin-rs patch notes

This directory contains `woff2-patched` 0.4.0 from crates.io
(`cb9e0d17aedee2afac15749407e107edce8485f57b7ad9fb929f0bbdf2c28f24`).

The local patch decodes transformed `glyf` triplet coordinates through a
wider signed integer before converting to `i16`. This preserves the valid
`-32768` coordinate delta while rejecting positive or cumulative coordinate
values that do not fit in the TrueType representation.

The permanent `public_api` regression from GitHub Actions run 30243677446
covers the minimum coordinate delta through `fontmin::ttf_to_woff2` followed
by `fontmin::woff2_to_ttf`.

Remove this override after an upstream release contains equivalent checked
coordinate handling and the repository regression corpus passes against that
release.
