import { createRequire } from 'node:module'

type NativeBinding = typeof import('@fontmin-rs/binding')

const require = createRequire(import.meta.url)
let binding: NativeBinding | undefined

export class NativeBindingLoadError extends Error {
  constructor(cause: unknown) {
    super('fontmin-rs native binding is unavailable', { cause })
    this.name = 'NativeBindingLoadError'
  }
}

export function loadNativeBinding(): NativeBinding {
  if (binding !== undefined) {
    return binding
  }

  try {
    binding = require('@fontmin-rs/binding') as NativeBinding
  } catch (cause) {
    throw new NativeBindingLoadError(cause)
  }

  return binding
}
