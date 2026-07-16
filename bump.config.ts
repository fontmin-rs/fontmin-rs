import { defineConfig } from 'bumpp'

export default defineConfig({
  commit: false,
  execute: 'cargo check --workspace',
  files: [
    'package.json',
    'packages/fontmin/package.json',
    'napi/fontmin/package.json',
    'wasm/fontmin/package.json',
    'npm/*/package.json',
    'Cargo.toml',
    'packages/fontmin/src/optimize.ts',
    'packages/fontmin/bin/fontmin-rs.mjs',
    'napi/fontmin/src-js/bindings.js',
  ],
  push: false,
  tag: false,
})
