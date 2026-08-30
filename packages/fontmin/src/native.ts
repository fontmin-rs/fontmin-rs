import { readFileSync } from 'node:fs'
import { readFile } from 'node:fs/promises'
import {
  withFontminDiagnostics,
  withFontminDiagnosticsAsync,
} from './diagnostics'
import { NativeBindingLoadError, loadNativeBinding } from './native-loader'
import { missingGlyphWarning } from './runtime-neutral/optimize-policy'
import type {
  CoverageOptions,
  CoverageReport,
  CssFontSource,
  CssOptions,
  FontCapabilityReport,
  FontCollectionInfo,
  FontInfo,
  InstanceOptions,
  Otf2TtfOptions,
  SubsetOptions,
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
} from './types'
import { loadWasmRuntime } from './wasm-fallback'

interface NativeSubsetOptions {
  text?: string
  unicodes?: number[]
  gids?: number[]
  glyphNames?: string[]
  unicodeRanges?: string[]
  basicText?: boolean
  preserveHinting?: boolean
  trim?: boolean
  keepNotdef?: boolean
  retainGids?: boolean
  retainGlyphNames?: boolean
  retainLegacyCmap?: boolean
  retainSymbolCmap?: boolean
  keepLayout?: string
  layoutFeatures?: string[]
  layoutScripts?: string[]
  layoutLanguages?: string[]
  nameIds?: number[]
  nameLanguages?: number[]
  dropTables?: string[]
  passThroughTables?: string[]
  missingGlyphs?: string
}

interface NativeCoverageOptions {
  basicText?: boolean
  text?: string
  unicodeRanges?: string[]
  unicodes?: number[]
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

interface NativeAxisRange {
  default?: number
  max: number
  min: number
}

interface NativeVariationSpaceOptions {
  downgradeCff2?: boolean
  pins?: Record<string, number>
  ranges?: Record<string, NativeAxisRange>
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
  const nativeOptions = toNativeSubsetOptions(options)

  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)
  const binding = loadNativeBinding()

  if (
    (options.missingGlyphs ?? 'warn') === 'warn' &&
    hasUnicodeSelection(options)
  ) {
    const report = withFontminDiagnostics(
      () =>
        binding.analyzeCoverage(
          inputBuffer,
          coverageOptionsFromSubset(nativeOptions),
        ) as CoverageReport,
    )
    const warning = missingGlyphWarning(report)

    if (warning !== undefined) {
      process.emitWarning(warning, { code: 'FONTMIN_MISSING_GLYPHS' })
    }
  }

  return withFontminDiagnostics(() =>
    binding.subsetTtf(inputBuffer, nativeOptions),
  )
}

export async function subsetTtfAsync(
  input: Uint8Array,
  options: SubsetOptions = {},
): Promise<Buffer> {
  const nativeOptions = toNativeSubsetOptions(
    options,
    await resolveSubsetTextAsync(options),
  )
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)
  const binding = loadNativeBinding()

  if (
    (options.missingGlyphs ?? 'warn') === 'warn' &&
    hasUnicodeSelection(options)
  ) {
    const report = (await withFontminDiagnosticsAsync(() =>
      binding.analyzeCoverageAsync(
        inputBuffer,
        coverageOptionsFromSubset(nativeOptions),
      ),
    )) as CoverageReport
    const warning = missingGlyphWarning(report)

    if (warning !== undefined) {
      process.emitWarning(warning, { code: 'FONTMIN_MISSING_GLYPHS' })
    }
  }

  return withFontminDiagnosticsAsync(() =>
    binding.subsetTtfAsync(inputBuffer, nativeOptions),
  )
}

export function subsetTtfWithReport(
  input: Uint8Array,
  options: SubsetOptions = {},
): SubsetResult {
  const nativeOptions = toNativeSubsetOptions(options)
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnostics(
    () =>
      loadNativeBinding().subsetTtfWithReport(
        inputBuffer,
        nativeOptions,
      ) as SubsetResult,
  )
}

export function createTtfSubsetPlan(
  input: Uint8Array,
  options: SubsetOptions = {},
): SubsetPlan {
  const nativeOptions = toNativeSubsetOptions(options)
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnostics(
    () =>
      loadNativeBinding().createTtfSubsetPlan(
        inputBuffer,
        nativeOptions,
      ) as SubsetPlan,
  )
}

export function subsetTtfWithPlan(
  input: Uint8Array,
  plan: SubsetPlan,
): SubsetResult {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnostics(
    () =>
      loadNativeBinding().subsetTtfWithPlan(inputBuffer, plan) as SubsetResult,
  )
}

function toNativeSubsetOptions(
  options: SubsetOptions,
  text = resolveSubsetText(options),
): NativeSubsetOptions {
  const nativeOptions: NativeSubsetOptions = {}

  assignDefined(nativeOptions, 'text', text)
  assignDefined(nativeOptions, 'unicodes', options.unicodes)
  assignDefined(nativeOptions, 'gids', options.gids)
  assignDefined(nativeOptions, 'glyphNames', options.glyphNames)
  assignDefined(nativeOptions, 'unicodeRanges', options.unicodeRanges)
  assignDefined(nativeOptions, 'basicText', options.basicText)
  assignDefined(nativeOptions, 'trim', options.trim)
  assignDefined(nativeOptions, 'keepNotdef', options.keepNotdef)
  assignDefined(nativeOptions, 'retainGids', options.retainGids)
  assignDefined(nativeOptions, 'retainGlyphNames', options.retainGlyphNames)
  assignDefined(nativeOptions, 'retainLegacyCmap', options.retainLegacyCmap)
  assignDefined(nativeOptions, 'retainSymbolCmap', options.retainSymbolCmap)
  assignDefined(nativeOptions, 'keepLayout', options.keepLayout)
  assignDefined(nativeOptions, 'layoutFeatures', options.layoutFeatures)
  assignDefined(nativeOptions, 'layoutScripts', options.layoutScripts)
  assignDefined(nativeOptions, 'layoutLanguages', options.layoutLanguages)
  assignDefined(nativeOptions, 'nameIds', options.nameIds)
  assignDefined(nativeOptions, 'nameLanguages', options.nameLanguages)
  assignDefined(nativeOptions, 'dropTables', options.dropTables)
  assignDefined(nativeOptions, 'passThroughTables', options.passThroughTables)
  assignDefined(nativeOptions, 'missingGlyphs', options.missingGlyphs)
  assignDefined(
    nativeOptions,
    'preserveHinting',
    options.preserveHinting ?? options.hinting,
  )

  return nativeOptions
}

function hasUnicodeSelection(options: SubsetOptions): boolean {
  return (
    options.basicText === true ||
    (options.text?.length ?? 0) > 0 ||
    options.textFile !== undefined ||
    (options.unicodes?.length ?? 0) > 0 ||
    (options.unicodeRanges?.length ?? 0) > 0
  )
}

function resolveSubsetText(options: SubsetOptions): string | undefined {
  if (options.textFile === undefined) {
    return options.text
  }

  const fileText = readFileSync(options.textFile, 'utf8')

  return options.text === undefined ? fileText : `${options.text}${fileText}`
}

async function resolveSubsetTextAsync(
  options: SubsetOptions,
): Promise<string | undefined> {
  if (options.textFile === undefined) {
    return options.text
  }

  const fileText = await readFile(options.textFile, 'utf8')

  return options.text === undefined ? fileText : `${options.text}${fileText}`
}

export function analyzeCoverage(
  input: Uint8Array,
  options: CoverageOptions = {},
): CoverageReport {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnostics(
    () =>
      loadNativeBinding().analyzeCoverage(
        inputBuffer,
        toNativeCoverageOptions(options),
      ) as CoverageReport,
  )
}

export function inspect(input: Uint8Array): FontInfo {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnostics(
    () => loadNativeBinding().inspectFont(inputBuffer) as FontInfo,
  )
}

export async function inspectAsync(input: Uint8Array): Promise<FontInfo> {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return (await withFontminDiagnosticsAsync(() =>
    loadNativeBinding().inspectFontAsync(inputBuffer),
  )) as FontInfo
}

export function inspectCapabilities(input: Uint8Array): FontCapabilityReport {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnostics(
    () =>
      loadNativeBinding().inspectCapabilities(
        inputBuffer,
      ) as FontCapabilityReport,
  )
}

export function inspectCollection(input: Uint8Array): FontCollectionInfo {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnostics(
    () =>
      loadNativeBinding().inspectCollection(inputBuffer) as FontCollectionInfo,
  )
}

export function extractCollectionFace(
  input: Uint8Array,
  faceIndex: number,
): Buffer {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnostics(() =>
    loadNativeBinding().extractCollectionFace(inputBuffer, faceIndex),
  )
}

export function instantiateFont(
  input: Uint8Array,
  options: InstanceOptions = {},
): Buffer {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnostics(() =>
    loadNativeBinding().instantiateFont(inputBuffer, options),
  )
}

export async function instantiateFontAsync(
  input: Uint8Array,
  options: InstanceOptions = {},
): Promise<Buffer> {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnosticsAsync(() =>
    loadNativeBinding().instantiateFontAsync(inputBuffer, options),
  )
}

export function reduceVariationSpace(
  input: Uint8Array,
  options: VariationSpaceOptions,
): Buffer {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnostics(() =>
    loadNativeBinding().reduceVariationSpace(
      inputBuffer,
      toNativeVariationSpaceOptions(options),
    ),
  )
}

export async function reduceVariationSpaceAsync(
  input: Uint8Array,
  options: VariationSpaceOptions,
): Promise<Buffer> {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnosticsAsync(() =>
    loadNativeBinding().reduceVariationSpaceAsync(
      inputBuffer,
      toNativeVariationSpaceOptions(options),
    ),
  )
}

function toNativeVariationSpaceOptions(
  options: VariationSpaceOptions,
): NativeVariationSpaceOptions {
  const pins: Record<string, number> = {}
  const ranges: Record<string, NativeAxisRange> = {}

  for (const [tag, setting] of Object.entries(options.axes)) {
    if (typeof setting === 'number') {
      pins[tag] = setting
    } else {
      ranges[tag] = setting
    }
  }

  return {
    ...(options.downgradeCff2 === undefined
      ? {}
      : { downgradeCff2: options.downgradeCff2 }),
    ...(Object.keys(pins).length === 0 ? {} : { pins }),
    ...(Object.keys(ranges).length === 0 ? {} : { ranges }),
  }
}

function toNativeCoverageOptions(
  options: CoverageOptions,
): NativeCoverageOptions {
  const nativeOptions: NativeCoverageOptions = {}

  assignDefined(nativeOptions, 'basicText', options.basicText)
  assignDefined(nativeOptions, 'text', options.text)
  assignDefined(nativeOptions, 'unicodeRanges', options.unicodeRanges)
  assignDefined(nativeOptions, 'unicodes', options.unicodes)

  return nativeOptions
}

function coverageOptionsFromSubset(
  options: NativeSubsetOptions,
): NativeCoverageOptions {
  return toNativeCoverageOptions(options)
}

export function ttfToWoff(
  input: Uint8Array,
  options: WoffOptions = {},
): Buffer {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnostics(() =>
    loadNativeBinding().ttfToWoff(inputBuffer, toNativeWoffOptions(options)),
  )
}

export async function ttfToWoffAsync(
  input: Uint8Array,
  options: WoffOptions = {},
): Promise<Buffer> {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnosticsAsync(() =>
    loadNativeBinding().ttfToWoffAsync(
      inputBuffer,
      toNativeWoffOptions(options),
    ),
  )
}

export function woffToTtf(input: Uint8Array): Buffer {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnostics(() =>
    loadNativeBinding().woffToTtf(inputBuffer),
  )
}

export async function woffToTtfAsync(input: Uint8Array): Promise<Buffer> {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnosticsAsync(() =>
    loadNativeBinding().woffToTtfAsync(inputBuffer),
  )
}

export function woff2ToTtf(input: Uint8Array): Buffer {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnostics(() =>
    loadNativeBinding().woff2ToTtf(inputBuffer),
  )
}

export async function woff2ToTtfAsync(input: Uint8Array): Promise<Buffer> {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnosticsAsync(() =>
    loadNativeBinding().woff2ToTtfAsync(inputBuffer),
  )
}

export function eotToTtf(input: Uint8Array): Buffer {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnostics(() => loadNativeBinding().eotToTtf(inputBuffer))
}

export async function eotToTtfAsync(input: Uint8Array): Promise<Buffer> {
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnosticsAsync(() =>
    loadNativeBinding().eotToTtfAsync(inputBuffer),
  )
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

  return withFontminDiagnostics(() =>
    loadNativeBinding().otfToTtf(inputBuffer, nativeOptions),
  )
}

export async function otfToTtfAsync(
  input: Uint8Array,
  options: Otf2TtfOptions = {},
): Promise<Buffer> {
  const nativeOptions: NativeOtf2TtfOptions = {}

  assignDefined(nativeOptions, 'preserveHinting', options.preserveHinting)
  assignDefined(
    nativeOptions,
    'variationCoordinates',
    options.variationCoordinates,
  )

  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnosticsAsync(() =>
    loadNativeBinding().otfToTtfAsync(inputBuffer, nativeOptions),
  )
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

  return withFontminDiagnostics(() =>
    loadNativeBinding().ttfToWoff2(inputBuffer, nativeOptions),
  )
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
    const nativeOptions: NativeWoff2Options = {}
    assignDefined(nativeOptions, 'quality', options.quality)
    const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

    return await withFontminDiagnosticsAsync(() =>
      loadNativeBinding().ttfToWoff2Async(inputBuffer, nativeOptions),
    )
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

  withFontminDiagnostics(() => loadNativeBinding().validateWoff2(inputBuffer))
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

  return withFontminDiagnostics(() =>
    loadNativeBinding().ttfToEot(inputBuffer, nativeOptions),
  )
}

export async function ttfToEotAsync(
  input: Uint8Array,
  options: Ttf2EotOptions = {},
): Promise<Buffer> {
  const nativeOptions: NativeEotOptions = {}
  assignDefined(nativeOptions, 'version', options.version)
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnosticsAsync(() =>
    loadNativeBinding().ttfToEotAsync(inputBuffer, nativeOptions),
  )
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

  return withFontminDiagnostics(() =>
    loadNativeBinding().ttfToSvg(inputBuffer, nativeOptions),
  )
}

export async function ttfToSvgAsync(
  input: Uint8Array,
  options: Ttf2SvgOptions = {},
): Promise<string> {
  const nativeOptions: NativeSvgOptions = {}
  assignDefined(nativeOptions, 'fontFamily', options.fontFamily)
  const inputBuffer = Buffer.isBuffer(input) ? input : Buffer.from(input)

  return withFontminDiagnosticsAsync(() =>
    loadNativeBinding().ttfToSvgAsync(inputBuffer, nativeOptions),
  )
}

export function svgFontToTtf(
  input: string,
  options: Svg2TtfOptions = {},
): Buffer {
  return withFontminDiagnostics(() =>
    loadNativeBinding().svgFontToTtf(input, toNativeSvg2TtfOptions(options)),
  )
}

export async function svgFontToTtfAsync(
  input: string,
  options: Svg2TtfOptions = {},
): Promise<Buffer> {
  return withFontminDiagnosticsAsync(() =>
    loadNativeBinding().svgFontToTtfAsync(
      input,
      toNativeSvg2TtfOptions(options),
    ),
  )
}

export function svgsToTtf(
  inputs: SvgIcon[],
  options: Svgs2TtfOptions = {},
): Buffer {
  return withFontminDiagnostics(() =>
    loadNativeBinding().svgsToTtf(
      inputs.map(input => toNativeSvgIcon(input)),
      toNativeSvgs2TtfOptions(options),
    ),
  )
}

export async function svgsToTtfAsync(
  inputs: SvgIcon[],
  options: Svgs2TtfOptions = {},
): Promise<Buffer> {
  return withFontminDiagnosticsAsync(() =>
    loadNativeBinding().svgsToTtfAsync(
      inputs.map(input => toNativeSvgIcon(input)),
      toNativeSvgs2TtfOptions(options),
    ),
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

  return withFontminDiagnostics(() =>
    loadNativeBinding().generateFontFaceCss(nativeSources, nativeOptions),
  )
}

export async function generateFontFaceCssAsync(
  sources: CssFontSource[],
  options: CssOptions = {},
): Promise<string> {
  const nativeSources = sources.map(source => toNativeCssFontSource(source))
  const nativeOptions: NativeCssOptions = {}

  if (options.fontFamily !== undefined) {
    nativeOptions.fontFamily = await resolveCssFontFamilyAsync(
      sources,
      options.fontFamily,
    )
  }
  assignDefined(nativeOptions, 'fontPath', options.fontPath)
  assignDefined(nativeOptions, 'base64', options.base64)
  assignDefined(nativeOptions, 'glyph', options.glyph)
  assignDefined(nativeOptions, 'iconPrefix', options.iconPrefix)
  assignDefined(nativeOptions, 'asFileName', options.asFileName)
  assignDefined(nativeOptions, 'local', options.local)
  assignDefined(nativeOptions, 'fontDisplay', options.fontDisplay)
  assignDefined(nativeOptions, 'target', options.target)
  assignDefined(nativeOptions, 'unicodeRanges', options.unicodeRanges)

  return withFontminDiagnosticsAsync(() =>
    loadNativeBinding().generateFontFaceCssAsync(nativeSources, nativeOptions),
  )
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

async function resolveCssFontFamilyAsync(
  sources: CssFontSource[],
  fontFamily: NonNullable<CssOptions['fontFamily']>,
): Promise<string> {
  if (typeof fontFamily === 'string') {
    return fontFamily
  }

  const source = sources.find(source => source.contents !== undefined)

  if (source?.contents === undefined) {
    throw new Error('CSS fontFamily resolver requires source contents')
  }

  return fontFamily(await inspectAsync(source.contents))
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
