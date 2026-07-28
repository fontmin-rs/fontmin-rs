# Fuzzing

The fuzz workspace separates failures by responsibility:

- `parsers` covers inspection, coverage, and wrapped-font decoders.
- `converters` covers subsetting and every supported conversion direction.
- `configuration` covers typed JSON deserialization and pipeline construction.
- `output_naming` covers destination containment and extension validation.
- `public_api` preserves broad facade coverage and the existing regression
  corpus.

Every target reserves the first input byte for its operation. The remaining
bytes are passed to the boundary unchanged. Deterministic corpora combine real
font fixtures, malformed tables, valid and invalid configuration, adversarial
output paths, and all promoted regressions.

Prepare every corpus and run the complete bounded local smoke check:

```shell
pnpm run fuzz:corpus
pnpm run fuzz:smoke
```

GitHub Actions runs all five AddressSanitizer targets in parallel for 30
seconds when relevant files change and for five minutes on the weekly
schedule. A trusted failure is minimized with `cargo fuzz tmin`, recorded under
the target-specific `fuzz/regressions/<target>` directory, and proposed through
a reviewable pull request. Parser and converter crashes should also become
stable malformed-fixture assertions when they reproduce across public runtime
boundaries.
