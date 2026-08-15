# Support policy

This page defines the environments exercised by CI on `main`. The public API
inventory and the machine-readable
[`contracts/support.json`](https://github.com/fontmin-rs/fontmin-rs/blob/main/contracts/support.json)
form the stable `1.1` contract. Every listed environment must keep passing
the same conformance, compatibility, and packaging gates before a release is
published.

## Node.js

| Entry point                   | Supported versions              | Evidence                                                  |
| ----------------------------- | ------------------------------- | --------------------------------------------------------- |
| `fontmin-rs` CLI and Node API | Node.js 22.18, 24, and 26       | Full test matrix on Linux, macOS, and Windows             |
| Native binding packages       | Node-API 8 on the targets below | Native build plus packed-package smoke tests              |
| `@fontmin-rs/wasm` tooling    | Node.js 22.18 or newer          | Typecheck, build, Vitest, and browser package smoke tests |

The published `fontmin-rs` package declares `node >=22.18.0`. Newer Node.js
majors are best-effort until they are added to the CI matrix. Executable
TypeScript, MTS, MJS, and CJS configuration loading also requires Node.js 22.18
or newer.

## Native platforms

The release workflow builds and packages these exact targets:

| Operating system | CPU   | Runtime               |
| ---------------- | ----- | --------------------- |
| macOS            | x64   | Darwin native binding |
| macOS            | arm64 | Darwin native binding |
| Windows          | x64   | MSVC native binding   |
| Windows          | arm64 | MSVC native binding   |
| Linux            | x64   | glibc                 |
| Linux            | x64   | musl                  |
| Linux            | arm64 | glibc                 |
| Linux            | arm64 | musl                  |

Node `runtime: "auto"` uses the native package when one of these artifacts can
load and falls back to WASM when it cannot. `runtime: "native"` remains a hard
requirement and reports a load error instead of silently changing behavior.

## Browser WASM

The browser package is tested in the current Playwright Chromium, Firefox, and
WebKit engines. Its public boundary is asynchronous and memory-only: it has no
filesystem paths, glob expansion, disk cache, CLI, or arbitrary Node.js plugin
hooks.

Native and WASM execute the same semantic conformance corpus for every built-in
transform, preset, output metadata contract, and malformed-input diagnostic.
Byte-for-byte output equality is not a compatibility promise.

## Rust toolchains

- **MSRV:** Rust 1.88.0, declared by workspace metadata and checked with
  `cargo check --locked --workspace --all-targets --all-features`.
- **Pinned development and release toolchain:** Rust 1.97.1, used for
  formatting, Clippy, tests, coverage, native builds, WASM builds, and releases.
- **Fuzzing:** current nightly, isolated to the cargo-fuzz workspace and the
  scheduled AddressSanitizer job.

The pinned toolchain is reviewed during release preparation or when a
dependency requires an upgrade. A toolchain change must be an explicit
repository commit and pass the complete release gate. Raising the MSRV before
1.0 requires a changelog entry and migration note; after 1.0 it requires at
least a minor release.

## Support boundaries

Only environments in this matrix are release-blocking. Other operating systems,
CPUs, Node.js versions, and browsers may work through WASM but are best-effort
until represented in CI. Report reproducible gaps through the GitHub issue
tracker with the runtime, target, input format, and diagnostic output.
