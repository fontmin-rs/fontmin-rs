# Quality Gates, Unicode Range, and Playground Design

## Goal

Improve the repository without publishing packages: make docs and package
artifacts first-class verification targets, add safe `@font-face`
`unicode-range` support across public APIs, and expose it in the browser
Playground with end-to-end coverage.

## Scope and order

The work has three dependent stages. Stage 1 is independent infrastructure and
must land first. Stage 2 adds the font-delivery feature that Stage 3 surfaces
to browser users.

1. **Quality and artifact gates.** Root scripts and GitHub Actions run docs
   tests and production builds. Local package checks build npm tarballs and
   install them in disposable consumers; no registry command is run. A docs
   Playwright acceptance test uploads a local fixture, subsets it, and verifies
   individual and ZIP download results.
2. **Unicode range.** `@font-face` CSS can include validated, canonical
   `unicode-range` descriptors. The feature is available to Rust, config,
   CLI build options, N-API, Node, and WASM APIs.
3. **Playground.** The localized Playground accepts optional Unicode-range
   descriptors, forwards them to CSS generation, displays validation errors,
   and explains that they govern browser font matching rather than subsetting.

## Unicode-range model

`fontmin_css` owns a `UnicodeRange` value type with inclusive `start` and
`end` scalar values. It accepts the unambiguous CSS subset `U+XXXX` and
`U+XXXX-YYYYYY`, where each endpoint has 1–6 ASCII hexadecimal digits,
including lowercase input. Ranges must be within `U+0..U+10FFFF` and have
`start <= end`. The canonical CSS form is uppercase, at least four digits, and
uses one comma-separated `unicode-range` declaration.

Wildcards, arbitrary CSS text, control characters, and malformed ranges are
rejected. This prevents CSS injection at every public boundary.

The Rust `CssOptions` stores `unicode_ranges: Vec<UnicodeRange>`, serializing
as `unicodeRanges`. The JS and JSON-facing APIs accept `unicodeRanges?:
string[]`; each string represents one descriptor. An empty list omits the CSS
property. This keeps config, N-API, WASM JSON, and browser form values
consistent.

## API flow

```text
CLI/config | Node/N-API | WASM/Playground
        └──────── unicodeRanges: string[] ────────┐
                                                   v
                           fontmin_css::UnicodeRange::parse
                                                   v
                     CssOptions { unicode_ranges: Vec<UnicodeRange> }
                                                   v
                    @font-face ... unicode-range: U+....;
```

The CLI accepts repeatable `--css-unicode-range RANGE` only for `build`.
Config uses `css.unicodeRanges`. Explicit CLI values replace config values,
matching the existing CLI override convention.

## Quality gates

The root gains explicit `docs:test`, `docs:build`, and local package smoke
commands. `check` includes the docs tests and build after the existing quality
checks. CI has a docs/Playground browser job that builds the WASM package using
the Rust target already installed for the existing browser matrix, builds the
docs site, starts a local static server, and runs the acceptance script in
Chromium.

The package smoke command packs `fontmin-rs` and `@fontmin-rs/wasm` to local
tarballs, installs each into an isolated temporary project, and runs a tiny
ESM import/use program. It never runs `pnpm publish`, reads package-manager
credentials, or changes package versions.

## Playground behavior

The form adds an optional `Unicode ranges for CSS` text field, accepting
comma-separated descriptors. It is visible only when CSS output is selected;
removing CSS clears the field from the outgoing request but preserves editing
state. Before computation, the client validates the same limited grammar and
shows the exact malformed descriptor. Valid values are split, normalized in
the WASM core, and passed to `generateFontFaceCss`. The generated CSS contains
the canonical descriptor list.

English and Chinese copy explains that character text controls which glyphs
are retained, while Unicode ranges tell browsers which code points should use
the generated face.

## Verification

- Rust tests cover valid canonicalization, malformed/inverted/out-of-scalar
  rejection, and CSS emission/omission.
- CLI, N-API, Node, WASM, and config tests verify forwarding and output.
- Playground unit/component tests cover localized validation and request
  forwarding.
- The browser acceptance test covers actual file upload, generation, CSS
  contents, individual download, and ZIP download.
- Root `pnpm run check`, full WASM build, docs build, package tarball smoke
  test, and CI job syntax all pass before completion.

## Non-goals

- No npm publishing, registry inspection, version bump, or release workflow
  execution.
- No wildcard CSS unicode-range syntax, automatic language detection, or
  multiple independent subset files.
- No new font-outline format conversion in this project.
