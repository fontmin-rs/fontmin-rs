import { readFile } from 'node:fs/promises'
import { beforeEach, expect, it, vi } from 'vitest'

const wasm = new URL(
  '../src/generated/fontmin_wasm_core_bg.wasm',
  import.meta.url,
)

beforeEach(() => {
  vi.resetModules()
})

async function loadRuntime() {
  return import('../src/runtime')
}

it('rejects module access before WASM initialization', async () => {
  const { getWasmModule } = await loadRuntime()

  await expect(getWasmModule()).rejects.toThrow(
    'fontmin-rs WASM runtime is not initialized; call initWasm() first',
  )
})

it('returns the initialized generated module', async () => {
  const { getWasmModule, initWasm, isWasmInitialized } = await loadRuntime()

  await initWasm(await readFile(wasm))

  expect(isWasmInitialized()).toBe(true)
  expect((await getWasmModule()).runtime_name()).toBe('fontmin-rs')
})
