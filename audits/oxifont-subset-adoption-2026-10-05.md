# oxifont-subset upstream adoption — 2026-10-05

Decision: retain the local `oxifont-subset` override. The public upstream branch
and published crate are unchanged from the base used by the
[eight-patch preparation series](oxifont-subset-upstream-2026-08-31.md).
Owner: fontmin-rs maintainers.

## Observed upstream state

Checked on 2026-10-05 (Asia/Shanghai) using the GitHub and crates.io APIs:

| Surface                          | Observed value                                                   | Primary source                                                                                    |
| -------------------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `master`                         | `8dca5507d127404061f54f1341ac061ff88562df`, committed 2026-08-06 | [Branch commit API](https://api.github.com/repos/cool-japan/oxifont/commits/master)               |
| Latest tag                       | `v0.2.2`, pointing to the same commit                            | [Tags API](https://api.github.com/repos/cool-japan/oxifont/tags)                                  |
| Latest GitHub release            | `v0.2.2`, published 2026-08-06T14:40:03Z                         | [Release](https://github.com/cool-japan/oxifont/releases/tag/v0.2.2)                              |
| Latest, newest, and stable crate | `0.2.2`, published 2026-08-06T14:30:38.478413Z, not yanked       | [Crate API](https://crates.io/api/v1/crates/oxifont-subset)                                       |
| Public pull requests, all states | Empty result                                                     | [Pull requests API](https://api.github.com/repos/cool-japan/oxifont/pulls?state=all&per_page=100) |

The `0.2.2` crate checksum reported by crates.io is
`fa12c3b706d76088020dc49cf3bde491b69b2050942914645ad91c46082b0a1f`.
The [pinned source commit](https://github.com/cool-japan/oxifont/commit/8dca5507d127404061f54f1341ac061ff88562df)
is exactly the base recorded on 2026-08-31, so none of the subsequent prepared
changes has reached this public default branch. There is no newer published
crate to evaluate for adoption. These observations do not establish the state
of private work or communication outside this repository.

## Patch readiness

A fresh shallow clone of the public default branch resolved to the recorded
base commit. Each of the eight checked-in mail patches passed `git apply
--check` and was applied in order in that temporary checkout; the resulting
tree passed `git diff --check`. No commits or remote changes were created.

This check verifies that the prepared series remains applicable. It does not
repeat or extend the compiler, regression, or fuzz-build results recorded in
the [preparation audit](oxifont-subset-upstream-2026-08-31.md#validation).

The local copy also carries subsequent GPOS positioning and GSUB glyph-closure
maintenance changes documented in
[`FONTMIN_PATCH.md`](../vendor/oxifont-subset/FONTMIN_PATCH.md). Those changes are
not part of the historical eight-patch series and must be included in any
upstream replacement comparison. The
[production shaping/rendering gate](../fixtures/production/README.md) checks
the retained glyph sequence, kerning, variable positioning, and rendered
outlines/color layers independently across native and WASM subsets.

## Adoption gate

Recheck when an upstream release changes the available implementation. Before
removing the override:

1. Match the released source against the prepared selection/table controls,
   CFF/CFF2 offset and INDEX safety, CID support, COLR v1 closure/remapping,
   malformed cmap format 12 handling, GPOS positioning, and GSUB glyph closure.
   Review equivalent implementations
   by behavior rather than commit identity alone.
2. Run the fontmin-rs malformed-font, color-font, and native/WASM conformance
   regressions and the production shaping/rendering gate against the actual
   released crate.
3. Audit the manifest and dependency graph, including the locally removed
   production `oxifont-parser` and `ttf-parser` dependencies and the historical
   vendored `rust-version` metadata difference. The fontmin-rs workspace
   currently requires Rust `1.98.0`.
4. Remove the root Cargo override, vendored copy, and retained audit decision
   together only after the replacement passes the applicable checks.

The checked-in patches remain prepared for upstream review; this audit does
not submit them or authorize a fontmin-rs release. Release timing remains a
maintainer decision.
