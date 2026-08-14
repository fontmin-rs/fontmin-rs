# HarfBuzz subset WASM

`harfbuzz-subset.wasm` is the prebuilt subset module distributed by
[`harfbuzzjs@1.6.0`](https://www.npmjs.com/package/harfbuzzjs/v/1.6.0).
That release pins HarfBuzz commit
[`4de187dd0a915d13c976fa8bd474c084229f3aab`](https://github.com/harfbuzz/harfbuzz/tree/4de187dd0a915d13c976fa8bd474c084229f3aab),
which identifies itself as HarfBuzz 14.3.0.

The binary is vendored so Rust, N-API, and browser WASM can use the same
variable-font subset semantics without a system HarfBuzz installation. It has
no WebAssembly imports and exposes the allocation, blob, face, subset-input,
axis, and subset functions used by `fontmin_subset`.

## Integrity

- npm tarball SHA-256:
  `c1e2c37480396d8d8721f909f2a7fce42153bbbc26cdd712c611d498311a2088`
- `harfbuzz-subset.wasm` SHA-256:
  `e3bf5ad5841bfdcc37878dff5330a13eea071b5860471d9d6515e31506e4ff96`

The upstream MIT license is reproduced in `LICENSE`.
