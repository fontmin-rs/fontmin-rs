import { cp } from 'node:fs/promises'

await cp(
  new URL('../src/generated/fontmin_wasm_core_bg.wasm', import.meta.url),
  new URL('../dist/fontmin_wasm_core_bg.wasm', import.meta.url),
)
