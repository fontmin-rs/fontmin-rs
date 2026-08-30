import { extname } from 'node:path'
import type { OptimizeRuntime } from './optimize-runtime'
import {
  planAutoDeliverySlices,
  unicodeRangesFromCodePoints,
} from './runtime-neutral/auto-delivery'
import { ByteLruCache } from './runtime-neutral/byte-lru-cache'
import { unicodeCodePointsFromSfnt } from './runtime-neutral/sfnt-unicode'
import type { AutoDeliveryOptions, FontAsset, PluginContext } from './types'

const CACHE_MAX_BYTES = 32 * 1024 * 1024
const CACHE_MAX_ENTRIES = 32

export async function runAutoDeliverySlices(
  assets: FontAsset[],
  options: AutoDeliveryOptions,
  runtime: OptimizeRuntime,
  warn: PluginContext['warn'],
): Promise<FontAsset[]> {
  const sources = assets.flatMap((asset, index) =>
    asset.format === 'ttf'
      ? [
          {
            asset,
            codePoints: new Set(unicodeCodePointsFromSfnt(asset.contents)),
            index,
          },
        ]
      : [],
  )
  if (sources.length === 0) {
    return assets
  }

  const supported = [
    ...new Set(sources.flatMap(source => [...source.codePoints])),
  ]
  const subsetCache = new ByteLruCache<string>({
    maxBytes: CACHE_MAX_BYTES,
    maxEntries: CACHE_MAX_ENTRIES,
  })
  const pendingSubsets = new Map<string, Promise<Uint8Array>>()
  const subsetFor = async (
    source: (typeof sources)[number],
    codePoints: readonly number[],
  ): Promise<Uint8Array> => {
    const key = `${source.index}:${codePoints.join(',')}`
    const cached = subsetCache.get(key)
    if (cached !== undefined) {
      return cached
    }
    const pending = pendingSubsets.get(key)
    if (pending !== undefined) {
      return pending
    }

    const operation = runtime.subsetTtf(source.asset.contents, {
      ...options.subset,
      missingGlyphs: 'ignore',
      unicodeRanges: unicodeRangesFromCodePoints(codePoints),
    })
    pendingSubsets.set(key, operation)

    try {
      const result = await operation
      const contents = Buffer.isBuffer(result) ? result : Buffer.from(result)

      subsetCache.set(key, contents)

      return contents
    } finally {
      pendingSubsets.delete(key)
    }
  }
  const measure = async (codePoints: readonly number[]): Promise<number> => {
    const sizes = await Promise.all(
      sources
        .filter(source =>
          codePoints.some(codePoint => source.codePoints.has(codePoint)),
        )
        .map(async source =>
          measureSubset(await subsetFor(source, codePoints), options, runtime),
        ),
    )

    return Math.max(...sizes)
  }
  const plan = await planAutoDeliverySlices(supported, options, measure)
  const maximumBytes = plan.targetBytes * (1 + plan.tolerance)

  for (const slice of plan.slices) {
    if (slice.estimatedBytes > maximumBytes) {
      warn(
        `auto delivery slice ${slice.name} is ${slice.estimatedBytes} bytes, above the ${Math.round(maximumBytes)} byte limit after reaching maxSlices`,
      )
    }
  }

  const output: FontAsset[] = []
  for (const [index, asset] of assets.entries()) {
    const source = sources.find(candidate => candidate.index === index)
    if (source === undefined) {
      output.push(asset)
      continue
    }
    for (const slice of plan.slices) {
      const codePoints = slice.codePoints.filter(codePoint =>
        source.codePoints.has(codePoint),
      )
      if (codePoints.length === 0) {
        continue
      }
      output.push({
        ...asset,
        contents: await subsetFor(source, codePoints),
        path: appendAssetSuffix(asset.path, slice.name),
        meta: {
          ...asset.meta,
          autoDelivery: {
            estimatedBytes: slice.estimatedBytes,
            languages: plan.languages,
            measureFormat: options.measureFormat ?? 'woff2',
            targetBytes: plan.targetBytes,
            tolerance: plan.tolerance,
          },
          cssUnicodeRanges: unicodeRangesFromCodePoints(codePoints),
        },
      })
    }
  }

  return output
}

async function measureSubset(
  contents: Uint8Array,
  options: AutoDeliveryOptions,
  runtime: OptimizeRuntime,
): Promise<number> {
  const format = options.measureFormat ?? 'woff2'

  if (format === 'ttf') {
    return contents.byteLength
  }
  if (format === 'woff') {
    const compressionOptions =
      options.woffCompressionLevel === undefined
        ? {}
        : { compressionLevel: options.woffCompressionLevel }
    const compressed = await runtime.ttfToWoff(contents, compressionOptions)

    return compressed.byteLength
  }

  const compressionOptions =
    options.woff2Quality === undefined ? {} : { quality: options.woff2Quality }
  const compressed = await runtime.ttfToWoff2(contents, compressionOptions)

  return compressed.byteLength
}

function appendAssetSuffix(path: string, suffix: string): string {
  const currentExtension = extname(path)

  return currentExtension === ''
    ? `${path}-${suffix}`
    : `${path.slice(0, -currentExtension.length)}-${suffix}${currentExtension}`
}
