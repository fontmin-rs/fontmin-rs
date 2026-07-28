# Deprecation policy

This policy applies to the CLI, Node and WASM APIs, configuration files,
diagnostic codes, generated file naming, and supported runtime behavior.

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
