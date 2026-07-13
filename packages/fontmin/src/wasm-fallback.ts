import { readFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'
import type {
  CssFontSource,
  CssOptions,
  FontInfo,
  Otf2TtfOptions,
  Svg2TtfOptions,
  SvgIcon,
  Svgs2TtfOptions,
  Ttf2EotOptions,
  Ttf2SvgOptions,
  Ttf2Woff2Options,
  WoffOptions,
} from './types'

export interface WasmRuntime {
  generateFontFaceCss(
    sources: CssFontSource[],
    options?: Omit<CssOptions, 'fontFamily'> & { fontFamily?: string },
  ): Promise<string>
  inspect(input: Uint8Array): Promise<FontInfo>
  initWasm(input?: Uint8Array): Promise<void>
  otfToTtf(input: Uint8Array, options?: Otf2TtfOptions): Promise<Uint8Array>
  subsetTtf(
    input: Uint8Array,
    options?: Record<string, unknown>,
  ): Promise<Uint8Array>
  svgFontToTtf(input: string, options?: Svg2TtfOptions): Promise<Uint8Array>
  svgsToTtf(inputs: SvgIcon[], options?: Svgs2TtfOptions): Promise<Uint8Array>
  ttfToEot(input: Uint8Array, options?: Ttf2EotOptions): Promise<Uint8Array>
  ttfToSvg(input: Uint8Array, options?: Ttf2SvgOptions): Promise<string>
  ttfToWoff(input: Uint8Array, options?: WoffOptions): Promise<Uint8Array>
  ttfToWoff2(input: Uint8Array, options?: Ttf2Woff2Options): Promise<Uint8Array>
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
  } catch (error) {
    throw new Error('fontmin-rs WASM runtime failed to initialize', {
      cause: error,
    })
  }
}
