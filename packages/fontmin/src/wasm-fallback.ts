import { readFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'

interface WasmRuntime {
  initWasm(input?: Uint8Array): Promise<void>
  ttfToWoff2(
    input: Uint8Array,
    options?: { quality?: number },
  ): Promise<Uint8Array>
}

const require = createRequire(import.meta.url)
const wasmPackageName = '@fontmin-rs/wasm'
let runtime: Promise<WasmRuntime> | undefined

export async function loadWasmRuntime(): Promise<WasmRuntime> {
  runtime ??= initializeWasmRuntime()

  return runtime
}

async function initializeWasmRuntime(): Promise<WasmRuntime> {
  try {
    const entry = require.resolve(wasmPackageName)
    const bytes = await readFile(
      join(dirname(entry), 'fontmin_wasm_core_bg.wasm'),
    )
    const wasm = (await import(wasmPackageName)) as WasmRuntime

    await wasm.initWasm(bytes)

    return wasm
  } catch (cause) {
    throw new Error('WOFF2 WASM fallback failed', { cause })
  }
}
