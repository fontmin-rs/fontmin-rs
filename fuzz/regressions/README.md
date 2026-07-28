# Permanent fuzz regressions

Each target directory inventories minimized cargo-fuzz discoveries in its
`manifest.json`. Every binary begins with the target's operation selector; the
remaining bytes are the untrusted input. `public_api` retains regressions from
the original broad target, while `parsers`, `converters`, `configuration`, and
`output_naming` keep failures attributed to one responsibility.

The scheduled workflow creates a reviewable pull request containing a
content-addressed regression when fuzzing fails on a trusted repository event.
It never pushes directly to `main`. The regression remains a seed until the
underlying bug is fixed, then its payload should also be promoted to
`fixtures/malformed` with provenance, SHA-256, and a stable expected diagnostic.
