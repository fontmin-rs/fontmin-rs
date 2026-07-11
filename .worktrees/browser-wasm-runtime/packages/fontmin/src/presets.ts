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
  Ttf2WoffOptions,
} from './types'

export { modernWeb } from './plugins'

export function fontminCompatPreset(
  options: FontminCompatPresetOptions = {},
): FontminPlugin[] {
  const cssOptions = cssOptionsFromPreset(options)
  const woffOptions = woffOptionsFromPreset(options)

  return [
    otf2ttf(options),
    glyph(options),
    ttf2eot(options),
    ttf2svg(options),
    ttf2woff(woffOptions),
    ttf2woff2(options),
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
