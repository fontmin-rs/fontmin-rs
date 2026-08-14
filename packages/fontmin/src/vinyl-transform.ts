import { resolve } from 'node:path'
import { Transform } from 'node:stream'
import { callbackify } from 'node:util'
import Vinyl from 'vinyl'
import {
  createRuntimeSelector,
  resolvePipelineRuntimeMode,
} from './optimize-runtime'
import { createPluginContext } from './optimize-storage'
import {
  detectFormat,
  generateAssets,
  sortPlugins,
  transformAssets,
  woff2FallbacksFromPlugins,
} from './optimize-transforms'
import type { FontAsset, FontminPlugin } from './types'
import { resolvePluginTextFiles } from './workspace-io'

const VINYL_FILE_META_KEY = 'fontminVinylFile'

export function typedPluginTransform(plugins: FontminPlugin[]): Transform {
  const files: Vinyl[] = []
  const finishTransform = callbackify(transformVinylFiles)

  return new Transform({
    objectMode: true,
    transform(file: unknown, _encoding, callback) {
      try {
        files.push(assertVinyl(file))
        return callback()
      } catch (error) {
        return callback(errorFromUnknown(error))
      }
    },
    flush(callback) {
      finishTransform(files, plugins, (error, transformedFiles) => {
        if (error !== null) {
          return callback(errorFromUnknown(error))
        }

        for (const file of transformedFiles) {
          this.push(file)
        }
        return callback()
      })
    },
  })
}

async function transformVinylFiles(
  files: Vinyl[],
  unresolvedPlugins: FontminPlugin[],
): Promise<Vinyl[]> {
  const passthroughFiles: Vinyl[] = []
  const assets: FontAsset[] = []

  for (const file of files) {
    if (file.isNull()) {
      passthroughFiles.push(file)
      continue
    }
    if (file.isStream()) {
      throw new Error('Streaming Vinyl contents are not supported')
    }
    if (!file.isBuffer()) {
      throw new Error(`Unsupported Vinyl contents for ${file.relative}`)
    }

    const format = detectFormat(file.contents)
    assets.push({
      contents: Buffer.from(file.contents),
      format,
      meta: { [VINYL_FILE_META_KEY]: file },
      path: file.relative,
      sourceFormat: format,
    })
  }

  if (assets.length === 0) {
    return passthroughFiles
  }

  const cwd = files[0]?.cwd ?? process.cwd()
  const plugins = sortPlugins(
    await resolvePluginTextFiles(unresolvedPlugins, cwd),
  )
  const runtime = createRuntimeSelector(
    resolvePipelineRuntimeMode(undefined, woff2FallbacksFromPlugins(plugins)),
  )
  const emittedAssets: FontAsset[] = []
  const context = createPluginContext(cwd, emittedAssets)
  const startedPlugins: FontminPlugin[] = []
  let transformedAssets: FontAsset[] | undefined
  let primaryError: unknown

  try {
    for (const plugin of plugins) {
      startedPlugins.push(plugin)
      await plugin.buildStart?.(context)
    }

    let currentAssets = assets
    for (const plugin of plugins) {
      currentAssets = await transformAssets(
        currentAssets,
        plugin,
        context,
        runtime,
      )
      currentAssets = [...currentAssets, ...emittedAssets.splice(0)]
    }

    transformedAssets = await generateAssets(
      currentAssets,
      plugins,
      context,
      runtime,
      emittedAssets,
    )
  } catch (error) {
    primaryError = error
  }

  let cleanupError: unknown
  for (const plugin of startedPlugins) {
    try {
      await plugin.buildEnd?.(context)
    } catch (error) {
      cleanupError ??= error
    }
  }

  if (primaryError !== undefined) {
    throw errorFromUnknown(primaryError)
  }
  if (cleanupError !== undefined) {
    throw errorFromUnknown(cleanupError)
  }
  if (transformedAssets === undefined) {
    throw new Error('fontmin-rs Vinyl pipeline did not produce an asset result')
  }

  return [
    ...transformedAssets.map(asset => assetToVinyl(asset, cwd)),
    ...passthroughFiles,
  ]
}

function assetToVinyl(asset: FontAsset, cwd: string): Vinyl {
  const source = asset.meta[VINYL_FILE_META_KEY]
  const file = Vinyl.isVinyl(source)
    ? source.clone({ contents: false })
    : new Vinyl({ base: cwd, cwd })

  file.contents = Buffer.from(asset.contents)
  file.path = resolve(file.base, asset.path)
  return file
}

function assertVinyl(value: unknown): Vinyl {
  if (!Vinyl.isVinyl(value)) {
    throw new TypeError('Fontmin Vinyl streams must emit Vinyl files')
  }

  return value
}

function errorFromUnknown(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error))
}
