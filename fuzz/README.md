# Fuzzing

The `public_api` target exercises every binary font boundary exposed by the
Rust facade. The first input byte selects the operation; the remaining bytes
are passed to that public API unchanged.

Prepare deterministic seeds and run a bounded local smoke check:

```shell
pnpm run fuzz:corpus
RUSTC="$(rustup which --toolchain nightly rustc)" \
  cargo fuzz run public_api --sanitizer address -- \
  -runs=256 -max_len=1048576 -timeout=10
```

GitHub Actions runs the same AddressSanitizer target briefly when relevant
files change and for five minutes on the weekly schedule. Minimize every crash
with `cargo fuzz tmin`, then add the smallest reproducer to
`fixtures/malformed` and its stable public-boundary assertion to the Native/WASM
conformance suite.
