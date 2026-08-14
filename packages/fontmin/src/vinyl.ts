import { resolve } from 'node:path'
import { PassThrough, Readable } from 'node:stream'
import Vinyl from 'vinyl'
import vinylFs from 'vinyl-fs'
import type * as VinylFs from 'vinyl-fs'
import FontminCompat from './compat'
import { detectFormat } from './optimize-transforms'
import type { FontminPlugin } from './types'
import { typedPluginTransform } from './vinyl-transform'

type FontminSource = string | string[] | Uint8Array
type VinylPlugin =
  | FontminPlugin
  | NodeJS.ReadWriteStream
  | (() => FontminPlugin | NodeJS.ReadWriteStream)
type SourceArguments =
  | []
  | [FontminSource]
  | [FontminSource, VinylFs.SrcOptions]
type DestinationArguments = [] | [string] | [string, VinylFs.DestOptions]

/**
 * Opt-in compatibility chain for legacy Gulp and Vinyl transforms.
 *
 * The main `fontmin-rs` entry intentionally uses typed `FontAsset` objects.
 * Import this class from `fontmin-rs/vinyl` when plugins depend on Vinyl file
 * methods or `vinyl-fs` source and destination semantics.
 */
export default class FontminVinyl {
  static mime = FontminCompat.mime
  static plugins = FontminCompat.plugins
  static util = FontminCompat.util
  static glyph = FontminCompat.glyph
  static deliverySlices = FontminCompat.deliverySlices
  static ttf2eot = FontminCompat.ttf2eot
  static ttf2svg = FontminCompat.ttf2svg
  static ttf2woff = FontminCompat.ttf2woff
  static ttf2woff2 = FontminCompat.ttf2woff2
  static otf2ttf = FontminCompat.otf2ttf
  static svg2ttf = FontminCompat.svg2ttf
  static svgs2ttf = FontminCompat.svgs2ttf
  static css = FontminCompat.css

  private sourceArguments: SourceArguments = []
  private destinationArguments: DestinationArguments = []
  private stages: VinylPlugin[] = []

  src(): SourceArguments
  src(file: FontminSource, options?: VinylFs.SrcOptions): this
  src(
    file?: FontminSource,
    options?: VinylFs.SrcOptions,
  ): SourceArguments | this {
    if (file === undefined) {
      return this.sourceArguments
    }

    this.sourceArguments = options === undefined ? [file] : [file, options]
    return this
  }

  dest(): DestinationArguments
  dest(dir: string, options?: VinylFs.DestOptions): this
  dest(
    dir?: string,
    options?: VinylFs.DestOptions,
  ): DestinationArguments | this {
    if (dir === undefined) {
      return this.destinationArguments
    }

    this.destinationArguments = options === undefined ? [dir] : [dir, options]
    return this
  }

  use(plugin: VinylPlugin): this {
    this.stages.push(plugin)
    return this
  }

  run(callback: (error: Error | null, files?: Vinyl[]) => void): PassThrough {
    const stream = this.createStream()
    const files: Vinyl[] = []
    let settled = false
    const settle = (error: Error | null): void => {
      if (settled) {
        return
      }

      settled = true
      callback(error, error === null ? files : undefined)
    }

    stream.on('data', file => files.push(assertVinyl(file)))
    stream.once('end', () => settle(null))
    stream.once('error', error => settle(errorFromUnknown(error)))

    return stream
  }

  runAsync(): Promise<Vinyl[]> {
    return new Promise((resolveFiles, reject) => {
      this.run((error, files) => {
        if (error !== null) {
          reject(error)
          return
        }

        resolveFiles(files ?? [])
      })
    })
  }

  private createStream(): PassThrough {
    const output = new PassThrough({ objectMode: true })
    let current: NodeJS.ReadableStream = this.createSource()

    forwardErrors(current, output)

    for (const stage of groupStages(this.effectiveStages())) {
      const next = Array.isArray(stage)
        ? typedPluginTransform(stage)
        : resolveVinylStream(stage)

      forwardErrors(next, output)
      current = current.pipe(next)
    }

    const destination = this.destinationArguments
    const [destinationDir, destinationOptions] = destination
    if (destinationDir !== undefined) {
      const next = vinylFs.dest(destinationDir, destinationOptions)

      forwardErrors(next, output)
      current = current.pipe(next)
    }

    current.pipe(output)
    return output
  }

  private createSource(): NodeJS.ReadableStream {
    const source = this.sourceArguments

    if (source.length === 0) {
      return Readable.from([], { objectMode: true })
    }

    const [input, options] = source
    if (typeof input === 'string' || Array.isArray(input)) {
      return vinylFs.src(input, { encoding: false, ...options })
    }

    const cwd =
      typeof options?.cwd === 'string' ? resolve(options.cwd) : process.cwd()
    const base =
      typeof options?.base === 'string' ? resolve(cwd, options.base) : cwd
    const format = detectFormat(input)
    const extension = format === 'unknown' ? 'bin' : format
    const file = new Vinyl({
      base,
      contents: Buffer.from(input),
      cwd,
      path: resolve(base, `fontmin.${extension}`),
    })

    return Readable.from([file], { objectMode: true })
  }

  private effectiveStages(): VinylPlugin[] {
    return this.stages.length > 0
      ? this.stages
      : [
          FontminCompat.otf2ttf(),
          FontminCompat.ttf2eot(),
          FontminCompat.ttf2woff(),
          FontminCompat.ttf2woff2(),
          FontminCompat.ttf2svg(),
          FontminCompat.css(),
        ]
  }
}

function groupStages(
  stages: VinylPlugin[],
): (FontminPlugin[] | NodeJS.ReadWriteStream | (() => unknown))[] {
  const groups: (FontminPlugin[] | NodeJS.ReadWriteStream | (() => unknown))[] =
    []
  let typedPlugins: FontminPlugin[] = []

  for (const stage of stages) {
    const resolved = typeof stage === 'function' ? stage() : stage

    if (isFontminPlugin(resolved)) {
      typedPlugins.push(resolved)
      continue
    }

    if (typedPlugins.length > 0) {
      groups.push(typedPlugins)
      typedPlugins = []
    }
    groups.push(resolved)
  }

  if (typedPlugins.length > 0) {
    groups.push(typedPlugins)
  }

  return groups
}

function isFontminPlugin(value: unknown): value is FontminPlugin {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as { name?: unknown }).name === 'string'
  )
}

function resolveVinylStream(value: unknown): NodeJS.ReadWriteStream {
  if (
    typeof value !== 'object' ||
    value === null ||
    typeof (value as NodeJS.ReadWriteStream).pipe !== 'function' ||
    typeof (value as NodeJS.ReadWriteStream).write !== 'function'
  ) {
    throw new TypeError('Fontmin Vinyl plugins must be transform streams')
  }

  return value as NodeJS.ReadWriteStream
}

function assertVinyl(value: unknown): Vinyl {
  if (!Vinyl.isVinyl(value)) {
    throw new TypeError('Fontmin Vinyl streams must emit Vinyl files')
  }

  return value
}

function forwardErrors(
  stream: NodeJS.ReadableStream,
  output: PassThrough,
): void {
  stream.once('error', error => output.destroy(errorFromUnknown(error)))
}

function errorFromUnknown(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error))
}

export type { VinylPlugin }
export type { DestOptions, SrcOptions } from 'vinyl-fs'
