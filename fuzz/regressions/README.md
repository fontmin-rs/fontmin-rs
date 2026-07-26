# Permanent fuzz regressions

`public_api/manifest.json` inventories minimized cargo-fuzz discoveries. Each
binary begins with the operation selector consumed by
`fuzz/fuzz_targets/public_api.rs`; the remaining bytes are the untrusted input.

The scheduled workflow creates a reviewable pull request containing a
content-addressed regression when fuzzing fails on a trusted repository event.
It never pushes directly to `main`. The regression remains a seed until the
underlying bug is fixed, then its payload should also be promoted to
`fixtures/malformed` with provenance, SHA-256, and a stable expected diagnostic.
