# safer-bytes compatibility patch

This directory contains the `safer-bytes` 0.2.0 API used by
`woff2-patched`. It is derived from
<https://github.com/danieleades/safer-bytes> and retains the upstream MIT
license declaration.

The local patch replaces identifier-concatenating `paste` macros with explicit
stable Rust method identifiers. This removes the unmaintained `paste`
dependency without changing the public API consumed by the WOFF2 decoder.

Keep this patch until `woff2-patched` no longer depends on `safer-bytes` 0.2.0.
