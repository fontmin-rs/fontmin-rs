# fontmin-rs

Fast font subsetting and conversion tooling powered by Rust, with CLI, Node.js,
and browser WASM APIs.

> `fontmin-rs` is in the `0.1.0` release-candidate cycle. Install the `rc`
> dist-tag until the first stable release.

## Install

```sh
pnpm add fontmin-rs@rc
```

## Quick start

```sh
fontmin-rs build fonts/roboto.ttf \
  --out-dir build \
  --text "Hello, fontmin-rs" \
  --preset modern-web \
  --font-family Roboto
```

See the [documentation](https://fontmin-rs.ntnyq.dev/) for the Node.js API,
CLI configuration, browser WASM runtime, and migration guides.

## License

[MIT](https://github.com/fontmin-rs/fontmin-rs/blob/main/LICENSE)
