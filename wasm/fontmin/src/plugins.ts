import type {
  CssOptions,
  Otf2TtfOptions,
  SubsetOptions,
  Svg2TtfOptions,
  Svgs2TtfOptions,
  Ttf2EotOptions,
  Ttf2SvgOptions,
  Ttf2Woff2Options,
  WoffOptions,
} from '../types'
import type { BrowserAsset } from './optimize'

export type MaybePromise<T> = T | Promise<T>

export interface DeliverySlice {
  name: string
  unicodeRanges: string[]
}

export interface DeliverySlicesOptions {
  slices: DeliverySlice[]
}

export interface GlyphOptions extends SubsetOptions {
  clone?: boolean
}

export interface Ttf2WoffPluginOptions extends WoffOptions {
  clone?: boolean
}

export interface Ttf2Woff2PluginOptions extends Ttf2Woff2Options {
  clone?: boolean
}

export interface Ttf2EotPluginOptions extends Ttf2EotOptions {
  clone?: boolean
}

export interface Ttf2SvgPluginOptions extends Ttf2SvgOptions {
  clone?: boolean
}

export interface Otf2TtfPluginOptions extends Otf2TtfOptions {
  clone?: boolean
}

export interface Svg2TtfPluginOptions extends Svg2TtfOptions {
  clone?: boolean
}

export interface Svgs2TtfPluginOptions extends Svgs2TtfOptions {
  clone?: boolean
}

export interface ModernWebOptions
  extends
    GlyphOptions,
    Otf2TtfPluginOptions,
    Ttf2WoffPluginOptions,
    Ttf2Woff2PluginOptions {
  fontDisplay?: CssOptions['fontDisplay']
  fontFamily?: string
  fontPath?: string
  local?: boolean
}

export interface FontminCompatPresetOptions
  extends
    GlyphOptions,
    Otf2TtfPluginOptions,
    Ttf2EotPluginOptions,
    Ttf2SvgPluginOptions,
    Ttf2WoffPluginOptions,
    Ttf2Woff2PluginOptions {
  fontDisplay?: CssOptions['fontDisplay']
  fontPath?: string
  local?: boolean
}

export interface BrowserPluginContext {
  diagnostics: { level: 'warn'; message: string }[]
  emitFile(asset: BrowserAsset): void
  warn(message: string | Error): void
}

export interface BrowserPlugin<Options extends object = object> {
  name: string
  options?: Options
  transform?(
    asset: BrowserAsset,
    context: BrowserPluginContext,
  ): MaybePromise<BrowserAsset | BrowserAsset[] | null | undefined>
}

function plugin<Options extends object>(
  name: string,
  options: Options,
): BrowserPlugin<Options> {
  return { name, options }
}

export function glyph(options: GlyphOptions = {}): BrowserPlugin<GlyphOptions> {
  return plugin<GlyphOptions>('glyph', {
    basicText: false,
    keepNotdef: true,
    layout: 'conservative',
    preserveHinting: false,
    trim: true,
    unicodes: [],
    ...options,
  })
}

export function deliverySlices(
  slices: DeliverySlice[],
): BrowserPlugin<DeliverySlicesOptions> {
  return plugin<DeliverySlicesOptions>('unicodeSlices', {
    slices: slices.map(slice => ({
      name: slice.name,
      unicodeRanges: [...slice.unicodeRanges],
    })),
  })
}

export function normalizeDeliverySlices(
  options: DeliverySlicesOptions,
): DeliverySlice[] {
  const values: unknown = options.slices

  if (!Array.isArray(values) || values.length === 0) {
    throw new Error('unicode delivery slices must not be empty')
  }

  const names = new Set<string>()

  return values.map((value, index) => {
    if (typeof value !== 'object' || value === null || Array.isArray(value)) {
      throw new Error(`unicode delivery slice ${index + 1} must be an object`)
    }

    const { name, unicodeRanges } = value as {
      name?: unknown
      unicodeRanges?: unknown
    }

    if (
      typeof name !== 'string' ||
      name.length === 0 ||
      !/^[A-Za-z0-9_-]+$/u.test(name)
    ) {
      throw new Error(
        `unicode delivery slice ${index + 1} must have a name containing only letters, digits, hyphens, or underscores`,
      )
    }
    if (names.has(name)) {
      throw new Error(`unicode delivery slice name is duplicated: ${name}`)
    }
    if (
      !Array.isArray(unicodeRanges) ||
      unicodeRanges.length === 0 ||
      unicodeRanges.some(
        range => typeof range !== 'string' || range.length === 0,
      )
    ) {
      throw new Error(
        `unicode delivery slice ${name} must include at least one Unicode range`,
      )
    }

    names.add(name)

    return { name, unicodeRanges: [...unicodeRanges] }
  })
}

export function ttf2woff(
  options: Ttf2WoffPluginOptions = {},
): BrowserPlugin<Ttf2WoffPluginOptions> {
  return plugin('ttf2woff', options)
}

export function ttf2woff2(
  options: Ttf2Woff2PluginOptions = {},
): BrowserPlugin<Ttf2Woff2PluginOptions> {
  return plugin('ttf2woff2', options)
}

export function ttf2eot(
  options: Ttf2EotPluginOptions = {},
): BrowserPlugin<Ttf2EotPluginOptions> {
  return plugin('ttf2eot', options)
}

export function ttf2svg(
  options: Ttf2SvgPluginOptions = {},
): BrowserPlugin<Ttf2SvgPluginOptions> {
  return plugin('ttf2svg', options)
}

export function otf2ttf(
  options: Otf2TtfPluginOptions = {},
): BrowserPlugin<Otf2TtfPluginOptions> {
  return plugin('otf2ttf', options)
}

export function svg2ttf(
  options: Svg2TtfPluginOptions = {},
): BrowserPlugin<Svg2TtfPluginOptions> {
  return plugin('svg2ttf', options)
}

export function svgs2ttf(
  options: Svgs2TtfPluginOptions = {},
): BrowserPlugin<Svgs2TtfPluginOptions> {
  return plugin('svgs2ttf', options)
}

export function css(options: CssOptions = {}): BrowserPlugin<CssOptions> {
  return plugin<CssOptions>('css', {
    asFileName: false,
    base64: false,
    fontDisplay: 'swap',
    fontFamily: 'fontmin',
    fontPath: './',
    glyph: false,
    iconPrefix: 'icon',
    local: true,
    target: 'css',
    ...options,
  })
}

export function modernWeb(options: ModernWebOptions = {}): BrowserPlugin[] {
  const { fontFamily, fontPath, local, fontDisplay, ...subset } = options
  const cssOptions: CssOptions = {}

  if (fontFamily !== undefined) {
    cssOptions.fontFamily = fontFamily
  }
  if (fontPath !== undefined) {
    cssOptions.fontPath = fontPath
  }
  if (local !== undefined) {
    cssOptions.local = local
  }
  if (fontDisplay !== undefined) {
    cssOptions.fontDisplay = fontDisplay
  }

  const otfOptions: Otf2TtfPluginOptions = { clone: false }

  if (typeof options.preserveHinting === 'boolean') {
    otfOptions.preserveHinting = options.preserveHinting
  }
  if (options.variationCoordinates !== undefined) {
    otfOptions.variationCoordinates = options.variationCoordinates
  }

  return [
    otf2ttf(otfOptions),
    glyph(subset),
    ttf2woff(options),
    ttf2woff2(options),
    css(cssOptions),
  ]
}

export function fontminCompatPreset(
  options: FontminCompatPresetOptions = {},
): BrowserPlugin[] {
  const { fontFamily, fontPath, local, fontDisplay, ...subset } = options
  const cssOptions: CssOptions = {}

  if (fontFamily !== undefined) {
    cssOptions.fontFamily = fontFamily
  }
  if (fontPath !== undefined) {
    cssOptions.fontPath = fontPath
  }
  if (local !== undefined) {
    cssOptions.local = local
  }
  if (fontDisplay !== undefined) {
    cssOptions.fontDisplay = fontDisplay
  }

  return [
    otf2ttf(options),
    glyph(subset),
    ttf2eot(options),
    ttf2svg(options),
    ttf2woff(options),
    ttf2woff2(options),
    css(cssOptions),
  ]
}
