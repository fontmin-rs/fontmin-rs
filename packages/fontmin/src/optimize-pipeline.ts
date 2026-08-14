import { resolve } from 'node:path'
import {
  createRuntimeSelector,
  resolvePipelineRuntimeMode,
} from './optimize-runtime'
import {
  cacheKeyForAssets,
  cacheRuntimeIdentity,
  createPluginContext,
  isCacheablePipeline,
  normalizeCacheOptions,
  readCachedAssets,
  writeCachedAssets,
} from './optimize-storage'
import {
  generateAssets,
  pluginsFromConfig,
  runGlyph,
  sortPlugins,
  transformAssets,
  woff2FallbacksFromPlugins,
} from './optimize-transforms'
import { flatMapAssets } from './runtime-neutral/optimize-policy'
import type { FontAsset, FontminConfig, FontminPlugin } from './types'
import { preserveWebDeliverySources } from './web-delivery'
import {
  cleanOutputDirectory,
  loadInputAssets,
  resolveConfigTextFile,
  resolvePluginTextFiles,
  writeAssets,
} from './workspace-io'

export async function optimize(
  unresolvedConfig: FontminConfig,
): Promise<FontAsset[]> {
  const cwd =
    unresolvedConfig.cwd === undefined ? process.cwd() : unresolvedConfig.cwd
  const config = await resolveConfigTextFile(unresolvedConfig, cwd)
  const plugins = sortPlugins(
    await resolvePluginTextFiles(pluginsFromConfig(config), cwd),
  )
  const legacyFallbacks = woff2FallbacksFromPlugins(plugins)
  const runtimeMode = resolvePipelineRuntimeMode(
    config.runtime,
    legacyFallbacks,
  )
  const runtime = createRuntimeSelector(runtimeMode)
  const cacheOptions = normalizeCacheOptions(config.cache, cwd)
  const emittedAssets: FontAsset[] = []
  const context = createPluginContext(cwd, emittedAssets)
  const startedPlugins: FontminPlugin[] = []
  let optimizedAssets: FontAsset[] | undefined
  let primaryError: unknown

  try {
    for (const plugin of plugins) {
      startedPlugins.push(plugin)

      if (plugin.buildStart !== undefined) {
        await plugin.buildStart(context)
      }
    }

    let assets = preserveWebDeliverySources(
      await loadInputAssets(config.input ?? [], cwd),
      plugins,
    )
    const protectedInputPaths = assets.flatMap(asset =>
      typeof asset.meta['inputPath'] === 'string'
        ? [asset.meta['inputPath']]
        : [],
    )
    const cacheRuntime =
      cacheOptions.enabled && isCacheablePipeline(plugins)
        ? await cacheRuntimeIdentity(config, plugins, runtime)
        : undefined
    const cacheKey =
      cacheRuntime === undefined
        ? undefined
        : cacheKeyForAssets(assets, config, plugins, cacheRuntime)
    const cachedAssets =
      cacheKey === undefined || cacheRuntime === undefined
        ? undefined
        : await readCachedAssets(cacheOptions.dir, cacheKey, cacheRuntime)

    if (cachedAssets === undefined) {
      const subset = config.subset

      if (subset !== undefined) {
        assets = await flatMapAssets(assets, async asset =>
          runGlyph(asset, subset, await runtime.resolve()),
        )
      }

      for (const plugin of plugins) {
        assets = await transformAssets(assets, plugin, context, runtime)
        assets = [...assets, ...emittedAssets.splice(0)]
      }

      assets = await generateAssets(
        assets,
        plugins,
        context,
        runtime,
        emittedAssets,
      )

      if (cacheKey !== undefined && cacheRuntime !== undefined) {
        await writeCachedAssets(
          cacheOptions.dir,
          cacheKey,
          cacheRuntime,
          assets,
        )
      }
    } else {
      assets = cachedAssets
    }

    if (config.outDir !== undefined) {
      const outDir = resolve(cwd, config.outDir)

      if (config.clean === true) {
        await cleanOutputDirectory(cwd, outDir, protectedInputPaths)
      }

      await writeAssets(outDir, assets)
    }

    optimizedAssets = assets
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
    const error = errorFromUnknown(primaryError)

    throw new Error(error.message, { cause: error })
  }
  if (cleanupError !== undefined) {
    const error = errorFromUnknown(cleanupError)

    throw new Error(error.message, { cause: error })
  }
  if (optimizedAssets === undefined) {
    throw new Error('fontmin-rs optimize did not produce an asset result')
  }

  return optimizedAssets
}

function errorFromUnknown(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error))
}
