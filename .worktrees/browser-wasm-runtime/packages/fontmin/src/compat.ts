import { defineConfig } from './config'
import { optimize } from './optimize'
import {
  css,
  glyph,
  otf2ttf,
  svg2ttf,
  svgs2ttf,
  ttf2eot,
  ttf2svg,
  ttf2woff,
  ttf2woff2,
} from './plugins'
import type { FontAsset, FontminConfig, FontminPlugin } from './types'

export default class FontminCompat {
  static glyph = glyph
  static ttf2eot = ttf2eot
  static ttf2svg = ttf2svg
  static ttf2woff = ttf2woff
  static ttf2woff2 = ttf2woff2
  static otf2ttf = otf2ttf
  static svg2ttf = svg2ttf
  static svgs2ttf = svgs2ttf
  static css = css

  private input: (string | Uint8Array)[] = []
  private outputDir?: string
  private plugins: FontminPlugin[] = []

  src(file: string | string[] | Uint8Array): this {
    this.input = Array.isArray(file) ? file : [file]
    return this
  }

  dest(dir: string): this {
    this.outputDir = dir
    return this
  }

  use(plugin: FontminPlugin): this {
    this.plugins.push(plugin)
    return this
  }

  config(): FontminConfig {
    const config: FontminConfig = {
      input: this.input,
      plugins: this.plugins,
    }

    if (this.outputDir !== undefined) {
      config.outDir = this.outputDir
    }

    return defineConfig(config)
  }

  runAsync(): Promise<FontAsset[]> {
    return optimize(this.config())
  }

  run(callback: (error: Error | null, files?: FontAsset[]) => void): void {
    const handleSuccess = (files: FontAsset[]): void => {
      callback(null, files)
    }
    const handleError = (error: Error): void => {
      callback(error)
    }

    void this.runAsync().then(handleSuccess).catch(handleError)
  }
}
