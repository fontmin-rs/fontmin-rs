import { getWasmModule } from './runtime'

export interface FontMetadata {
  ascender: number
  descender: number
  familyName?: string
  fullName?: string
  glyphCount: number
  postScriptName?: string
  subfamilyName?: string
  tables: string[]
  unitsPerEm: number
}

export interface FontInfo {
  format: string
  metadata: FontMetadata
  size: number
}

export interface CssFontSource {
  contents?: Uint8Array
  fileName: string
  format: 'eot' | 'svg' | 'ttf' | 'woff' | 'woff2'
  glyphs?: { name?: string; unicode: number }[]
  unicodeRanges?: string[]
}

export interface CssOptions {
  asFileName?: boolean
  base64?: boolean
  fontDisplay?: 'auto' | 'block' | 'swap' | 'fallback' | 'optional'
  fontFamily?: string
  fontPath?: string
  glyph?: boolean
  iconPrefix?: string
  local?: boolean
  target?: 'css' | 'scss' | 'less'
  unicodeRanges?: string[]
}

export interface SvgIcon {
  contents: string
  name: string
  unicode?: number
}

type Options = Record<string, unknown>

function bytes(value: unknown): Uint8Array {
  return new Uint8Array(value as ArrayLike<number>)
}

async function binary(
  operation: string,
  input: Uint8Array,
  options: Options = {},
): Promise<Uint8Array> {
  const wasm = await getWasmModule()
  return bytes(wasm.transform(operation, input, options))
}

export async function subsetTtf(
  input: Uint8Array,
  options: Options = {},
): Promise<Uint8Array> {
  return binary('subsetTtf', input, options)
}

export async function ttfToWoff(
  input: Uint8Array,
  options: Options = {},
): Promise<Uint8Array> {
  return binary('ttfToWoff', input, options)
}

export async function woffToTtf(input: Uint8Array): Promise<Uint8Array> {
  return binary('woffToTtf', input)
}

export async function ttfToWoff2(
  input: Uint8Array,
  options: Options = {},
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
  options: Options = {},
): Promise<Uint8Array> {
  return binary('ttfToEot', input, options)
}

export async function eotToTtf(input: Uint8Array): Promise<Uint8Array> {
  return binary('eotToTtf', input)
}

export async function ttfToSvg(
  input: Uint8Array,
  options: Options = {},
): Promise<string> {
  const wasm = await getWasmModule()
  return wasm.transform('ttfToSvg', input, options) as string
}

export async function svgFontToTtf(
  input: string,
  options: Options = {},
): Promise<Uint8Array> {
  const wasm = await getWasmModule()
  return bytes(wasm.transform_text('svgFontToTtf', input, options))
}

export async function svgsToTtf(
  inputs: SvgIcon[],
  options: Options = {},
): Promise<Uint8Array> {
  const wasm = await getWasmModule()
  return bytes(wasm.transform_icons(inputs, options))
}

export async function otfToTtf(
  input: Uint8Array,
  options: Options = {},
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
