import * as browserWasm from '@fontmin-rs/wasm'
import type { CssFontSource, CssOptions } from '@fontmin-rs/wasm'
import type {
  BrowserDeliverySlice,
  InputFormat,
  PlaygroundAsset,
  PlaygroundDeliveryPreset,
  PlaygroundFormat,
  ProcessFontRequest,
} from './types'

export interface BrowserWasmApi {
  eotToTtf(input: Uint8Array): Promise<Uint8Array>
  generateFontFaceCss(
    sources: CssFontSource[],
    options: CssOptions,
  ): Promise<string>
  initWasm(): Promise<void>
  otfToTtf(
    input: Uint8Array,
    options?: Record<string, unknown>,
  ): Promise<Uint8Array>
  subsetTtf(
    input: Uint8Array,
    options?: Record<string, unknown>,
  ): Promise<Uint8Array>
  svgFontToTtf(
    input: string,
    options?: Record<string, unknown>,
  ): Promise<Uint8Array>
  ttfToEot(
    input: Uint8Array,
    options?: Record<string, unknown>,
  ): Promise<Uint8Array>
  ttfToSvg(
    input: Uint8Array,
    options?: Record<string, unknown>,
  ): Promise<string>
  ttfToWoff(
    input: Uint8Array,
    options?: Record<string, unknown>,
  ): Promise<Uint8Array>
  ttfToWoff2(
    input: Uint8Array,
    options?: Record<string, unknown>,
  ): Promise<Uint8Array>
  woff2ToTtf(input: Uint8Array): Promise<Uint8Array>
  woffToTtf(input: Uint8Array): Promise<Uint8Array>
}

const fontFormats: PlaygroundFormat[] = ['ttf', 'woff', 'woff2', 'eot', 'svg']
const unicodeRangePattern =
  /^[Uu]\+([0-9A-Fa-f]{1,6})(?:-([0-9A-Fa-f]{1,6}))?$/u
const deliveryPresetRanges: Record<
  Exclude<PlaygroundDeliveryPreset, 'custom'>,
  string[]
> = {
  cjk: ['U+4E00-9FFF'],
  latin: ['U+0000-00FF'],
}

export function detectInputFormat(fileName: string): InputFormat {
  const extension = fileName.split('.').pop()?.toLowerCase() ?? ''

  if (['ttf', 'woff', 'woff2', 'eot', 'otf', 'svg'].includes(extension)) {
    return extension as InputFormat
  }

  throw new Error(`Unsupported input format: .${extension || 'unknown'}.`)
}

export function isSupportedInputFile(fileName: string): boolean {
  try {
    detectInputFormat(fileName)
    return true
  } catch {
    return false
  }
}

export function parseUnicodeRanges(value: string): string[] {
  if (value.trim().length === 0) {
    return []
  }

  return value.split(',').map(range => {
    const descriptor = range.trim()
    const match = unicodeRangePattern.exec(descriptor)
    const start = match?.[1] ? Number.parseInt(match[1], 16) : Number.NaN
    const end = match?.[2] ? Number.parseInt(match[2], 16) : start

    if (
      !match ||
      !isUnicodeScalar(start) ||
      !isUnicodeScalar(end) ||
      start > end
    ) {
      throw new Error(`Invalid Unicode range: ${descriptor}.`)
    }

    return descriptor
  })
}

export function createDeliverySlices(
  presets: ReadonlySet<PlaygroundDeliveryPreset>,
  customRanges: string,
): BrowserDeliverySlice[] {
  const slices: BrowserDeliverySlice[] = []

  for (const preset of ['latin', 'cjk', 'custom'] as const) {
    if (!presets.has(preset)) {
      continue
    }

    slices.push({
      name: preset,
      unicodeRanges:
        preset === 'custom'
          ? parseUnicodeRanges(customRanges)
          : deliveryPresetRanges[preset],
    })
  }

  return slices
}

export function validateRequest(request: ProcessFontRequest): void {
  if (request.text.trim().length === 0) {
    throw new Error('Enter at least one character to subset.')
  }

  if (request.formats.size === 0) {
    throw new Error('Select at least one output format.')
  }

  if (request.formats.size === 1 && request.formats.has('css')) {
    throw new Error('Select at least one font output when generating CSS.')
  }

  validateDeliverySlices(request.deliverySlices)
}

export async function processFont(
  request: ProcessFontRequest,
  wasm: BrowserWasmApi = browserWasm,
): Promise<PlaygroundAsset[]> {
  validateRequest(request)
  request.onPhase?.('initializing')
  await wasm.initWasm()

  const inputFormat = detectInputFormat(request.fileName)
  request.onPhase?.('normalizing')
  const ttf = await normalizeToTtf(request.contents, inputFormat, wasm)
  request.onPhase?.('subsetting')
  const stem = fileStem(request.fileName)
  request.onPhase?.('converting')
  const deliverySlices = request.deliverySlices ?? []
  const assets = await generateAssets(
    stem,
    ttf,
    request.text,
    request.formats,
    deliverySlices,
    wasm,
  )

  if (request.formats.has('css')) {
    assets.push({
      contents: new TextEncoder().encode(
        await wasm.generateFontFaceCss(
          cssSources(assets),
          cssOptions(
            stem,
            deliverySlices.length === 0 ? request.unicodeRanges : undefined,
          ),
        ),
      ),
      fileName: `${stem}.css`,
      format: 'css',
    })
  }

  return assets
}

async function generateAssets(
  stem: string,
  ttf: Uint8Array,
  text: string,
  formats: ReadonlySet<PlaygroundFormat>,
  deliverySlices: BrowserDeliverySlice[],
  wasm: BrowserWasmApi,
): Promise<PlaygroundAsset[]> {
  if (deliverySlices.length === 0) {
    const subset = await wasm.subsetTtf(ttf, subsetOptions(text))

    return generateFontAssets(stem, subset, formats, wasm)
  }

  const assetGroups = await Promise.all(
    deliverySlices.map(async slice => {
      const subset = await wasm.subsetTtf(
        ttf,
        subsetOptions(text, slice.unicodeRanges),
      )
      const assets = await generateFontAssets(
        `${stem}-${slice.name}`,
        subset,
        formats,
        wasm,
      )

      return assets.map(asset => ({
        ...asset,
        unicodeRanges: slice.unicodeRanges,
      }))
    }),
  )

  return assetGroups.flat()
}

async function normalizeToTtf(
  input: Uint8Array,
  format: InputFormat,
  wasm: BrowserWasmApi,
): Promise<Uint8Array> {
  switch (format) {
    case 'ttf': {
      return input
    }
    case 'woff': {
      return wasm.woffToTtf(input)
    }
    case 'woff2': {
      return wasm.woff2ToTtf(input)
    }
    case 'eot': {
      return wasm.eotToTtf(input)
    }
    case 'otf': {
      return wasm.otfToTtf(input)
    }
    case 'svg': {
      return wasm.svgFontToTtf(new TextDecoder().decode(input))
    }
  }
}

async function generateFontAssets(
  stem: string,
  input: Uint8Array,
  formats: ReadonlySet<PlaygroundFormat>,
  wasm: BrowserWasmApi,
): Promise<PlaygroundAsset[]> {
  const assets: PlaygroundAsset[] = []

  for (const format of fontFormats) {
    if (!formats.has(format)) {
      continue
    }

    const contents = await convertTtf(input, format, wasm)
    assets.push({ contents, fileName: `${stem}.${format}`, format })
  }

  return assets
}

async function convertTtf(
  input: Uint8Array,
  format: Exclude<PlaygroundFormat, 'css'>,
  wasm: BrowserWasmApi,
): Promise<Uint8Array> {
  switch (format) {
    case 'ttf': {
      return input
    }
    case 'woff': {
      return wasm.ttfToWoff(input, {})
    }
    case 'woff2': {
      return wasm.ttfToWoff2(input, {})
    }
    case 'eot': {
      return wasm.ttfToEot(input, {})
    }
    case 'svg': {
      return new TextEncoder().encode(await wasm.ttfToSvg(input, {}))
    }
  }
}

function cssSources(assets: PlaygroundAsset[]): CssFontSource[] {
  return assets.map(asset => {
    const source: CssFontSource = {
      contents: asset.contents,
      fileName: asset.fileName,
      format: asset.format as Exclude<PlaygroundFormat, 'css'>,
    }

    if (asset.unicodeRanges !== undefined) {
      source.unicodeRanges = asset.unicodeRanges
    }

    return source
  })
}

function cssOptions(fontFamily: string, unicodeRanges?: string[]): CssOptions {
  const options: CssOptions = {
    asFileName: false,
    base64: false,
    fontDisplay: 'swap',
    fontFamily,
    fontPath: './',
    glyph: false,
    iconPrefix: 'icon',
    local: false,
    target: 'css',
  }

  if (unicodeRanges?.length) {
    options.unicodeRanges = unicodeRanges
  }

  return options
}

function subsetOptions(
  text: string,
  unicodeRanges?: string[],
): Record<string, unknown> {
  const options: Record<string, unknown> = {
    basicText: false,
    keepNotdef: true,
    layout: 'conservative',
    preserveHinting: false,
    text,
    trim: true,
    unicodes: [],
  }

  if (unicodeRanges?.length) {
    options['unicodeRanges'] = unicodeRanges
  }

  return options
}

function validateDeliverySlices(
  slices: BrowserDeliverySlice[] | undefined,
): void {
  const names = new Set<string>()

  for (const slice of slices ?? []) {
    if (!/^[A-Za-z0-9_-]+$/u.test(slice.name)) {
      throw new Error(`Invalid delivery slice name: ${slice.name}.`)
    }
    if (names.has(slice.name)) {
      throw new Error(`Duplicate delivery slice name: ${slice.name}.`)
    }
    if (slice.unicodeRanges.length === 0) {
      throw new Error(`Delivery slice ${slice.name} requires a Unicode range.`)
    }

    parseUnicodeRanges(slice.unicodeRanges.join(','))
    names.add(slice.name)
  }
}

function fileStem(fileName: string): string {
  return fileName.replace(/\.[^.]+$/u, '') || 'fontmin'
}

function isUnicodeScalar(value: number): boolean {
  return value <= 0x10_ff_ff && (value < 0xd8_00 || value > 0xdf_ff)
}
