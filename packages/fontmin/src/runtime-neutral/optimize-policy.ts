/**
 * Runtime-neutral optimizer semantics shared by Node and browser pipelines.
 * Keep filesystem access, native binding loading, and WASM initialization out
 * of this module.
 */
type MaybePromise<T> = T | Promise<T>

export interface DeliverySlice {
  name: string
  unicodeRanges: string[]
}

interface DeliverySliceNormalizationOptions {
  allowEmpty?: boolean
}

interface MissingGlyphReport {
  missing: number[]
}

export async function applyAssetTransform<InputAsset, OutputAsset, Context>(
  assets: InputAsset[],
  transform: (
    asset: InputAsset,
    context: Context,
  ) => MaybePromise<OutputAsset | OutputAsset[] | null | undefined>,
  context: Context,
  normalize: (asset: OutputAsset) => InputAsset,
): Promise<InputAsset[]> {
  const transformedAssets: InputAsset[] = []

  for (const asset of assets) {
    const result = await transform(asset, context)

    if (result === undefined) {
      transformedAssets.push(asset)
    } else if (Array.isArray(result)) {
      transformedAssets.push(...result.map(asset => normalize(asset)))
    } else if (result !== null) {
      transformedAssets.push(normalize(result))
    }
  }

  return transformedAssets
}

export async function flatMapAssets<Asset>(
  assets: Asset[],
  transform: (asset: Asset) => MaybePromise<Asset[]>,
): Promise<Asset[]> {
  const transformedAssets: Asset[] = []

  for (const asset of assets) {
    transformedAssets.push(...(await transform(asset)))
  }

  return transformedAssets
}

export function missingGlyphWarning(
  report: MissingGlyphReport,
): string | undefined {
  if (report.missing.length === 0) {
    return undefined
  }

  const visible = report.missing
    .slice(0, 16)
    .map(
      codePoint => `U+${codePoint.toString(16).toUpperCase().padStart(4, '0')}`,
    )
    .join(', ')
  const remaining = report.missing.length - 16

  return `missing glyphs for requested Unicode code points: ${visible}${remaining > 0 ? `, and ${remaining} more` : ''}`
}

export function normalizeDeliverySlices(
  values: unknown,
  options: DeliverySliceNormalizationOptions = {},
): DeliverySlice[] {
  if (!Array.isArray(values)) {
    throw new TypeError('unicode delivery slices must be an array')
  }
  if (values.length === 0) {
    if (options.allowEmpty === true) {
      return []
    }

    throw new Error('unicode delivery slices must not be empty')
  }

  const names = new Set<string>()

  return values.map((value, index) => {
    if (typeof value !== 'object' || value === null || Array.isArray(value)) {
      throw new Error(`unicode delivery slice ${index + 1} must be an object`)
    }

    const { name, unicodeRanges } = value as {
      name?: unknown
      unicodeRanges?: unknown
    }

    if (
      typeof name !== 'string' ||
      name.length === 0 ||
      !/^[A-Za-z0-9_-]+$/u.test(name)
    ) {
      throw new Error(
        `unicode delivery slice ${index + 1} must have a name containing only letters, digits, hyphens, or underscores`,
      )
    }
    if (names.has(name)) {
      throw new Error(`unicode delivery slice name is duplicated: ${name}`)
    }
    if (
      !Array.isArray(unicodeRanges) ||
      unicodeRanges.length === 0 ||
      unicodeRanges.some(
        range => typeof range !== 'string' || range.length === 0,
      )
    ) {
      throw new Error(
        `unicode delivery slice ${name} must include at least one Unicode range`,
      )
    }

    names.add(name)

    return { name, unicodeRanges: [...unicodeRanges] }
  })
}
