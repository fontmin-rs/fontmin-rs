import type { InitInput } from './generated/fontmin_wasm_core'

let initialization: Promise<void> | undefined
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

export async function initWasm(input?: InitInput): Promise<void> {
  initialization ??= import('./generated/fontmin_wasm_core').then(
    async module => {
      await module.default(
        input === undefined ? undefined : { module_or_path: input },
      )

      if (module.runtime_name() !== 'fontmin-rs') {
        throw new Error('fontmin-rs WASM runtime did not initialize correctly')
      }

      initialized = true
    },
  )

  return initialization
}
