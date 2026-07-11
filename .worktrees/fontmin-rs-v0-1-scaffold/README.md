# fontmin-rs

[![CI](https://github.com/ntnyq/fontmin-rs/workflows/CI/badge.svg)](https://github.com/ntnyq/fontmin-rs/actions)
[![NPM VERSION](https://img.shields.io/npm/v/fontmin-rs.svg)](https://www.npmjs.com/package/fontmin-rs)
[![NPM DOWNLOADS](https://img.shields.io/npm/dy/fontmin-rs.svg)](https://www.npmjs.com/package/fontmin-rs)
[![LICENSE](https://img.shields.io/github/license/ntnyq/fontmin-rs.svg)](https://github.com/ntnyq/fontmin-rs/blob/main/LICENSE)

Fast font subsetting and conversion tooling written in Rust with Node.js bindings.

## v0.1 Scope

The first working slice provides:

- a Rust workspace split into core crates, CLI, napi binding, and TypeScript package;
- native TTF subsetting through `subsetTtf(input, { text })`;
- a Rust CLI command: `fontmin-rs subset input.ttf -t "Hello" -o output.ttf`;
- typed JS helpers for config and built-in plugin declarations;
- a Fontmin-compatible chain shell for `.src().use().dest()`.

## Node API

```ts
import { readFileSync, writeFileSync } from 'node:fs'
import { subsetTtf } from 'fontmin-rs'

const input = readFileSync('fixtures/fonts/ttf/roboto-regular.ttf')
const output = subsetTtf(input, { text: 'Hello' })

writeFileSync('build/roboto-subset.ttf', output)
```

## Rust CLI

```shell
cargo run -p fontmin_app -- subset fixtures/fonts/ttf/roboto-regular.ttf -t "Hello" -o build/roboto-subset.ttf
```

## Roadmap

The design document in `docs/fontmin-rs-design.md` tracks the longer-term plan: WOFF, WOFF2, CSS generation, iconfont support, config loading, cache, wasm fallback, and full Fontmin compatibility.

## License

[MIT](./LICENSE) License © 2026-PRESENT [ntnyq](https://github.com/ntnyq)
