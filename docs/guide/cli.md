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
  --output build/roboto-subset.ttf
```

Options:

| Option                  | Description                         |
| ----------------------- | ----------------------------------- |
| `INPUT`                 | Input font path                     |
| `-o, --output <OUTPUT>` | Output TTF path                     |
| `-t, --text <TEXT>`     | Text whose glyphs should be kept    |
| `--text-file <FILE>`    | File whose text should be kept      |
| `--unicodes <LIST>`     | Comma-separated Unicode code points |
| `-b, --basic-text`      | Also keep the basic text characters |

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
Executable module configs require Node.js 22 or newer. JSON and JSONC are
parsed entirely in Rust and remain available when Node.js is not installed.
See [Configuration](./config) for module exports, security, and supported
plugin boundaries.

Use `--formats` for exact output control, or `--preset modern-web` / `--preset compat` for common font output groups. Use `--preset iconfont` with multiple SVG icon inputs to emit `iconfont.ttf` and `iconfont.css`.

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

Options:

| Option                           | Description                                                        |
| -------------------------------- | ------------------------------------------------------------------ |
| `INPUT...`                       | Input font paths, with glob support                                |
| `-c, --config <CONFIG>`          | TS, MTS, MJS, CJS, JSON, or JSONC configuration file               |
| `-o, --out-dir <OUT_DIR>`        | Output directory                                                   |
| `-t, --text <TEXT>`              | Text used for subsetting                                           |
| `--text-file <FILE>`             | File content used for subsetting                                   |
| `--unicodes <LIST>`              | Comma-separated Unicode code points                                |
| `-b, --basic-text`               | Also keep the basic text characters                                |
| `-d, --deflate-woff`             | Keep Fontmin-compatible WOFF deflate behavior                      |
| `-T, --show-time`                | Print build elapsed time                                           |
| `--silent`                       | Suppress optional build timing output                              |
| `--cache`                        | Enable the native build cache                                      |
| `--no-cache`                     | Disable the native build cache                                     |
| `--css-glyph`                    | Generate glyph class CSS rules                                     |
| `--delivery-slice <NAME:RANGES>` | Add a named Unicode delivery slice; repeat to add ranges or slices |
| `--variation <TAG=VALUE>`        | Select a CFF2 user-space axis coordinate for OTF input             |
| `--formats <FORMATS>`            | Comma-separated output formats                                     |
| `--preset <PRESET>`              | `modern-web`, `compat`, or `iconfont`                              |
| `--no-original`                  | Drop requested original TTF output                                 |
| `--font-family <FONT_FAMILY>`    | Font family name used in CSS                                       |
| `--font-path <FONT_PATH>`        | Path prefix used for font file references CSS                      |

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

## doctor

Print local environment and native binding status for installation troubleshooting.

```sh
fontmin-rs doctor
```
