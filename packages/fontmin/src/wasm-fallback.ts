import { readFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'
import type {
  CssFontSource,
  CssOptions as WasmCssOptions,
  FontCapabilityReport,
  FontInfo,
  FontCollectionInfo,
  InstanceOptions,
  Otf2TtfOptions,
  SubsetOptions as WasmSubsetOptions,
  SubsetPlan,
  SubsetResult,
  Svg2TtfOptions,
  SvgIcon,
  Svgs2TtfOptions,
  Ttf2EotOptions,
  Ttf2SvgOptions,
  Ttf2Woff2Options,
  VariationSpaceOptions,
  WoffOptions,
} from '../../../wasm/fontmin/types'

export interface WasmRuntime {
  eotToTtf(input: Uint8Array): Promise<Uint8Array>
  generateFontFaceCss(
    sources: CssFontSource[],
    options?: WasmCssOptions,
  ): Promise<string>
  initWasm(input?: Uint8Array): Promise<void>
  extractCollectionFace(
    input: Uint8Array,
    faceIndex: number,
  ): Promise<Uint8Array>
  inspect(input: Uint8Array): Promise<FontInfo>
  inspectCapabilities(input: Uint8Array): Promise<FontCapabilityReport>
  inspectCollection(input: Uint8Array): Promise<FontCollectionInfo>
  instantiateFont(
    input: Uint8Array,
    options?: InstanceOptions,
  ): Promise<Uint8Array>
  otfToTtf(input: Uint8Array, options?: Otf2TtfOptions): Promise<Uint8Array>
  reduceVariationSpace(
    input: Uint8Array,
    options: VariationSpaceOptions,
  ): Promise<Uint8Array>
  subsetTtf(input: Uint8Array, options?: WasmSubsetOptions): Promise<Uint8Array>
  createTtfSubsetPlan(
    input: Uint8Array,
    options?: WasmSubsetOptions,
  ): Promise<SubsetPlan>
  subsetTtfWithPlan(input: Uint8Array, plan: SubsetPlan): Promise<SubsetResult>
  subsetTtfWithReport(
    input: Uint8Array,
    options?: WasmSubsetOptions,
  ): Promise<SubsetResult>
  svgFontToTtf(input: string, options?: Svg2TtfOptions): Promise<Uint8Array>
  svgsToTtf(inputs: SvgIcon[], options?: Svgs2TtfOptions): Promise<Uint8Array>
  ttfToEot(input: Uint8Array, options?: Ttf2EotOptions): Promise<Uint8Array>
  ttfToSvg(input: Uint8Array, options?: Ttf2SvgOptions): Promise<string>
  ttfToWoff(input: Uint8Array, options?: WoffOptions): Promise<Uint8Array>
  ttfToWoff2(input: Uint8Array, options?: Ttf2Woff2Options): Promise<Uint8Array>
  validateWoff2(input: Uint8Array): Promise<void>
  woff2ToTtf(input: Uint8Array): Promise<Uint8Array>
  woffToTtf(input: Uint8Array): Promise<Uint8Array>
}

const require = createRequire(import.meta.url)
const wasmPackageName = '@fontmin-rs/wasm'
let runtime: Promise<WasmRuntime> | undefined

export async function loadWasmRuntime(): Promise<WasmRuntime> {
  const attempt = (runtime ??= initializeWasmRuntime())

  try {
    return await attempt
  } catch (error) {
    if (runtime === attempt) {
      runtime = undefined
    }

    throw error
  }
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
