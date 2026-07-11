# Browser Font Playground Design

## Goal

Add a VitePress playground that lets a user process a local font entirely in
their browser:

1. Upload one supported font file.
2. Enter a required character set for subsetting.
3. Select output formats.
4. Generate a ZIP containing all requested outputs.

The playground must use `@fontmin-rs/wasm`; uploaded font bytes never leave
the browser.

## Scope

### Supported inputs

The playground accepts the formats the browser WASM package can normalize into
TTF:

- TTF
- WOFF
- WOFF2
- EOT
- OTF
- SVG Font

The uploaded SVG must be an SVG font. A standalone path-based SVG is not a
font input for this one-file playground and produces a clear error.

### Outputs

The user can select TTF, WOFF, WOFF2, EOT, SVG, and CSS. WOFF2, WOFF, and CSS
are selected initially. CSS is generated from the output font assets and uses
their ZIP-relative file names.

The character set is mandatory. Empty or whitespace-only input disables
generation; the playground never silently ships an unsubsetted font.
Selecting CSS also requires at least one font output, because CSS needs a
font source to reference.

## User experience

The page lives at `/playground` and is linked from the English and Chinese
VitePress navigation. It uses the existing VitePress variables, typography,
card treatment, controls, and dark-mode behavior.

The page contains these ordered sections:

1. A short title and a local-processing statement.
2. A click-and-drop upload control showing the input name, format, and size.
3. A character-set textarea that displays character and unique Unicode counts.
4. Output format checkboxes.
5. A generate control that shows the current stage.
6. A result list and a ZIP download control.

The ZIP is named `<input-stem>-fontmin.zip`. The result list shows each output
name, format, and byte size.

## Processing pipeline

The client component initializes the WASM runtime once. On generation it:

1. Reads the uploaded `File` into a `Uint8Array` and determines its extension.
2. Normalizes supported input to TTF:
   - TTF passes through unchanged.
   - WOFF, WOFF2, and EOT decode to TTF.
   - OTF and SVG Font convert to TTF.
3. Calls `subsetTtf()` with the required textarea contents.
4. Produces each requested font output from the subset TTF.
5. Produces CSS after the selected font outputs are known.
6. Adds every output to a browser-generated ZIP and exposes it as a download.

No partial ZIP is exposed: if any selected conversion fails, the prior result
is kept and an error is shown.

## Client modules

The implementation is split into small browser-only modules:

- A `Playground` Vue component owns input state, interaction, progress, and
  presentation.
- A font processing helper converts a file plus selection into named output
  assets using `@fontmin-rs/wasm`.
- A ZIP helper uses `fflate` to create ZIP bytes from the assets; `tinysaver`
  triggers the browser download and releases the download resource.
- A local component test suite covers validation, successful output creation,
  and errors without requiring a real download.

The page dynamically imports browser-only code so VitePress server rendering
does not attempt to initialize WASM.

## Errors and boundaries

The UI reports an actionable error for unsupported extensions, malformed font
data, invalid SVG Font input, WASM initialization failure, and conversion
failure. It rejects multiple uploads and prevents generation without a
character set or selected output format.

The playground has no filesystem access, server upload, glob support, cache,
or Node plugin hooks. Downloading and cleanup use browser `Blob` and object URL
APIs only.

## Verification

- Unit/component tests cover input validation, format normalization, output
  selection, CSS output, ZIP entries, and failed generation.
- `pnpm -C docs run build` verifies VitePress client and server builds.
- The existing Chromium, Firefox, and WebKit WASM acceptance test continues to
  verify generated WOFF2 can load through `FontFace`.
