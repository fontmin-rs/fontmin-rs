# Rust CLI build context

`../build.rs` is the orchestration facade. It resolves configuration and inputs,
runs the pipeline, and coordinates the modules in this directory.

## Modules

- `cache.rs`: cache keys, manifests, restore/store, atomic persistence.
- `cache/lock.rs`: cross-process lock acquisition, stale-lock recovery, release.
- `output.rs`: output representation, duplicate detection, containment,
  symlink checks, safe writes, and guarded cleanup.

Cache code must not write final build outputs directly. Output code must not
interpret pipeline configuration or construct cache keys. New build behavior
belongs in the facade unless it deepens one of these existing interfaces.

The integration-test setup for this command lives in
`apps/fontmin/tests/cli/support.rs`; production helpers must not depend on it.
