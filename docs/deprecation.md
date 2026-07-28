# Deprecation policy

This policy applies to the CLI, Node and WASM APIs, configuration files,
diagnostic codes, generated file naming, and supported runtime behavior.

## 1.0 readiness audit

No API is eligible for removal in 1.0. The `0.x` line did not announce and
carry an API through a complete deprecation window, so `1.0` retains every
public item in `contracts/public-api.json`.

The Fontmin-compatible default export, the `glyph({ hinting })` alias for
`preserveHinting`, and `ttf2woff2({ fallback })` runtime selection remain
compatibility paths. They are not removed or silently changed. Future
deprecation must start with the replacement-and-warning sequence below.

The machine-readable decision is recorded in
[`contracts/support.json`](https://github.com/fontmin-rs/fontmin-rs/blob/main/contracts/support.json).

## Before 1.0

The stable `0.x` public contract remains governed even though SemVer permits a
minor release to make breaking changes. Every breaking change must:

- be called out under `Changed` or `Removed` in `CHANGELOG.md`;
- include a concrete migration step in the relevant guide;
- preserve an alias or compatibility path when doing so does not create a
  correctness or security risk.

The stable `0.1` contract is frozen. A breaking change starts a new `0.x`
minor line, uses its own prerelease validation cycle, and cannot ship in a
patch release.

## After 1.0

Public behavior is not removed in a patch release. A planned removal follows
this sequence:

1. Introduce the replacement and document it.
2. Mark the old behavior deprecated in types and API documentation.
3. Emit a non-fatal warning where the CLI or configuration loader can identify
   the deprecated use reliably.
4. Keep both paths for at least one minor release.
5. Remove the old path only in a SemVer-major release.

Security or data-corruption fixes may bypass the compatibility period. Such an
exception requires a security advisory or prominent changelog rationale and a
migration path when one exists.

## Diagnostics

Deprecation warnings go to stderr and must not corrupt JSON or binary stdout.
Library calls do not write warnings implicitly; TypeScript annotations,
documentation, and returned diagnostics carry the notice instead. Tests must
cover the replacement and the compatibility path until removal.
