# Repository Context

This file is the maintainer map for choosing the smallest correct module for a
change. Public behavior is documented under `docs/`; this file records internal
ownership and invariants.

## Public surfaces

- `apps/fontmin`: Rust CLI and project-file workflow.
- `packages/fontmin`: Node.js API, CLI wrapper, plugins, cache, and workspace
  I/O.
- `wasm/fontmin`: asynchronous, memory-only browser API.
- `crates/fontmin`: Rust facade shared by the CLI and JavaScript bridges.

## Change map

| Change                                            | Primary owner                               |
| ------------------------------------------------- | ------------------------------------------- |
| Font parsing, conversion, or metadata             | Format crate under `crates/`                |
| sfnt directory reading or writing                 | `crates/fontmin_ttf/src/sfnt.rs`            |
| Rust CLI build orchestration                      | `apps/fontmin/src/commands/build.rs`        |
| Rust CLI cache persistence or locking             | `apps/fontmin/src/commands/build/cache.rs`  |
| Rust CLI safe output writes or clean checks       | `apps/fontmin/src/commands/build/output.rs` |
| Node path expansion, text files, clean, or writes | `packages/fontmin/src/workspace-io.ts`      |
| Semantics shared by Node and browser pipelines    | `packages/fontmin/src/runtime-neutral/`     |
| Runtime loading and operation adapters            | `packages/fontmin/src/optimize-runtime.ts`  |
| Native target/package/artifact mapping            | `scripts/native-release-layout.mjs`         |
| CLI integration-test setup                        | `apps/fontmin/tests/cli/support.rs`         |

## Invariants

1. Runtime-neutral modules do not import Node APIs, native bindings, or WASM
   initialization code.
2. Format crates provide table payloads; `fontmin_ttf` owns sfnt directory
   validation, ordering, alignment, checksums, and checksum adjustment.
3. Output cleanup and writes pass through containment and symlink checks.
4. The N-API `targets` array is the source for native package and artifact
   layout; manifests and workflow matrices are checked against it.
5. Public contract changes update `contracts/`, migration guidance, changelog,
   and compatibility tests together.

## Verification

Use targeted checks while editing, then run `pnpm run check` before submitting.
The repository guide in `AGENTS.md` lists the supported commands and test
locations. Architecture decisions that change these ownership rules belong in
`docs/decisions/`.
