# fontmin-rs compatibility patch

This directory is the published `oxifont-core` 0.2.2 source. fontmin-rs changes
only the package `rust-version` declaration from 1.89 to 1.88. The unmodified
source compiles under Rust 1.88, and the repository MSRV job verifies it as part
of the complete workspace dependency graph.

Remove this override after an upstream release declares Rust 1.88 compatibility,
or when fontmin-rs raises its MSRV in a SemVer-minor release.
