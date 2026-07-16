import {
  css,
  glyph,
  otf2ttf,
  ttf2eot,
  ttf2svg,
  ttf2woff,
  ttf2woff2,
} from './plugins'
import type {
  CssOptions,
  FontminCompatPresetOptions,
  FontminPlugin,
  Otf2TtfOptions,
  Ttf2EotOptions,
  Ttf2SvgOptions,
  Ttf2Woff2Options,
  Ttf2WoffOptions,
} from './types'

export { modernWeb } from './plugins'

export function fontminCompatPreset(
  options: FontminCompatPresetOptions = {},
): FontminPlugin[] {
  const cssOptions = cssOptionsFromPreset(options)
  const eotOptions = eotOptionsFromPreset(options)
  const otfOptions = otfOptionsFromPreset(options)
  const svgOptions = svgOptionsFromPreset(options)
  const woffOptions = woffOptionsFromPreset(options)
  const woff2Options = woff2OptionsFromPreset(options)

  return [
    otf2ttf(otfOptions),
    glyph(options),
    ttf2eot(eotOptions),
    ttf2svg(svgOptions),
    ttf2woff(woffOptions),
    ttf2woff2(woff2Options),
    css(cssOptions),
  ]
}

function cssOptionsFromPreset(options: FontminCompatPresetOptions): CssOptions {
  const cssOptions: CssOptions = {}

  if (options.asFileName !== undefined) {
    cssOptions.asFileName = options.asFileName
  }
  if (options.base64 !== undefined) {
    cssOptions.base64 = options.base64
  }
  if (options.cssGlyph !== undefined) {
    cssOptions.glyph = options.cssGlyph
  } else if (options.glyphCss !== undefined) {
    cssOptions.glyph = options.glyphCss
  } else if (typeof options.glyph === 'boolean') {
    cssOptions.glyph = options.glyph
  }
  if (options.fontDisplay !== undefined) {
    cssOptions.fontDisplay = options.fontDisplay
  }
  if (options.fontFamily !== undefined) {
    cssOptions.fontFamily = options.fontFamily
  }
  if (options.fontPath !== undefined) {
    cssOptions.fontPath = options.fontPath
  }
  if (options.iconPrefix !== undefined) {
    cssOptions.iconPrefix = options.iconPrefix
  }
  if (options.local !== undefined) {
    cssOptions.local = options.local
  }
  if (options.target !== undefined) {
    cssOptions.target = options.target
  }

  return cssOptions
}

function eotOptionsFromPreset(
  options: FontminCompatPresetOptions,
): Ttf2EotOptions {
  const eotOptions: Ttf2EotOptions = {}

  if (options.clone !== undefined) {
    eotOptions.clone = options.clone
  }
  if (options.version !== undefined) {
    eotOptions.version = options.version
  }

  return eotOptions
}

function otfOptionsFromPreset(
  options: FontminCompatPresetOptions,
): Otf2TtfOptions {
  const otfOptions: Otf2TtfOptions = {}

  if (options.clone !== undefined) {
    otfOptions.clone = options.clone
  }
  if (options.preserveHinting !== undefined) {
    otfOptions.preserveHinting = options.preserveHinting
  }
  if (options.variationCoordinates !== undefined) {
    otfOptions.variationCoordinates = options.variationCoordinates
  }

  return otfOptions
}

function svgOptionsFromPreset(
  options: FontminCompatPresetOptions,
): Ttf2SvgOptions {
  const svgOptions: Ttf2SvgOptions = {}

  if (options.clone !== undefined) {
    svgOptions.clone = options.clone
  }
  if (options.fontFamily !== undefined) {
    svgOptions.fontFamily = options.fontFamily
  }

  return svgOptions
}

function woffOptionsFromPreset(
  options: FontminCompatPresetOptions,
): Ttf2WoffOptions {
  const woffOptions: Ttf2WoffOptions = {}

  if (options.clone !== undefined) {
    woffOptions.clone = options.clone
  }
  if (options.compressionLevel !== undefined) {
    woffOptions.compressionLevel = options.compressionLevel
  }
  if (options.deflate !== undefined) {
    woffOptions.deflate = options.deflate
  } else if (options.deflateWoff !== undefined) {
    woffOptions.deflate = options.deflateWoff
  }

  return woffOptions
}

function woff2OptionsFromPreset(
  options: FontminCompatPresetOptions,
): Ttf2Woff2Options {
  const woff2Options: Ttf2Woff2Options = {}

  if (options.clone !== undefined) {
    woff2Options.clone = options.clone
  }
  if (options.fallback !== undefined) {
    woff2Options.fallback = options.fallback
  }
  if (options.quality !== undefined) {
    woff2Options.quality = options.quality
  }

  return woff2Options
}
