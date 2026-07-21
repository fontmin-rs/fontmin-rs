import type { InitInput } from './generated/fontmin_wasm_core'
import type * as generatedWasmModule from './generated/fontmin_wasm_core'

type WasmModule = typeof generatedWasmModule

let initialization: Promise<WasmModule> | undefined
let initialized = false

export function isWasmInitialized(): boolean {
  return initialized
}

export function assertWasmInitialized(): void {
  if (!initialized) {
    throw new Error(
      'fontmin-rs WASM runtime is not initialized; call initWasm() first',
    )
  }
}

export async function getWasmModule(): Promise<WasmModule> {
  assertWasmInitialized()
  const module = initialization
  if (module === undefined) {
    throw new Error('fontmin-rs WASM runtime initialization is unavailable')
  }

  return module
}

export async function initWasm(input?: InitInput): Promise<void> {
  initialization ??= initializeWasm(input)

  await initialization
}

async function initializeWasm(input?: InitInput): Promise<WasmModule> {
  const module = await import('./generated/fontmin_wasm_core')
  await module.default(
    input === undefined ? undefined : { module_or_path: input },
  )

  if (module.runtime_name() !== 'fontmin-rs') {
    throw new Error('fontmin-rs WASM runtime did not initialize correctly')
  }

  initialized = true
  return module
}
