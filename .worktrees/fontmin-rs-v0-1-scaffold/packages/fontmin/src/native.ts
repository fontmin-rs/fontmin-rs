import { subsetTtf as nativeSubsetTtf } from '@fontmin-rs/binding'
import type { SubsetOptions } from './types'

interface NativeSubsetOptions {
  text?: string
  unicodes?: number[]
  basicText?: boolean
  preserveHinting?: boolean
  trim?: boolean
  keepNotdef?: boolean
  keepLayout?: string
}

export function subsetTtf(
  input: Uint8Array,
  options: SubsetOptions = {},
): Buffer {
  const nativeOptions: NativeSubsetOptions = {}

  if (options.text !== undefined) {
    nativeOptions.text = options.text
  }
  if (options.unicodes !== undefined) {
    nativeOptions.unicodes = options.unicodes
  }
  if (options.basicText !== undefined) {
    nativeOptions.basicText = options.basicText
  }
  if (options.trim !== undefined) {
    nativeOptions.trim = options.trim
  }
  if (options.keepNotdef !== undefined) {
    nativeOptions.keepNotdef = options.keepNotdef
  }
  if (options.keepLayout !== undefined) {
    nativeOptions.keepLayout = options.keepLayout
  }

  const preserveHinting = options.preserveHinting ?? options.hinting
  if (preserveHinting !== undefined) {
    nativeOptions.preserveHinting = preserveHinting
  }

  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return nativeSubsetTtf(inputBuffer, nativeOptions)
}
