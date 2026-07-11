import type { BrowserAsset } from './optimize'

export type MaybePromise<T> = T | Promise<T>

export interface BrowserPluginContext {
  diagnostics: { level: 'warn'; message: string }[]
  emitFile(asset: BrowserAsset): void
  warn(message: string | Error): void
}

export interface BrowserPlugin {
  name: string
  options?: Record<string, unknown>
  transform?(
    asset: BrowserAsset,
    context: BrowserPluginContext,
  ): MaybePromise<BrowserAsset | BrowserAsset[] | null | undefined>
}

function plugin(
  name: string,
  options: Record<string, unknown> = {},
): BrowserPlugin {
  return { name, options }
}

export function glyph(options: Record<string, unknown> = {}): BrowserPlugin {
  return plugin('glyph', {
    basicText: false,
    keepNotdef: true,
    layout: 'conservative',
    preserveHinting: false,
    trim: true,
    unicodes: [],
    ...options,
  })
}

export function ttf2woff(options: Record<string, unknown> = {}): BrowserPlugin {
  return plugin('ttf2woff', options)
}

export function ttf2woff2(
  options: Record<string, unknown> = {},
): BrowserPlugin {
  return plugin('ttf2woff2', options)
}

export function ttf2eot(options: Record<string, unknown> = {}): BrowserPlugin {
  return plugin('ttf2eot', options)
}

export function ttf2svg(options: Record<string, unknown> = {}): BrowserPlugin {
  return plugin('ttf2svg', options)
}

export function otf2ttf(options: Record<string, unknown> = {}): BrowserPlugin {
  return plugin('otf2ttf', options)
}

export function svg2ttf(options: Record<string, unknown> = {}): BrowserPlugin {
  return plugin('svg2ttf', options)
}

export function svgs2ttf(options: Record<string, unknown> = {}): BrowserPlugin {
  return plugin('svgs2ttf', options)
}

export function css(options: Record<string, unknown> = {}): BrowserPlugin {
  return plugin('css', {
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

export function modernWeb(
  options: Record<string, unknown> = {},
): BrowserPlugin[] {
  const { fontFamily, fontPath, local, fontDisplay, ...subset } = options
  const cssOptions: Record<string, unknown> = {}

  if (fontFamily !== undefined) cssOptions['fontFamily'] = fontFamily
  if (fontPath !== undefined) cssOptions['fontPath'] = fontPath
  if (local !== undefined) cssOptions['local'] = local
  if (fontDisplay !== undefined) cssOptions['fontDisplay'] = fontDisplay

  return [glyph(subset), ttf2woff(options), ttf2woff2(options), css(cssOptions)]
}

export function fontminCompatPreset(
  options: Record<string, unknown> = {},
): BrowserPlugin[] {
  const { fontFamily, fontPath, local, fontDisplay, ...subset } = options
  const cssOptions: Record<string, unknown> = {}

  if (fontFamily !== undefined) cssOptions['fontFamily'] = fontFamily
  if (fontPath !== undefined) cssOptions['fontPath'] = fontPath
  if (local !== undefined) cssOptions['local'] = local
  if (fontDisplay !== undefined) cssOptions['fontDisplay'] = fontDisplay

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
