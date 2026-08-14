import { createHash } from 'node:crypto'
import { extname } from 'node:path'
import type {
  ArtifactFormat,
  FontAsset,
  FontminPlugin,
  WebDeliveryManifest,
  WebDeliveryManifestAsset,
  WebDeliveryManifestSource,
  WebDeliveryOptions,
} from './types'

interface CapturedSource {
  asset: FontAsset
  id: string
}

const SOURCE_ID_META_KEY = 'fontminWebDeliverySourceId'
const ORIGINAL_ASSET_META_KEY = 'fontminWebDeliveryOriginalAsset'
const SUBSET_UNICODE_RANGES_META_KEY = 'subsetUnicodeRanges'
const FONT_FORMATS = new Set<ArtifactFormat>([
  'eot',
  'otf',
  'svg',
  'ttf',
  'woff',
  'woff2',
])
const FORMAT_PRIORITY: Partial<Record<ArtifactFormat, number>> = {
  woff2: 0,
  woff: 1,
  otf: 2,
  ttf: 2,
  eot: 3,
  svg: 4,
}

/**
 * Capture full source fonts and emit a Web delivery manifest, preload markup,
 * CSS, and optional fallback assets after all other pipeline work.
 *
 * @param options - Delivery file names, family, and fallback/preload policy.
 * @returns A capture/report plugin pair; spread both into a plugin list.
 */
export function webDelivery(options: WebDeliveryOptions): FontminPlugin[] {
  const normalized = normalizeOptions(options)
  const captured: CapturedSource[] = []
  let nextSourceId = 0

  const capture: FontminPlugin = {
    name: 'fontmin:web-delivery-capture',
    enforce: 'pre',
    buildStart() {
      captured.length = 0
      nextSourceId = 0
    },
    transform(asset) {
      if (!FONT_FORMATS.has(asset.format)) {
        return asset
      }
      const id = `font-${nextSourceId++}`
      const original = originalAssetOf(asset)
      const { [ORIGINAL_ASSET_META_KEY]: _originalAsset, ...publicMeta } =
        asset.meta
      const meta: Record<string, unknown> = {
        ...publicMeta,
        [SOURCE_ID_META_KEY]: id,
      }

      captured.push({ asset: original, id })

      return {
        ...asset,
        meta,
      }
    },
  }

  const report: FontminPlugin = {
    name: 'fontmin:web-delivery',
    enforce: 'post',
    generateBundle(assets, context) {
      emitDeliveryAssets(assets, captured, normalized, context.emitFile)
    },
    buildEnd() {
      captured.length = 0
    },
  }

  return [capture, report]
}

/**
 * Preserve original inputs before a top-level subset runs. This is internal to
 * the optimizer; plugin-driven subsets are already preceded by the capture
 * plugin.
 *
 * @param assets - Original pipeline input assets.
 * @param plugins - Sorted pipeline plugins.
 * @returns Assets carrying private copies for the capture plugin.
 */
export function preserveWebDeliverySources(
  assets: FontAsset[],
  plugins: FontminPlugin[],
): FontAsset[] {
  if (!plugins.some(plugin => plugin.name === 'fontmin:web-delivery-capture')) {
    return assets
  }

  return assets.map(asset =>
    FONT_FORMATS.has(asset.format)
      ? {
          ...asset,
          meta: {
            ...asset.meta,
            [ORIGINAL_ASSET_META_KEY]: cloneAsset(asset),
          },
        }
      : asset,
  )
}

interface NormalizedWebDeliveryOptions {
  basePath: string
  cssFile: string
  fallback: boolean
  fontDisplay: NonNullable<WebDeliveryOptions['fontDisplay']>
  fontFamily: string
  manifestFile: string
  preload: NonNullable<WebDeliveryOptions['preload']>
  preloadFile: string
  selector: string
}

function normalizeOptions(
  options: WebDeliveryOptions,
): NormalizedWebDeliveryOptions {
  if (options.fontFamily.trim().length === 0) {
    throw new TypeError('webDelivery fontFamily must not be empty')
  }

  const normalized = {
    basePath: options.basePath ?? './',
    cssFile: options.cssFile ?? 'fontmin-delivery.css',
    fallback: options.fallback ?? true,
    fontDisplay: options.fontDisplay ?? 'swap',
    fontFamily: options.fontFamily.trim(),
    manifestFile: options.manifestFile ?? 'fontmin-manifest.json',
    preload: options.preload ?? 'first',
    preloadFile: options.preloadFile ?? 'fontmin-preload.html',
    selector: options.selector ?? '.fontmin-fonts',
  }

  for (const [name, path] of [
    ['cssFile', normalized.cssFile],
    ['manifestFile', normalized.manifestFile],
    ['preloadFile', normalized.preloadFile],
  ] as const) {
    if (path.trim().length === 0) {
      throw new TypeError(`webDelivery ${name} must not be empty`)
    }
  }

  return normalized
}

function emitDeliveryAssets(
  assets: FontAsset[],
  captured: CapturedSource[],
  options: NormalizedWebDeliveryOptions,
  emit: (asset: FontAsset) => void,
): void {
  const occupiedPaths = new Set(assets.map(asset => asset.path))
  const manifestSources: WebDeliveryManifestSource[] = []
  const cssBlocks: string[] = []
  const preloadPaths = new Set<string>()
  let hasFallback = false
  let hasSubset = false

  for (const source of captured) {
    const finalFonts = assets
      .filter(asset => asset.meta[SOURCE_ID_META_KEY] === source.id)
      .filter(asset => FONT_FORMATS.has(asset.format))
      .toSorted(compareAssets)
    const preloadAssets = selectPreloads(finalFonts, options.preload)
    hasSubset ||= finalFonts.length > 0
    for (const asset of preloadAssets) {
      preloadPaths.add(asset.path)
    }
    const manifestSource: WebDeliveryManifestSource = {
      id: source.id,
      sourceFormat: source.asset.sourceFormat,
      sourcePath: source.asset.path,
      subsets: finalFonts.map(asset =>
        manifestAsset(asset, preloadPaths.has(asset.path)),
      ),
    }

    for (const group of groupByStem(finalFonts).values()) {
      cssBlocks.push(cssFace(group, `${options.fontFamily} Subset`, options))
    }

    if (options.fallback) {
      const fallback = cloneAsset(source.asset)
      fallback.path = uniqueFallbackPath(fallback, occupiedPaths)
      occupiedPaths.add(fallback.path)
      fallback.meta = withoutInternalMeta(fallback.meta)
      manifestSource.fallback = manifestAsset(fallback, false)
      cssBlocks.push(
        cssFace([fallback], `${options.fontFamily} Fallback`, options),
      )
      hasFallback = true
      emit(fallback)
    }
    manifestSources.push(manifestSource)
  }

  for (const asset of assets) {
    asset.meta = withoutInternalMeta(asset.meta)
  }
  const familyStack = [
    ...(hasSubset ? [`${options.fontFamily} Subset`] : []),
    ...(hasFallback ? [`${options.fontFamily} Fallback`] : []),
  ]
  if (familyStack.length > 0) {
    cssBlocks.push(
      `${options.selector} {\n  font-family: ${familyStack.map(family => cssString(family)).join(', ')};\n}`,
    )
  }
  const preload = [...preloadPaths]
    .toSorted((left, right) => left.localeCompare(right))
    .map(path => preloadLink(path, assets, options.basePath))
    .join('\n')
  const manifest: WebDeliveryManifest = {
    css: options.cssFile,
    fontFamily: options.fontFamily,
    preload: options.preloadFile,
    schemaVersion: 1,
    sources: manifestSources,
  }

  emit(textAsset(options.cssFile, 'css', `${cssBlocks.join('\n\n')}\n`))
  emit(textAsset(options.preloadFile, 'html', `${preload}\n`))
  emit(
    textAsset(
      options.manifestFile,
      'json',
      `${JSON.stringify(manifest, undefined, 2)}\n`,
    ),
  )
}

function selectPreloads(
  assets: FontAsset[],
  policy: NormalizedWebDeliveryOptions['preload'],
): FontAsset[] {
  if (policy === false || assets.length === 0) {
    return []
  }
  const preferred = [...groupByStem(assets).values()]
    .map(group => group.toSorted(compareAssets)[0])
    .filter(asset => asset !== undefined)

  return policy === 'all' ? preferred : preferred.slice(0, 1)
}

function groupByStem(assets: FontAsset[]): Map<string, FontAsset[]> {
  const groups = new Map<string, FontAsset[]>()
  for (const asset of assets) {
    const stem = removeExtension(asset.path)
    const group = groups.get(stem) ?? []
    group.push(asset)
    groups.set(stem, group)
  }

  return groups
}

function compareAssets(left: FontAsset, right: FontAsset): number {
  const pathOrder = removeExtension(left.path).localeCompare(
    removeExtension(right.path),
  )
  if (pathOrder !== 0) {
    return pathOrder
  }

  return (
    (FORMAT_PRIORITY[left.format] ?? 99) -
      (FORMAT_PRIORITY[right.format] ?? 99) ||
    left.path.localeCompare(right.path)
  )
}

function cssFace(
  assets: FontAsset[],
  family: string,
  options: NormalizedWebDeliveryOptions,
): string {
  const sources = assets
    .toSorted(compareAssets)
    .map(
      asset =>
        `url(${cssString(assetUrl(options.basePath, asset.path))}) format(${cssString(cssFormat(asset.format))})`,
    )
    .join(',\n       ')
  const unicodeRanges = unicodeRangesOf(assets[0])
  const rangeDescriptor =
    unicodeRanges.length === 0
      ? ''
      : `\n  unicode-range: ${unicodeRanges.join(', ')};`

  return `@font-face {\n  font-family: ${cssString(family)};\n  src: ${sources};\n  font-display: ${options.fontDisplay};${rangeDescriptor}\n}`
}

function preloadLink(
  path: string,
  assets: FontAsset[],
  basePath: string,
): string {
  const format = assets.find(asset => asset.path === path)?.format ?? 'ttf'

  return `<link rel="preload" href="${htmlAttribute(assetUrl(basePath, path))}" as="font" type="${mimeType(format)}" crossorigin>`
}

function manifestAsset(
  asset: FontAsset,
  preload: boolean,
): WebDeliveryManifestAsset {
  return {
    format: asset.format,
    path: asset.path,
    preload,
    sha256: createHash('sha256').update(asset.contents).digest('hex'),
    size: asset.contents.byteLength,
    unicodeRanges: unicodeRangesOf(asset),
  }
}

function unicodeRangesOf(asset: FontAsset | undefined): string[] {
  const ranges =
    asset?.meta['cssUnicodeRanges'] ??
    asset?.meta[SUBSET_UNICODE_RANGES_META_KEY]
  return Array.isArray(ranges) &&
    ranges.every(range => typeof range === 'string')
    ? ranges
    : []
}

function uniqueFallbackPath(
  asset: FontAsset,
  occupiedPaths: Set<string>,
): string {
  const extension = extname(asset.path)
  const stem = removeExtension(asset.path)
  let path = `${stem}-fallback${extension}`
  let suffix = 2

  while (occupiedPaths.has(path)) {
    path = `${stem}-fallback-${suffix}${extension}`
    suffix += 1
  }

  return path
}

function textAsset(
  path: string,
  format: Extract<ArtifactFormat, 'css' | 'html' | 'json'>,
  contents: string,
): FontAsset {
  return {
    path,
    contents: Buffer.from(contents),
    format,
    sourceFormat: 'unknown',
    meta: {},
  }
}

function cloneAsset(asset: FontAsset): FontAsset {
  return {
    ...asset,
    contents: Buffer.from(asset.contents),
    meta: { ...asset.meta },
  }
}

function originalAssetOf(asset: FontAsset): FontAsset {
  const original = asset.meta[ORIGINAL_ASSET_META_KEY]
  const source =
    typeof original === 'object' && original !== null
      ? cloneAsset(original as FontAsset)
      : cloneAsset(asset)

  source.meta = withoutInternalMeta(source.meta)

  return source
}

function withoutInternalMeta(
  meta: Record<string, unknown>,
): Record<string, unknown> {
  const {
    [ORIGINAL_ASSET_META_KEY]: _originalAsset,
    [SOURCE_ID_META_KEY]: _sourceId,
    ...clean
  } = meta

  return clean
}

function removeExtension(path: string): string {
  const extension = extname(path)
  return extension.length === 0 ? path : path.slice(0, -extension.length)
}

function assetUrl(basePath: string, path: string): string {
  const normalizedBase = basePath.endsWith('/') ? basePath : `${basePath}/`
  const normalizedPath = path
    .split(/[\\/]/u)
    .map(segment => encodeURIComponent(segment))
    .join('/')

  return `${normalizedBase}${normalizedPath}`
}

function cssString(value: string): string {
  return JSON.stringify(value)
}

function htmlAttribute(value: string): string {
  return value.replaceAll('&', '&amp;').replaceAll('"', '&quot;')
}

function cssFormat(format: ArtifactFormat): string {
  const formats: Partial<Record<ArtifactFormat, string>> = {
    eot: 'embedded-opentype',
    otf: 'opentype',
    ttf: 'truetype',
    woff: 'woff',
    woff2: 'woff2',
    svg: 'svg',
  }

  return formats[format] ?? format
}

function mimeType(format: ArtifactFormat): string {
  const types: Partial<Record<ArtifactFormat, string>> = {
    eot: 'application/vnd.ms-fontobject',
    otf: 'font/otf',
    ttf: 'font/ttf',
    woff: 'font/woff',
    woff2: 'font/woff2',
    svg: 'image/svg+xml',
  }

  return types[format] ?? 'application/octet-stream'
}
