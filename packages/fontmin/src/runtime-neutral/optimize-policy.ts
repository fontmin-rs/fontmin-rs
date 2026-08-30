/**
 * Runtime-neutral optimizer semantics shared by Node and browser pipelines.
 * Keep filesystem access, native binding loading, and WASM initialization out
 * of this module.
 */
type MaybePromise<T> = T | Promise<T>

export async function mapWithConcurrency<Input, Output>(
  inputs: readonly Input[],
  concurrency: number,
  transform: (input: Input, index: number) => MaybePromise<Output>,
): Promise<Output[]> {
  if (!Number.isInteger(concurrency) || concurrency < 1) {
    throw new TypeError('parallel thread count must be a positive integer')
  }
  if (inputs.length === 0) {
    return []
  }

  const outputs: Output[] = []
  const failures: unknown[] = []
  let nextIndex = 0
  const worker = async () => {
    while (failures.length === 0) {
      const index = nextIndex
      nextIndex += 1

      if (index >= inputs.length) {
        return
      }
      const input = inputs[index] as Input

      try {
        outputs[index] = await transform(input, index)
      } catch (error) {
        failures.push(error)
      }
    }
  }
  const workerCount = Math.min(concurrency, inputs.length)

  await Promise.all(Array.from({ length: workerCount }, () => worker()))

  if (failures.length > 0) {
    throw failures[0]
  }

  return outputs
}

export const FONT_CONVERSIONS = [
  { inputFormat: 'otf', name: 'otf2ttf', outputFormat: 'ttf' },
  { inputFormat: 'svg', name: 'svg2ttf', outputFormat: 'ttf' },
  { inputFormat: 'ttf', name: 'ttf2eot', outputFormat: 'eot' },
  { inputFormat: 'ttf', name: 'ttf2svg', outputFormat: 'svg' },
  { inputFormat: 'ttf', name: 'ttf2woff', outputFormat: 'woff' },
  { inputFormat: 'ttf', name: 'ttf2woff2', outputFormat: 'woff2' },
] as const

export type FontConversion = (typeof FONT_CONVERSIONS)[number]

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
  concurrency = 1,
): Promise<InputAsset[]> {
  const transformedAssets = await mapWithConcurrency(
    assets,
    concurrency,
    async asset => {
      const result = await transform(asset, context)

      if (result === undefined) {
        return [asset]
      }
      if (Array.isArray(result)) {
        return result.map(asset => normalize(asset))
      }
      if (result === null) {
        return []
      }

      return [normalize(result)]
    },
  )

  return transformedAssets.flat()
}

export async function applyAssetConversion<Asset>(
  assets: Asset[],
  clone: boolean,
  convert: (asset: Asset) => MaybePromise<Asset | undefined>,
  concurrency = 1,
): Promise<Asset[]> {
  const converted = await mapWithConcurrency(
    assets,
    concurrency,
    async asset => ({ asset, convertedAsset: await convert(asset) }),
  )
  const primaryAssets: Asset[] = []
  const clonedAssets: Asset[] = []

  for (const { asset, convertedAsset } of converted) {
    if (convertedAsset === undefined) {
      primaryAssets.push(asset)
    } else if (clone) {
      primaryAssets.push(asset)
      clonedAssets.push(convertedAsset)
    } else {
      primaryAssets.push(convertedAsset)
    }
  }

  return clone ? [...primaryAssets, ...clonedAssets] : primaryAssets
}

export async function applyFontConversion<Asset>(
  assets: Asset[],
  pluginName: string,
  clone: boolean,
  formatOf: (asset: Asset) => string,
  convert: (asset: Asset, conversion: FontConversion) => MaybePromise<Asset>,
  concurrency = 1,
): Promise<Asset[] | undefined> {
  const conversion = FONT_CONVERSIONS.find(
    candidate => candidate.name === pluginName,
  )

  if (conversion === undefined) {
    return undefined
  }

  return applyAssetConversion(
    assets,
    clone,
    asset =>
      formatOf(asset) === conversion.inputFormat
        ? convert(asset, conversion)
        : undefined,
    concurrency,
  )
}

export async function flatMapAssets<Asset>(
  assets: Asset[],
  transform: (asset: Asset) => MaybePromise<Asset[]>,
  concurrency = 1,
): Promise<Asset[]> {
  const transformedAssets = await mapWithConcurrency(
    assets,
    concurrency,
    transform,
  )

  return transformedAssets.flat()
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
