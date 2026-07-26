# Security policy

## Supported versions

Security fixes are made for the latest release on each active npm dist-tag.
During prerelease development, only the newest beta or release candidate is
supported. Older prereleases should be upgraded before reporting a problem.

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability. Use
[GitHub private vulnerability reporting](https://github.com/fontmin-rs/fontmin-rs/security/advisories/new)
and include:

- the affected package, version, runtime, and platform;
- the smallest input or configuration that reproduces the issue;
- the security impact and any known preconditions;
- whether the report or a proof of concept has been shared elsewhere.

Avoid attaching proprietary fonts unless they are essential to reproduce the
issue. A minimized synthetic input is preferred.

The maintainers aim to acknowledge a report within three business days and
provide an initial assessment within seven. Timelines for a fix and coordinated
disclosure depend on severity, exploitability, and release complexity. Please
allow a reasonable remediation window before public disclosure.

## Release controls

Every release runs Rust advisory and source checks without accepted advisory
exceptions, plus a pnpm audit that rejects high and critical findings. Native,
WASM, malformed-input, sanitizer, package, and provenance checks are described
in the [release guide](./docs/releasing.md).
