# fontmin-rs compatibility patch

This directory is the published `oxifont-subset` 0.2.2 source. fontmin-rs makes
two manifest-only changes: it lowers the package `rust-version` declaration
from 1.89 to 1.88, and removes the unused production `oxifont-parser` and
`ttf-parser` dependencies. Neither crate is referenced by `src/`; `ttf-parser`
remains only a development dependency for the upstream test suite. The
unmodified Rust source compiles under Rust 1.88, and the repository MSRV job
verifies it as part of the complete workspace dependency graph.

Remove this override after an upstream release both declares Rust 1.88
compatibility and removes the unused production dependencies, or when
fontmin-rs raises its MSRV in a SemVer-minor release and the dependency audit
accepts the upstream graph.
