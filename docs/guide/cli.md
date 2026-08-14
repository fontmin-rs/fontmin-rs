# Command Line

Installing `fontmin-rs` exposes the matching bin command:

```sh
fontmin-rs --help
```

When developing this repository, you can also run the Rust CLI directly:

```sh
cargo run -p fontmin_app -- inspect fixtures/fonts/ttf/roboto-regular.ttf --json
```

## init

Create a starter `fontmin.config.jsonc` in the current directory.

```sh
fontmin-rs init
```

The command refuses to overwrite an existing `fontmin.config.jsonc`.

## subset

Trim a TTF font by text.

```sh
fontmin-rs subset fixtures/fonts/ttf/roboto-regular.ttf \
  --text "Hello" \
  --report build/roboto-subset.json \
  --output build/roboto-subset.ttf
```

Options:

| Option                         | Description                                                                   |
| ------------------------------ | ----------------------------------------------------------------------------- |
| `INPUT`                        | Input font path                                                               |
| `-o, --output <OUTPUT>`        | Output TTF path                                                               |
| `-t, --text <TEXT>`            | Text whose glyphs should be kept                                              |
| `--text-file <FILE>`           | File whose text should be kept                                                |
| `--unicodes <LIST>`            | Comma-separated Unicode code points                                           |
| `--gids <LIST>`                | Comma-separated decimal or `0x` original glyph IDs                            |
| `--glyph-names <NAMES>`        | Comma-separated exact PostScript glyph names                                  |
| `--retain-gids`                | Preserve original GIDs and leave empty intermediate glyph slots               |
| `--retain-glyph-names`         | Retain PostScript glyph names in a rewritten `post` v2 table                  |
| `--retain-legacy-cmap`         | Remap and retain non-Unicode, non-symbol `cmap` records                       |
| `--retain-symbol-cmap`         | Remap and retain the Windows symbol `cmap` record                             |
| `--layout-features <TAGS>`     | Comma-separated four-byte GSUB/GPOS feature tags                              |
| `--layout-scripts <TAGS>`      | Comma-separated four-byte GSUB/GPOS script tags                               |
| `--layout-languages <TAGS>`    | Comma-separated language tags; `default` selects DefaultLangSys               |
| `--name-ids <IDS>`             | Comma-separated decimal or `0x` OpenType name IDs                             |
| `--name-languages <IDS>`       | Comma-separated platform-specific decimal or `0x` name language IDs           |
| `--drop-tables <TAGS>`         | Comma-separated four-byte optional tables to remove                           |
| `--pass-through-tables <TAGS>` | Comma-separated source tables to copy verbatim                                |
| `-b, --basic-text`             | Also keep the basic text characters                                           |
| `--missing-glyphs <POLICY>`    | `ignore`, `warn` (default), or `error` for unsupported requested characters   |
| `--report <REPORT>`            | Write subset sizes, retained tables, and original/subset GID mappings as JSON |

Three-character table tags passed to `--drop-tables` or
`--pass-through-tables` are right-padded with a space, so `SVG`, `CFF`, and
`cvt` select the standard `SVG `, `CFF `, and `cvt ` tables.

`--gids` and `--glyph-names` can be used without a text or Unicode selector.
The JSON report also records requested, supported, and missing selectors plus
their original GIDs, making it suitable for build manifests and downstream
glyph remapping.

## coverage

Audit whether a font supports every requested character without producing a
subset:

```sh
fontmin-rs coverage fixtures/fonts/ttf/roboto-regular.ttf \
  --text "A𠮷" \
  --json
```

The report contains sorted `requested`, `supported`, and `missing` code points
plus `coveragePercent`. It accepts TTF, OTF, WOFF, WOFF2, EOT, and SVG font
inputs. Omit `--json` for a short terminal summary.

## convert

Convert between supported font formats.

```sh
fontmin-rs convert fixtures/fonts/ttf/roboto-regular.ttf \
  --format woff2 \
  --output build/roboto.woff2
```

Common target formats:

| Format  | Purpose                                                 |
| ------- | ------------------------------------------------------- |
| `woff2` | Preferred web font format for modern web                |
| `woff`  | Web font fallback format                                |
| `eot`   | Legacy IE compatibility                                 |
| `svg`   | SVG font output                                         |
| `ttf`   | Convert static CFF OTF, or decode WOFF/EOT, back to TTF |

For CFF2 variable fonts, repeat `--variation TAG=VALUE` to select a user-space instance:

```sh
fontmin-rs convert fixtures/fonts/otf/source-serif-4-variable-roman.otf \
  --format ttf \
  --variation wght=700 \
  --variation opsz=14 \
  --output build/source-serif-4.ttf
```

The result is a static TTF without CFF2 or variation tables; Type 2 hinting is not preserved.

## instance

Pin every axis of a variable font and write one static TTF. Unlike `convert`,
this command also supports a default instance with no explicit coordinates:

```sh
fontmin-rs instance fixtures/fonts/ttf/noto-sans-sc-variable-compact.ttf \
  --variation wght=700 \
  --output build/noto-sans-sc-700.ttf
```

`glyf` variable input may be TTF, WOFF, WOFF2, or EOT; CFF2 input uses OTF.
Repeat `--variation TAG=VALUE` for explicit user-space coordinates. Omitted axes
use their `fvar` defaults, while unknown, non-finite, duplicate, and out-of-range
values fail the command. The static output preserves glyph IDs and removes
variation tables and TrueType hinting programs.

To keep the result variable, add `--keep-variable`. Pin an axis with
`--variation`, narrow another with `--variation-range TAG=MIN:MAX[:DEFAULT]`,
and leave all unlisted axes unchanged:

```sh
fontmin-rs instance fixtures/fonts/ttf/estedad-variable.ttf \
  --keep-variable \
  --variation wdth=150 \
  --variation-range wght=300:700:500 \
  --output build/estedad-reduced.ttf
```

`--downgrade-cff2` converts CFF2 to CFF1 when the supplied pins remove every
axis. Range reduction preserves CFF2.

## build

`build` is the batch processing entry point for project scripts and CI.

```sh
fontmin-rs build fixtures/fonts/ttf/roboto-regular.ttf \
  -o build \
  --text "Hello" \
  --preset modern-web \
  --font-family Roboto
```

Without `--config`, `build` discovers the first existing file in this exact
order: `fontmin.config.ts`, `fontmin.config.mts`, `fontmin.config.mjs`,
`fontmin.config.cjs`, `fontmin.config.json`, then `fontmin.config.jsonc`.
Executable module configs require Node.js 22.18 or newer. JSON and JSONC are
parsed entirely in Rust and remain available when Node.js is not installed.
See [Configuration](./config) for module exports, security, and supported
plugin boundaries.

Use `--formats` for exact output control, or `--preset modern-web` / `--preset compat` for common font output groups. Use `--preset iconfont` with multiple SVG icon inputs to emit `iconfont.ttf` and `iconfont.css`; delivery slices are not supported by the iconfont preset.

Static CFF OTF and CFF2 variable OTF inputs are normalized to static TTF before
the Web pipeline subsets or converts them. For CFF2, repeat `--variation` to
select an instance:

```sh
fontmin-rs build fixtures/fonts/otf/source-serif-4-variable-roman.otf \
  -o build \
  --preset modern-web \
  --variation wght=700 \
  --variation opsz=14
```

Repeat `--css-unicode-range` to add global `unicode-range` descriptors to CSS
sources. This annotates browser matching and does not change the glyph subset
or create additional font files.

To emit named Unicode delivery slices, repeat `--delivery-slice`. Each slice
creates a matching font file and `@font-face` descriptor when CSS output is
selected:

```sh
fontmin-rs build fixtures/fonts/ttf/roboto-regular.ttf \
  -o build \
  --text "Hello" \
  --preset modern-web \
  --delivery-slice latin:U+0000-00FF \
  --delivery-slice cjk:U+4E00-9FFF
```

Repeated flags with the same name append ranges to that slice. Slice names may
contain letters, digits, hyphens, and underscores. Supplying any
`--delivery-slice` flags replaces slices declared in the configuration file.

For measured automatic slicing, use `--auto-delivery` with language and byte
constraints. The target applies to the selected measurement format; every
generated face still receives its exact own `unicode-range`:

```sh
fontmin-rs build fixtures/fonts/ttf/noto-sans-sc-compact.ttf \
  -o build \
  --preset modern-web \
  --auto-delivery \
  --delivery-languages en,zh-Hans \
  --delivery-frequency-text "AB中文" \
  --delivery-target-bytes 102400 \
  --delivery-tolerance 0.15 \
  --delivery-max-slices 16 \
  --delivery-measure-format woff2
```

Language presets are `ar`, `el`, `en`, `hi`, `ja`, `ko`, `ru`, `zh-Hans`,
and `zh-Hant`. If `--delivery-languages` is omitted, the planner detects them
from frequency text. Automatic and manual delivery flags cannot be combined.

Options:

| Option                           | Description                                                         |
| -------------------------------- | ------------------------------------------------------------------- |
| `INPUT...`                       | Input font paths, with glob support                                 |
| `-c, --config <CONFIG>`          | TS, MTS, MJS, CJS, JSON, or JSONC configuration file                |
| `-o, --out-dir <OUT_DIR>`        | Output directory                                                    |
| `-t, --text <TEXT>`              | Text used for subsetting                                            |
| `--text-file <FILE>`             | File content used for subsetting                                    |
| `--unicodes <LIST>`              | Comma-separated Unicode code points                                 |
| `--gids <LIST>`                  | Comma-separated decimal or `0x` original glyph IDs                  |
| `--glyph-names <NAMES>`          | Comma-separated exact PostScript glyph names                        |
| `--retain-gids`                  | Preserve original GIDs and leave empty intermediate glyph slots     |
| `--retain-glyph-names`           | Retain PostScript glyph names in a rewritten `post` v2 table        |
| `--retain-legacy-cmap`           | Remap and retain non-Unicode, non-symbol `cmap` records             |
| `--retain-symbol-cmap`           | Remap and retain the Windows symbol `cmap` record                   |
| `--layout-features <TAGS>`       | Comma-separated four-byte GSUB/GPOS feature tags                    |
| `--layout-scripts <TAGS>`        | Comma-separated four-byte GSUB/GPOS script tags                     |
| `--layout-languages <TAGS>`      | Comma-separated language tags; `default` selects DefaultLangSys     |
| `--name-ids <IDS>`               | Comma-separated decimal or `0x` OpenType name IDs                   |
| `--name-languages <IDS>`         | Comma-separated platform-specific name language IDs                 |
| `--drop-tables <TAGS>`           | Comma-separated four-byte optional tables to remove                 |
| `--pass-through-tables <TAGS>`   | Comma-separated source tables to copy verbatim                      |
| `-b, --basic-text`               | Also keep the basic text characters                                 |
| `--missing-glyphs <POLICY>`      | `ignore`, `warn` (default), or `error` for unsupported characters   |
| `-d, --deflate-woff`             | Keep Fontmin-compatible WOFF deflate behavior                       |
| `-T, --show-time`                | Print build elapsed time                                            |
| `--silent`                       | Suppress optional build timing output                               |
| `--cache`                        | Enable the native build cache                                       |
| `--no-cache`                     | Disable the native build cache                                      |
| `--css-glyph`                    | Generate glyph class CSS rules                                      |
| `--css-unicode-range <RANGE>`    | Add a global CSS `unicode-range` descriptor; repeat for more ranges |
| `--delivery-slice <NAME:RANGES>` | Add a named Unicode delivery slice; repeat to add ranges or slices  |
| `--auto-delivery`                | Enable measured language-aware delivery slicing                     |
| `--delivery-languages <TAGS>`    | Comma-separated language presets for automatic slicing              |
| `--delivery-frequency-text <T>`  | Prioritize repeated business characters and detect languages        |
| `--delivery-target-bytes <N>`    | Target encoded bytes per automatic slice                            |
| `--delivery-tolerance <N>`       | Fractional target tolerance in the range `[0, 1)`                   |
| `--delivery-max-slices <N>`      | Cap generated slices and font requests at 1–256                     |
| `--delivery-measure-format <F>`  | Enforce the target against `ttf`, `woff`, or `woff2`                |
| `--variation <TAG=VALUE>`        | Fully instance a variable-font axis before subsetting               |
| `--variation-range <RANGE>`      | Retain an axis with `TAG=MIN:MAX[:DEFAULT]` in `instance`           |
| `--keep-variable`                | Leave unlisted axes variable in `instance`                          |
| `--downgrade-cff2`               | Downgrade fully pinned CFF2 output to CFF1                          |
| `--formats <FORMATS>`            | Comma-separated output formats                                      |
| `--preset <PRESET>`              | `modern-web`, `compat`, or `iconfont`                               |
| `--no-original`                  | Drop requested original TTF output                                  |
| `--font-family <FONT_FAMILY>`    | Font family name used in CSS                                        |
| `--font-path <FONT_PATH>`        | Path prefix used for font file references CSS                       |

Iconfont example:

```sh
fontmin-rs build icons/home.svg icons/user.svg \
  -o build/icons \
  --preset iconfont \
  --font-family "Project Icons"
```

## bench

Measure native subset performance for one TTF input.

```sh
fontmin-rs bench fixtures/fonts/ttf/roboto-regular.ttf \
  --text-file chars.txt \
  --json
```

Without `--json`, the command prints a short terminal summary. With `--json`, it emits `operation`, `inputBytes`, `outputBytes`, and `elapsedMs` for scripts and benchmark harnesses.

## inspect

Read font format and metadata.

```sh
fontmin-rs inspect fixtures/fonts/ttf/roboto-regular.ttf --json
```

Without `--json`, the command prints a human-readable terminal summary. With `--json`, it emits script-friendly structured output.

For WOFF2 files, inspect validates the WOFF2 header and table directory and reads sfnt metadata tables such as `name`, `head`, `hhea`, and `maxp`. `fontmin-rs convert input.woff2 -f ttf -o output.ttf` decodes WOFF2 back to TTF.

## Exit status and diagnostics

Successful commands exit with status `0`. Processing, configuration, and input
failures exit nonzero. Structured fontmin-rs failures include a stable
machine-readable code in stderr, for example `fontmin::invalid_font`, followed
by a human-readable message. Malformed input is rejected without emitting a
Rust panic or backtrace through the public CLI.

Scripts should branch on the exit status and diagnostic code, not the
presentation glyphs or indentation used by the terminal reporter.

## doctor

Verify that the Rust CLI starts successfully and print a short status line.

```sh
fontmin-rs doctor
```
