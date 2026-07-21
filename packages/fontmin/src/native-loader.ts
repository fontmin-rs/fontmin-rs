import { createRequire } from 'node:module'
import type * as nativeBindingModule from '@fontmin-rs/binding'

type NativeBinding = typeof nativeBindingModule

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
  } catch (error) {
    throw new NativeBindingLoadError(error)
  }

  return binding
}
