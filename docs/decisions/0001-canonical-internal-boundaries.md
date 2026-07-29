# ADR 0001: Canonical internal boundaries

- Status: Accepted
- Date: 2026-07-29

## Context

The same behavior had accumulated in several public-surface adapters: Node and
WASM optimized assets with similar policy code, multiple format crates wrote
sfnt directories, native release scripts repeated platform maps, and the Rust
CLI build command owned orchestration, cache persistence, locking, and output
safety in one file.

Those repetitions made local edits risky. A new native target, output-safety
rule, diagnostic behavior, or sfnt invariant could be updated in one path while
another path silently drifted.

## Decision

Use one canonical module for each cross-cutting invariant and keep public
facades thin:

| Invariant                                                      | Canonical owner                             |
| -------------------------------------------------------------- | ------------------------------------------- |
| Node workspace reads, path expansion, safe cleanup, and writes | `packages/fontmin/src/workspace-io.ts`      |
| Node/WASM optimizer policy and diagnostic normalization        | `packages/fontmin/src/runtime-neutral/`     |
| Rust CLI cache persistence and locking                         | `apps/fontmin/src/commands/build/cache.rs`  |
| Rust CLI output containment and guarded cleanup                | `apps/fontmin/src/commands/build/output.rs` |
| sfnt directory validation and serialization                    | `crates/fontmin_ttf/src/sfnt.rs`            |
| Native target-to-package layout                                | `scripts/native-release-layout.mjs`         |
| CLI integration-test process and temporary workspace setup     | `apps/fontmin/tests/cli/support.rs`         |

SVG icon-font internals are separated by responsibility into markup extraction,
path geometry, and TTF table construction. Their public functions remain in
`crates/fontmin_svg/src/icon.rs`.

The N-API `targets` array is the authoritative native target inventory.
Platform package names, directories, artifact names, runtime selection, and
metadata checks are derived from it. Workflow matrices remain declarative YAML,
with tests preventing drift from the inventory.

## Consequences

- A cross-runtime policy change has one implementation and must pass both Node
  and WASM tests.
- A format crate cannot customize sfnt directory mechanics; it supplies table
  data to the canonical writer.
- Cache and output safety can evolve independently of CLI build orchestration.
- Adding a native target requires the N-API target, platform manifest, and
  workflow runner, while validation reports every inconsistent surface.
- Internal modules may be private and opinionated. Public API compatibility is
  still governed by `contracts/` and the deprecation policy.

The trade-off is additional internal modules and dependencies on their
interfaces. `CONTEXT.md` files record ownership so new behavior deepens an
existing module instead of recreating a parallel implementation.

## Rejected alternatives

- Keep duplicated implementations synchronized by convention. Tests showed
  that several independent lists and serializers were already costly to audit.
- Create a general utility crate or package for all shared code. The concerns
  have different dependencies and rates of change; one broad utility module
  would weaken locality.
- Generate CI workflow YAML from code. The repository keeps reviewable workflow
  configuration and uses a drift test instead of adding a generation step.
