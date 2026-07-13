import { readFileSync } from 'node:fs'
import { NativeBindingLoadError, loadNativeBinding } from './native-loader'
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
} from './types'
import { loadWasmRuntime } from './wasm-fallback'

interface NativeSubsetOptions {
  text?: string
  unicodes?: number[]
  unicodeRanges?: string[]
  basicText?: boolean
  preserveHinting?: boolean
  trim?: boolean
  keepNotdef?: boolean
  keepLayout?: string
}

interface NativeCssOptions {
  asFileName?: boolean
  base64?: boolean
  fontFamily?: string
  fontPath?: string
  glyph?: boolean
  iconPrefix?: string
  local?: boolean
  fontDisplay?: string
  target?: NonNullable<CssOptions['target']>
  unicodeRanges?: string[]
}

interface NativeCssFontSource {
  contents?: Buffer
  fileName: string
  format: CssFontSource['format']
  glyphs?: NonNullable<CssFontSource['glyphs']>
  unicodeRanges?: string[]
}

interface NativeWoff2Options {
  quality?: number
}

interface NativeWoffOptions {
  compressionLevel?: number
  deflate?: boolean
  metadata?: string
  privateData?: Buffer
}

interface NativeEotOptions {
  version?: number
}

interface NativeOtf2TtfOptions {
  preserveHinting?: boolean
  variationCoordinates?: Record<string, number>
}

interface NativeSvgOptions {
  fontFamily?: string
}

interface NativeSvg2TtfOptions {
  hinting?: boolean
  normalize?: boolean
}

interface NativeSvgIcon {
  name: string
  contents: string
  unicode?: number
}

interface NativeSvgs2TtfOptions {
  fontName?: string
  startUnicode?: number
  ascent?: number
  descent?: number
  normalize?: boolean
}

function assignDefined<Target extends object, Key extends keyof Target>(
  target: Target,
  key: Key,
  value: Target[Key] | undefined,
): void {
  if (value !== undefined) {
    target[key] = value
  }
}

function assertAvailableWoff2Fallback(
  fallback: Ttf2Woff2Options['fallback'] | undefined,
): void {
  if (fallback === undefined || fallback === 'native' || fallback === 'auto') {
    return
  }

  throw new Error(
    fallback === 'wasm'
      ? 'WOFF2 fallback `wasm` is asynchronous; use ttfToWoff2Async() instead.'
      : unavailableWoff2Fallback(fallback).message,
  )
}

function unavailableWoff2Fallback(fallback: 'js'): Error {
  return new Error(
    `WOFF2 fallback \`${fallback}\` is not available in this build`,
  )
}

function toNativeSvgIcon(input: SvgIcon): NativeSvgIcon {
  const nativeInput: NativeSvgIcon = {
    contents: input.contents,
    name: input.name,
  }

  assignDefined(nativeInput, 'unicode', input.unicode)

  return nativeInput
}

function toNativeSvg2TtfOptions(options: Svg2TtfOptions): NativeSvg2TtfOptions {
  const nativeOptions: NativeSvg2TtfOptions = {}

  assignDefined(nativeOptions, 'hinting', options.hinting)
  assignDefined(nativeOptions, 'normalize', options.normalize)

  return nativeOptions
}

function toNativeSvgs2TtfOptions(
  options: Svgs2TtfOptions,
): NativeSvgs2TtfOptions {
  const nativeOptions: NativeSvgs2TtfOptions = {}

  assignDefined(nativeOptions, 'fontName', options.fontName)
  assignDefined(nativeOptions, 'startUnicode', options.startUnicode)
  assignDefined(nativeOptions, 'ascent', options.ascent)
  assignDefined(nativeOptions, 'descent', options.descent)
  assignDefined(nativeOptions, 'normalize', options.normalize)

  return nativeOptions
}

function toNativeWoffOptions(options: WoffOptions): NativeWoffOptions {
  const nativeOptions: NativeWoffOptions = {}

  assignDefined(nativeOptions, 'deflate', options.deflate)
  assignDefined(nativeOptions, 'compressionLevel', options.compressionLevel)
  assignDefined(nativeOptions, 'metadata', options.metadata)

  if (options.privateData !== undefined) {
    nativeOptions.privateData = Buffer.isBuffer(options.privateData)
      ? options.privateData
      : Buffer.from(options.privateData)
  }

  return nativeOptions
}

export function subsetTtf(
  input: Uint8Array,
  options: SubsetOptions = {},
): Buffer {
  const nativeOptions: NativeSubsetOptions = {}
  const text = resolveSubsetText(options)

  if (text !== undefined) {
    nativeOptions.text = text
  }
  if (options.unicodes !== undefined) {
    nativeOptions.unicodes = options.unicodes
  }
  if (options.unicodeRanges !== undefined) {
    nativeOptions.unicodeRanges = options.unicodeRanges
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

  return loadNativeBinding().subsetTtf(inputBuffer, nativeOptions)
}

function resolveSubsetText(options: SubsetOptions): string | undefined {
  if (options.textFile === undefined) {
    return options.text
  }

  const fileText = readFileSync(options.textFile, 'utf8')

  return options.text === undefined ? fileText : `${options.text}${fileText}`
}

export function inspect(input: Uint8Array): FontInfo {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return loadNativeBinding().inspectFont(inputBuffer) as FontInfo
}

export function ttfToWoff(
  input: Uint8Array,
  options: WoffOptions = {},
): Buffer {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return loadNativeBinding().ttfToWoff(
    inputBuffer,
    toNativeWoffOptions(options),
  )
}

export function woffToTtf(input: Uint8Array): Buffer {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return loadNativeBinding().woffToTtf(inputBuffer)
}

export function woff2ToTtf(input: Uint8Array): Buffer {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return loadNativeBinding().woff2ToTtf(inputBuffer)
}

export function eotToTtf(input: Uint8Array): Buffer {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return loadNativeBinding().eotToTtf(inputBuffer)
}

export function otfToTtf(
  input: Uint8Array,
  options: Otf2TtfOptions = {},
): Buffer {
  const nativeOptions: NativeOtf2TtfOptions = {}

  assignDefined(nativeOptions, 'preserveHinting', options.preserveHinting)
  assignDefined(
    nativeOptions,
    'variationCoordinates',
    options.variationCoordinates,
  )

  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return loadNativeBinding().otfToTtf(inputBuffer, nativeOptions)
}

export function ttfToWoff2(
  input: Uint8Array,
  options: Ttf2Woff2Options = {},
): Buffer {
  const nativeOptions: NativeWoff2Options = {}

  assertAvailableWoff2Fallback(options.fallback)

  if (options.quality !== undefined) {
    nativeOptions.quality = options.quality
  }

  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return loadNativeBinding().ttfToWoff2(inputBuffer, nativeOptions)
}

export async function ttfToWoff2Async(
  input: Uint8Array,
  options: Ttf2Woff2Options = {},
): Promise<Buffer> {
  if (options.fallback === 'wasm') {
    return ttfToWoff2WithWasm(input, options)
  }
  if (options.fallback === 'js') {
    throw unavailableWoff2Fallback('js')
  }

  try {
    return ttfToWoff2(input, { ...options, fallback: 'native' })
  } catch (error) {
    if (
      options.fallback === 'native' ||
      !(error instanceof NativeBindingLoadError)
    ) {
      throw error
    }

    return ttfToWoff2WithWasm(input, options)
  }
}

async function ttfToWoff2WithWasm(
  input: Uint8Array,
  options: Ttf2Woff2Options,
): Promise<Buffer> {
  const wasm = await loadWasmRuntime()
  const wasmOptions =
    options.quality === undefined ? {} : { quality: options.quality }

  try {
    const output = await wasm.ttfToWoff2(input, wasmOptions)

    return Buffer.from(output)
  } catch (error) {
    throw new Error('WOFF2 WASM fallback failed', { cause: error })
  }
}

export function validateWoff2(input: Uint8Array): void {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  loadNativeBinding().validateWoff2(inputBuffer)
}

export function ttfToEot(
  input: Uint8Array,
  options: Ttf2EotOptions = {},
): Buffer {
  const nativeOptions: NativeEotOptions = {}

  if (options.version !== undefined) {
    nativeOptions.version = options.version
  }

  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return loadNativeBinding().ttfToEot(inputBuffer, nativeOptions)
}

export function ttfToSvg(
  input: Uint8Array,
  options: Ttf2SvgOptions = {},
): string {
  const nativeOptions: NativeSvgOptions = {}

  if (options.fontFamily !== undefined) {
    nativeOptions.fontFamily = options.fontFamily
  }

  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return loadNativeBinding().ttfToSvg(inputBuffer, nativeOptions)
}

export function svgFontToTtf(
  input: string,
  options: Svg2TtfOptions = {},
): Buffer {
  return loadNativeBinding().svgFontToTtf(
    input,
    toNativeSvg2TtfOptions(options),
  )
}

export function svgsToTtf(
  inputs: SvgIcon[],
  options: Svgs2TtfOptions = {},
): Buffer {
  return loadNativeBinding().svgsToTtf(
    inputs.map(input => toNativeSvgIcon(input)),
    toNativeSvgs2TtfOptions(options),
  )
}

export function generateFontFaceCss(
  sources: CssFontSource[],
  options: CssOptions = {},
): string {
  const nativeSources = sources.map(source => toNativeCssFontSource(source))
  const nativeOptions: NativeCssOptions = {}

  if (options.fontFamily !== undefined) {
    nativeOptions.fontFamily = resolveCssFontFamily(sources, options.fontFamily)
  }
  if (options.fontPath !== undefined) {
    nativeOptions.fontPath = options.fontPath
  }
  if (options.base64 !== undefined) {
    nativeOptions.base64 = options.base64
  }
  if (options.glyph !== undefined) {
    nativeOptions.glyph = options.glyph
  }
  if (options.iconPrefix !== undefined) {
    nativeOptions.iconPrefix = options.iconPrefix
  }
  if (options.asFileName !== undefined) {
    nativeOptions.asFileName = options.asFileName
  }
  if (options.local !== undefined) {
    nativeOptions.local = options.local
  }
  if (options.fontDisplay !== undefined) {
    nativeOptions.fontDisplay = options.fontDisplay
  }
  if (options.target !== undefined) {
    nativeOptions.target = options.target
  }
  if (options.unicodeRanges !== undefined) {
    nativeOptions.unicodeRanges = options.unicodeRanges
  }

  return loadNativeBinding().generateFontFaceCss(nativeSources, nativeOptions)
}

function resolveCssFontFamily(
  sources: CssFontSource[],
  fontFamily: NonNullable<CssOptions['fontFamily']>,
): string {
  if (typeof fontFamily === 'string') {
    return fontFamily
  }

  const source = sources.find(source => source.contents !== undefined)

  if (source?.contents === undefined) {
    throw new Error('CSS fontFamily resolver requires source contents')
  }

  return fontFamily(inspect(source.contents))
}

function toNativeCssFontSource(source: CssFontSource): NativeCssFontSource {
  const nativeSource: NativeCssFontSource = {
    fileName: source.fileName,
    format: source.format,
  }

  if (source.contents !== undefined) {
    nativeSource.contents = Buffer.isBuffer(source.contents)
      ? source.contents
      : Buffer.from(source.contents)
  }
  if (source.glyphs !== undefined) {
    nativeSource.glyphs = source.glyphs
  }
  if (source.unicodeRanges !== undefined) {
    nativeSource.unicodeRanges = source.unicodeRanges
  }

  return nativeSource
}
