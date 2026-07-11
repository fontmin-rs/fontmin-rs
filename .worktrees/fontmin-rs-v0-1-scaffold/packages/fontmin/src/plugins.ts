import type {
  CssOptions,
  FontminPlugin,
  SubsetOptions,
  Ttf2Woff2Options,
  Ttf2WoffOptions,
} from './types'

export function definePlugin<T extends FontminPlugin>(plugin: T): T {
  return plugin
}

export function glyph(options: SubsetOptions = {}): FontminPlugin {
  const preserveHinting = options.preserveHinting ?? options.hinting ?? false

  return {
    name: 'fontmin:glyph',
    native: {
      kind: 'builtin',
      name: 'glyph',
      options: {
        text: options.text,
        unicodes: options.unicodes,
        basicText: options.basicText,
        hinting: options.hinting,
        trim: options.trim,
        keepNotdef: options.keepNotdef,
        keepLayout: options.keepLayout,
        clone: options.clone,
        preserveHinting,
      },
    },
  }
}

export function ttf2woff(options: Ttf2WoffOptions = {}): FontminPlugin {
  return {
    name: 'fontmin:ttf2woff',
    native: {
      kind: 'builtin',
      name: 'ttf2woff',
      options: { ...options },
    },
  }
}

export function ttf2woff2(options: Ttf2Woff2Options = {}): FontminPlugin {
  return {
    name: 'fontmin:ttf2woff2',
    native: {
      kind: 'builtin',
      name: 'ttf2woff2',
      options: { ...options },
    },
  }
}

export function css(options: CssOptions = {}): FontminPlugin {
  return {
    name: 'fontmin:css',
    native: {
      kind: 'builtin',
      name: 'css',
      options: { ...options },
    },
  }
}
