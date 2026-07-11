import { readFile } from 'node:fs/promises'
import { expect, it } from 'vitest'
import { initWasm, isWasmInitialized } from '../src/index'
it('initializes without importing the native binding', async () => {
  expect(isWasmInitialized()).toBe(false)
  const wasm = await readFile(
    new URL('../src/generated/fontmin_wasm_core_bg.wasm', import.meta.url),
  )
  await initWasm(wasm)
  expect(isWasmInitialized()).toBe(true)
})
