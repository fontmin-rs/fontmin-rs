import { PassThrough } from 'node:stream'
import { createBuiltinPlugin } from './builtin-plugin'
import { mime, plugins, util } from './compat-exports'
import { defineConfig } from './config'
import { optimize } from './optimize'
import {
  deliverySlices,
  glyph as modernGlyph,
  otf2ttf as modernOtf2Ttf,
  svg2ttf as modernSvg2Ttf,
  svgs2ttf,
  ttf2eot,
  ttf2svg,
  ttf2woff,
  ttf2woff2,
} from './plugins'
import type {
  FontAsset,
  FontminCompatCssOptions,
  FontminCompatGlyphOptions,
  FontminConfig,
  FontminPlugin,
  Otf2TtfOptions,
  SubsetOptions,
  Svg2TtfOptions,
} from './types'

type FontminSource = string | string[] | Uint8Array
type FontminSourceArguments = [] | [FontminSource]
type FontminDestinationArguments = [] | [string]

export default class FontminCompat {
  static mime = mime
  static plugins = plugins
  static util = util
  static glyph = compatGlyph
  static deliverySlices = deliverySlices
  static ttf2eot = ttf2eot
  static ttf2svg = ttf2svg
  static ttf2woff = ttf2woff
  static ttf2woff2 = ttf2woff2
  static otf2ttf = compatOtf2Ttf
  static svg2ttf = compatSvg2Ttf
  static svgs2ttf = svgs2ttf
  static css = compatCss

  private input: (string | Uint8Array)[] = []
  private outputDir?: string
  private plugins: FontminPlugin[] = []
  private sourceArguments: FontminSourceArguments = []
  private destinationArguments: FontminDestinationArguments = []

  src(): FontminSourceArguments
  src(file: FontminSource): this
  src(file?: FontminSource): FontminSourceArguments | this {
    if (file === undefined) {
      return this.sourceArguments
    }

    this.sourceArguments = [file]
    this.input = Array.isArray(file) ? file : [file]
    return this
  }

  dest(): FontminDestinationArguments
  dest(dir: string): this
  dest(dir?: string): FontminDestinationArguments | this {
    if (dir === undefined) {
      return this.destinationArguments
    }

    this.destinationArguments = [dir]
    this.outputDir = dir
    return this
  }

  use(plugin: FontminPlugin): this {
    this.plugins.push(plugin)
    return this
  }

  config(): FontminConfig {
    const plugins =
      this.plugins.length === 0
        ? [
            compatOtf2Ttf(),
            ttf2eot(),
            ttf2woff(),
            ttf2woff2(),
            ttf2svg(),
            compatCss(),
          ]
        : this.plugins
    const config: FontminConfig = {
      input: this.input,
      plugins,
    }

    if (this.outputDir !== undefined) {
      config.outDir = this.outputDir
    }

    return defineConfig(config)
  }

  runAsync(): Promise<FontAsset[]> {
    return optimize(this.config())
  }

  run(
    callback: (error: Error | null, files?: FontAsset[]) => void,
  ): PassThrough {
    const stream = new PassThrough({ objectMode: true })
    const handleSuccess = (files: FontAsset[]): void => {
      for (const file of files) {
        stream.write(file)
      }
      stream.end()
      callback(null, files)
    }
    const handleError = (error: Error): void => {
      stream.destroy(error)
    }

    stream.once('error', error => callback(error))
    void this.runAsync().then(handleSuccess).catch(handleError)

    return stream
  }
}

function compatCss(options: FontminCompatCssOptions = {}): FontminPlugin {
  const { asFileName, fontFamily, ...modernOptions } = options

  return createBuiltinPlugin('css', {
    ...modernOptions,
    ...(typeof fontFamily === 'function'
      ? { fontminCompatFontFamily: fontFamily }
      : { fontFamily }),
    ...(asFileName === true ? { fontminCompatAsFileName: true } : {}),
    local: modernOptions.local ?? false,
  })
}

function compatGlyph(options: FontminCompatGlyphOptions = {}): FontminPlugin {
  if (!hasSubsetSelector(options)) {
    return {
      name: 'fontmin:glyph-empty',
      transform(asset) {
        return asset
      },
    }
  }

  const { use, ...subsetOptions } = options
  const plugin = modernGlyph({
    ...subsetOptions,
    preserveHinting: options.preserveHinting ?? options.hinting ?? true,
  })

  if (use !== undefined && plugin.native !== undefined) {
    plugin.native.options['fontminCompatUse'] = use
  }

  return plugin
}

function compatOtf2Ttf(options: Otf2TtfOptions = {}): FontminPlugin {
  return modernOtf2Ttf({
    ...options,
    clone: options.clone ?? false,
    preserveHinting: options.preserveHinting ?? true,
  })
}

function compatSvg2Ttf(options: Svg2TtfOptions = {}): FontminPlugin {
  return modernSvg2Ttf({
    ...options,
    hinting: options.hinting ?? true,
  })
}

function hasSubsetSelector(options: SubsetOptions): boolean {
  return (
    options.basicText === true ||
    (options.text?.length ?? 0) > 0 ||
    options.textFile !== undefined ||
    (options.unicodes?.length ?? 0) > 0 ||
    (options.unicodeRanges?.length ?? 0) > 0
  )
}

export { mime, plugins, util } from './compat-exports'
