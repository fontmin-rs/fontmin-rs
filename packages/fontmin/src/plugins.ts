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

  return {
    name: 'fontmin:glyph',
    native: {
      kind: 'builtin',
      name: 'glyph',
      options: {
        text: options.text,
        textFile: options.textFile,
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

export function deliverySlices(slices: DeliverySlice[]): FontminPlugin {
  return {
    name: 'fontmin:unicode-slices',
    native: {
      kind: 'builtin',
      name: 'unicodeSlices',
      options: {
        slices: slices.map(slice => ({
          name: slice.name,
          unicodeRanges: [...slice.unicodeRanges],
        })),
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

export function ttf2eot(options: Ttf2EotOptions = {}): FontminPlugin {
  return {
    name: 'fontmin:ttf2eot',
    native: {
      kind: 'builtin',
      name: 'ttf2eot',
      options: { ...options },
    },
  }
}

export function otf2ttf(options: Otf2TtfOptions = {}): FontminPlugin {
  return {
    name: 'fontmin:otf2ttf',
    native: {
      kind: 'builtin',
      name: 'otf2ttf',
      options: { ...options },
    },
  }
}

export function ttf2svg(options: Ttf2SvgOptions = {}): FontminPlugin {
  return {
    name: 'fontmin:ttf2svg',
    native: {
      kind: 'builtin',
      name: 'ttf2svg',
      options: { ...options },
    },
  }
}

export function svg2ttf(options: Svg2TtfOptions = {}): FontminPlugin {
  return {
    name: 'fontmin:svg2ttf',
    native: {
      kind: 'builtin',
      name: 'svg2ttf',
      options: { ...options },
    },
  }
}

export function svgs2ttf(options: Svgs2TtfOptions = {}): FontminPlugin {
  return {
    name: 'fontmin:svgs2ttf',
    native: {
      kind: 'builtin',
      name: 'svgs2ttf',
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

  if (options.preserveHinting !== undefined) {
    otfOptions.preserveHinting = options.preserveHinting
  }
  if (options.variationCoordinates !== undefined) {
    otfOptions.variationCoordinates = options.variationCoordinates
  }

  return [
    otf2ttf(otfOptions),
    glyph(options),
    ttf2woff(options),
    ttf2woff2(options),
    css(cssOptions),
  ]
}
