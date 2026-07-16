import type {
  CssFontSource,
  CssOptions,
  FontInfo,
  Otf2TtfOptions,
  SubsetOptions,
  Svg2TtfOptions,
  SvgIcon,
  Svgs2TtfOptions,
  Ttf2EotOptions,
  Ttf2SvgOptions,
  Ttf2Woff2Options,
  WoffOptions,
} from '../types'
import { getWasmModule } from './runtime'

function bytes(value: unknown): Uint8Array {
  return new Uint8Array(value as ArrayLike<number>)
}

async function binary(
  operation: string,
  input: Uint8Array,
  options: object = {},
): Promise<Uint8Array> {
  const wasm = await getWasmModule()
  return bytes(wasm.transform(operation, input, options))
}

export async function subsetTtf(
  input: Uint8Array,
  options: SubsetOptions = {},
): Promise<Uint8Array> {
  return binary('subsetTtf', input, options)
}

export async function ttfToWoff(
  input: Uint8Array,
  options: WoffOptions = {},
): Promise<Uint8Array> {
  return binary('ttfToWoff', input, options)
}

export async function woffToTtf(input: Uint8Array): Promise<Uint8Array> {
  return binary('woffToTtf', input)
}

export async function ttfToWoff2(
  input: Uint8Array,
  options: Ttf2Woff2Options = {},
): Promise<Uint8Array> {
  return binary('ttfToWoff2', input, options)
}

export async function woff2ToTtf(input: Uint8Array): Promise<Uint8Array> {
  return binary('woff2ToTtf', input)
}

export async function validateWoff2(input: Uint8Array): Promise<void> {
  const wasm = await getWasmModule()
  wasm.transform('validateWoff2', input, {})
}

export async function ttfToEot(
  input: Uint8Array,
  options: Ttf2EotOptions = {},
): Promise<Uint8Array> {
  return binary('ttfToEot', input, options)
}

export async function eotToTtf(input: Uint8Array): Promise<Uint8Array> {
  return binary('eotToTtf', input)
}

export async function ttfToSvg(
  input: Uint8Array,
  options: Ttf2SvgOptions = {},
): Promise<string> {
  const wasm = await getWasmModule()
  return wasm.transform('ttfToSvg', input, options) as string
}

export async function svgFontToTtf(
  input: string,
  options: Svg2TtfOptions = {},
): Promise<Uint8Array> {
  const wasm = await getWasmModule()
  return bytes(wasm.transform_text('svgFontToTtf', input, options))
}

export async function svgsToTtf(
  inputs: SvgIcon[],
  options: Svgs2TtfOptions = {},
): Promise<Uint8Array> {
  const wasm = await getWasmModule()
  return bytes(wasm.transform_icons(inputs, options))
}

export async function otfToTtf(
  input: Uint8Array,
  options: Otf2TtfOptions = {},
): Promise<Uint8Array> {
  return binary('otfToTtf', input, options)
}

export async function inspect(input: Uint8Array): Promise<FontInfo> {
  const wasm = await getWasmModule()
  return wasm.transform('inspect', input, {}) as FontInfo
}

export async function generateFontFaceCss(
  sources: CssFontSource[],
  options: CssOptions = {},
): Promise<string> {
  const wasm = await getWasmModule()
  return wasm.generate_css(sources, options) as string
}
