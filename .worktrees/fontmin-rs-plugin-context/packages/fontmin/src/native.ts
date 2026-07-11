import {
  eotToTtf as nativeEotToTtf,
  generateFontFaceCss as nativeGenerateFontFaceCss,
  inspectFont as nativeInspectFont,
  otfToTtf as nativeOtfToTtf,
  subsetTtf as nativeSubsetTtf,
  svgFontToTtf as nativeSvgFontToTtf,
  svgsToTtf as nativeSvgsToTtf,
  ttfToEot as nativeTtfToEot,
  ttfToSvg as nativeTtfToSvg,
  ttfToWoff as nativeTtfToWoff,
  ttfToWoff2 as nativeTtfToWoff2,
  woffToTtf as nativeWoffToTtf,
} from '@fontmin-rs/binding'
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

interface NativeSubsetOptions {
  text?: string
  unicodes?: number[]
  basicText?: boolean
  preserveHinting?: boolean
  trim?: boolean
  keepNotdef?: boolean
  keepLayout?: string
}

interface NativeCssOptions {
  fontFamily?: string
  fontPath?: string
  local?: boolean
  fontDisplay?: string
}

interface NativeWoff2Options {
  quality?: number
}

interface NativeEotOptions {
  version?: number
}

interface NativeOtf2TtfOptions {
  preserveHinting?: boolean
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

export function inspect(input: Uint8Array): FontInfo {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return nativeInspectFont(inputBuffer) as FontInfo
}

export function ttfToWoff(
  input: Uint8Array,
  options: WoffOptions = {},
): Buffer {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return nativeTtfToWoff(inputBuffer, options)
}

export function woffToTtf(input: Uint8Array): Buffer {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return nativeWoffToTtf(inputBuffer)
}

export function eotToTtf(input: Uint8Array): Buffer {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return nativeEotToTtf(inputBuffer)
}

export function otfToTtf(
  input: Uint8Array,
  options: Otf2TtfOptions = {},
): Buffer {
  const nativeOptions: NativeOtf2TtfOptions = {}

  assignDefined(nativeOptions, 'preserveHinting', options.preserveHinting)

  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return nativeOtfToTtf(inputBuffer, nativeOptions)
}

export function ttfToWoff2(
  input: Uint8Array,
  options: Ttf2Woff2Options = {},
): Buffer {
  const nativeOptions: NativeWoff2Options = {}

  if (options.quality !== undefined) {
    nativeOptions.quality = options.quality
  }

  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return nativeTtfToWoff2(inputBuffer, nativeOptions)
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

  return nativeTtfToEot(inputBuffer, nativeOptions)
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

  return nativeTtfToSvg(inputBuffer, nativeOptions)
}

export function svgFontToTtf(
  input: string,
  options: Svg2TtfOptions = {},
): Buffer {
  return nativeSvgFontToTtf(input, toNativeSvg2TtfOptions(options))
}

export function svgsToTtf(
  inputs: SvgIcon[],
  options: Svgs2TtfOptions = {},
): Buffer {
  return nativeSvgsToTtf(
    inputs.map(input => toNativeSvgIcon(input)),
    toNativeSvgs2TtfOptions(options),
  )
}

export function generateFontFaceCss(
  sources: CssFontSource[],
  options: CssOptions = {},
): string {
  const nativeOptions: NativeCssOptions = {}

  if (options.fontFamily !== undefined) {
    nativeOptions.fontFamily = options.fontFamily
  }
  if (options.fontPath !== undefined) {
    nativeOptions.fontPath = options.fontPath
  }
  if (options.local !== undefined) {
    nativeOptions.local = options.local
  }
  if (options.fontDisplay !== undefined) {
    nativeOptions.fontDisplay = options.fontDisplay
  }

  return nativeGenerateFontFaceCss(sources, nativeOptions)
}
