# Troubleshooting

Start by recording the installed version, Node.js version, operating system,
CPU architecture, requested runtime, input format, and complete diagnostic.
`fontmin-rs inspect <font> --json` provides a machine-readable input summary.

## Native binding cannot load

The published `fontmin-rs` package keeps native platform packages optional.
Confirm that optional dependencies were not disabled and that the current
platform appears in the [support matrix](./support.md).

```sh
pnpm install
node --input-type=module -e \
  "import{readFileSync}from'node:fs';import{inspect}from'fontmin-rs';console.log(inspect(readFileSync(process.argv[1])))" \
  path/to/font.ttf
```

Use `runtime: "auto"` when WASM fallback is acceptable. Use
`runtime: "native"` when a missing native package must fail immediately. Do not
copy a `.node` file between operating systems, CPU architectures, libc
variants, or Node-API package versions.

## WASM fallback fails

The Node package requires the `@fontmin-rs/wasm` dependency. Reinstall from the
lockfile and check that bundler rules did not exclude `.wasm` assets. Browser
code must import `@fontmin-rs/wasm` directly and use its asynchronous,
memory-only API; filesystem paths and Node plugin hooks are not available.

## Configuration is not found or rejected

Run the CLI from the intended project directory or pass `--config` explicitly.
Executable TypeScript, MTS, MJS, and CJS configuration requires Node.js 22.18
or newer. Unknown fields and runtime/fallback conflicts are errors by design.
Compare the resolved shape with the [configuration reference](./guide/config.md).

## A font is rejected

First run `inspect` and `coverage` separately. A stable `invalid-font`,
`unsupported`, or missing-glyph diagnostic is preferable to retrying the same
input through a different output format. Reduce confidential inputs before
sharing them; malformed inputs that expose a parser failure can be reported
privately under the [security policy](https://github.com/fontmin-rs/fontmin-rs/security/policy).

## Output differs from classic Fontmin

Compare parsed metadata, requested glyph coverage, CSS semantics, and file
naming. Byte-for-byte equality is not a compatibility promise. The
[migration guide](./guide/migration.md) lists intentional API differences, and
the [performance policy](./benchmarks.md) explains the representative paired
benchmark.

## Reporting a reproducible problem

For ordinary bugs, open a GitHub issue with the smallest redistributable font
or synthetic reproducer and the information listed at the top of this page.
For a suspected vulnerability, use private vulnerability reporting instead of
a public issue.
