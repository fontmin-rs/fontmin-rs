import { createBuiltinPlugin } from './builtin-plugin'
import type {
  CssOptions,
  DeliverySlice,
  FontminPlugin,
  ModernWebOptions,
  Otf2TtfOptions,
  SubsetOptions,
  Svg2TtfOptions,
  Svgs2TtfOptions,
  Ttf2EotOptions,
  Ttf2SvgOptions,
  Ttf2Woff2Options,
  Ttf2WoffOptions,
} from './types'

export function definePlugin<T extends FontminPlugin>(plugin: T): T {
  return plugin
}

export function glyph(options: SubsetOptions = {}): FontminPlugin {
  const preserveHinting = options.preserveHinting ?? options.hinting ?? false

  return createBuiltinPlugin('glyph', {
    text: options.text,
    textFile: options.textFile,
    unicodes: options.unicodes,
    unicodeRanges: options.unicodeRanges,
    basicText: options.basicText,
    hinting: options.hinting,
    trim: options.trim,
    keepNotdef: options.keepNotdef,
    keepLayout: options.keepLayout,
    missingGlyphs: options.missingGlyphs,
    clone: options.clone,
    preserveHinting,
  })
}

export function deliverySlices(slices: DeliverySlice[]): FontminPlugin {
  return createBuiltinPlugin(
    'unicodeSlices',
    {
      slices: slices.map(slice => ({
        name: slice.name,
        unicodeRanges: [...slice.unicodeRanges],
      })),
    },
    'fontmin:unicode-slices',
  )
}

export function ttf2woff(options: Ttf2WoffOptions = {}): FontminPlugin {
  return createBuiltinPlugin('ttf2woff', { ...options })
}

export function ttf2woff2(options: Ttf2Woff2Options = {}): FontminPlugin {
  return createBuiltinPlugin('ttf2woff2', { ...options })
}

export function ttf2eot(options: Ttf2EotOptions = {}): FontminPlugin {
  return createBuiltinPlugin('ttf2eot', { ...options })
}

export function otf2ttf(options: Otf2TtfOptions = {}): FontminPlugin {
  return createBuiltinPlugin('otf2ttf', { ...options })
}

export function ttf2svg(options: Ttf2SvgOptions = {}): FontminPlugin {
  return createBuiltinPlugin('ttf2svg', { ...options })
}

export function svg2ttf(options: Svg2TtfOptions = {}): FontminPlugin {
  return createBuiltinPlugin('svg2ttf', { ...options })
}

export function svgs2ttf(options: Svgs2TtfOptions = {}): FontminPlugin {
  return createBuiltinPlugin('svgs2ttf', { ...options })
}

export function css(options: CssOptions = {}): FontminPlugin {
  return createBuiltinPlugin('css', { ...options })
}

export function modernWeb(options: ModernWebOptions = {}): FontminPlugin[] {
  const cssOptions: CssOptions = {}

  if (options.fontFamily !== undefined) {
    cssOptions.fontFamily = options.fontFamily
  }
  if (options.fontPath !== undefined) {
    cssOptions.fontPath = options.fontPath
  }
  if (options.local !== undefined) {
    cssOptions.local = options.local
  }
  if (options.fontDisplay !== undefined) {
    cssOptions.fontDisplay = options.fontDisplay
  }

  const otfOptions: Otf2TtfOptions = { clone: false }
  const woffOptions: Ttf2WoffOptions = {}
  const woff2Options: Ttf2Woff2Options = {}

  if (options.preserveHinting !== undefined) {
    otfOptions.preserveHinting = options.preserveHinting
  }
  if (options.variationCoordinates !== undefined) {
    otfOptions.variationCoordinates = options.variationCoordinates
  }
  if (options.clone !== undefined) {
    woffOptions.clone = options.clone
    woff2Options.clone = options.clone
  }
  if (options.deflate !== undefined) {
    woffOptions.deflate = options.deflate
  }
  if (options.compressionLevel !== undefined) {
    woffOptions.compressionLevel = options.compressionLevel
  }
  if (options.quality !== undefined) {
    woff2Options.quality = options.quality
  }
  if (options.fallback !== undefined) {
    woff2Options.fallback = options.fallback
  }

  return [
    otf2ttf(otfOptions),
    glyph(options),
    ttf2woff(woffOptions),
    ttf2woff2(woff2Options),
    css(cssOptions),
  ]
}
