import { createHash } from 'node:crypto'
import { extname } from 'node:path'
import type {
  ArtifactFormat,
  FontAsset,
  FontminPlugin,
  WebDeliveryManifest,
  WebDeliveryManifestAsset,
  WebDeliveryManifestSource,
  WebDeliveryManifestSummary,
  WebDeliveryOptions,
} from './types'

interface CapturedSource {
  asset: FontAsset
  id: string
}

const SOURCE_ID_META_KEY = 'fontminWebDeliverySourceId'
const ORIGINAL_ASSET_META_KEY = 'fontminWebDeliveryOriginalAsset'
const SUBSET_UNICODE_RANGES_META_KEY = 'subsetUnicodeRanges'
const DELIVERY_STEM_META_KEY = 'fontminWebDeliveryStem'
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
  hashFileNames: boolean
  hashLength: number
  manifestFile: string
  preload: NonNullable<WebDeliveryOptions['preload']>
  preloadFile: string
  selector: string
  testHtmlFile: string | false
  testText: string
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
    hashFileNames: options.hashFileNames ?? false,
    hashLength: options.hashLength ?? 8,
    manifestFile: options.manifestFile ?? 'fontmin-manifest.json',
    preload: options.preload ?? 'first',
    preloadFile: options.preloadFile ?? 'fontmin-preload.html',
    selector: options.selector ?? '.fontmin-fonts',
    testHtmlFile: options.testHtmlFile ?? false,
    testText: options.testText ?? 'Fontmin delivery preview 字体预览',
  }

  if (
    !Number.isInteger(normalized.hashLength) ||
    normalized.hashLength < 6 ||
    normalized.hashLength > 64
  ) {
    throw new TypeError('webDelivery hashLength must be an integer in [6, 64]')
  }

  for (const [name, path] of [
    ['cssFile', normalized.cssFile],
    ['manifestFile', normalized.manifestFile],
    ['preloadFile', normalized.preloadFile],
    ...(typeof normalized.testHtmlFile === 'string'
      ? ([['testHtmlFile', normalized.testHtmlFile]] as const)
      : []),
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
  const pathRewrites = new Map<string, string>()
  let hasFallback = false
  let hasSubset = false

  for (const source of captured) {
    const finalFonts = assets
      .filter(asset => asset.meta[SOURCE_ID_META_KEY] === source.id)
      .filter(asset => FONT_FORMATS.has(asset.format))
      .toSorted(compareAssets)
    if (options.hashFileNames) {
      for (const asset of finalFonts) {
        hashAssetPath(asset, occupiedPaths, pathRewrites, options.hashLength)
      }
    }
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
      if (options.hashFileNames) {
        hashAssetPath(fallback, occupiedPaths, pathRewrites, options.hashLength)
      }
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

  rewriteAssetReferences(assets, pathRewrites)

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
    summary: deliverySummary(captured, manifestSources),
    ...(typeof options.testHtmlFile === 'string'
      ? { testHtml: options.testHtmlFile }
      : {}),
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
  if (typeof options.testHtmlFile === 'string') {
    emit(
      textAsset(
        options.testHtmlFile,
        'html',
        deliveryTestHtml(manifest, familyStack, options),
      ),
    )
  }
}

function hashAssetPath(
  asset: FontAsset,
  occupiedPaths: Set<string>,
  rewrites: Map<string, string>,
  hashLength: number,
): void {
  const original = asset.path
  const extension = extname(original)
  const stem = removeExtension(original)
  const hash = createHash('sha256')
    .update(asset.contents)
    .digest('hex')
    .slice(0, hashLength)
  let path = `${stem}.${hash}${extension}`
  let suffix = 2

  occupiedPaths.delete(original)
  while (occupiedPaths.has(path)) {
    path = `${stem}.${hash}-${suffix}${extension}`
    suffix += 1
  }
  occupiedPaths.add(path)
  asset.path = path
  asset.meta[DELIVERY_STEM_META_KEY] = stem
  rewrites.set(original, path)
}

function rewriteAssetReferences(
  assets: FontAsset[],
  rewrites: Map<string, string>,
): void {
  if (rewrites.size === 0) {
    return
  }
  const replacements = [...rewrites]
    .flatMap(
      ([from, to]) =>
        [
          [from, to],
          [encodedAssetPath(from), encodedAssetPath(to)],
        ] as const,
    )
    .toSorted(([left], [right]) => right.length - left.length)

  for (const asset of assets) {
    if (asset.format !== 'css' && asset.format !== 'html') {
      continue
    }
    let contents = new TextDecoder().decode(asset.contents)
    for (const [from, to] of replacements) {
      contents = contents.replaceAll(from, to)
    }
    asset.contents = Buffer.from(contents)
  }
}

function deliverySummary(
  captured: CapturedSource[],
  sources: WebDeliveryManifestSource[],
): WebDeliveryManifestSummary {
  const subsets = sources.flatMap(source => source.subsets)
  const ranges = subsets.flatMap(asset => asset.unicodeRanges)

  return {
    codePointCount: coveredCodePointCount(ranges),
    fallbackBytes: sources.reduce(
      (total, source) => total + (source.fallback?.size ?? 0),
      0,
    ),
    requestCount: sources.reduce(
      (total, source) =>
        total +
        new Set(source.subsets.map(asset => asset.unicodeRanges.join(',')))
          .size,
      0,
    ),
    sourceBytes: captured.reduce(
      (total, source) => total + source.asset.contents.byteLength,
      0,
    ),
    subsetBytes: subsets.reduce((total, asset) => total + asset.size, 0),
    subsetCount: subsets.length,
  }
}

function coveredCodePointCount(ranges: string[]): number {
  const intervals = ranges
    .map(range =>
      /^U\+(?<start>[0-9A-F]+)(?:-(?<end>[0-9A-F]+))?$/u.exec(range),
    )
    .flatMap(match => {
      if (match?.groups === undefined) {
        return []
      }
      const start = Number.parseInt(match.groups['start'] ?? '', 16)
      const end = Number.parseInt(
        match.groups['end'] ?? match.groups['start'] ?? '',
        16,
      )

      return Number.isInteger(start) && Number.isInteger(end)
        ? [{ end, start }]
        : []
    })
    .toSorted((left, right) => left.start - right.start || left.end - right.end)
  let count = 0
  let currentEnd = -1

  for (const interval of intervals) {
    if (interval.end <= currentEnd) {
      continue
    }
    const start = Math.max(interval.start, currentEnd + 1)
    count += interval.end - start + 1
    currentEnd = interval.end
  }

  return count
}

function deliveryTestHtml(
  manifest: WebDeliveryManifest,
  familyStack: string[],
  options: NormalizedWebDeliveryOptions,
): string {
  const rows = manifest.sources
    .flatMap(source =>
      source.subsets.map(
        asset =>
          `<tr><td>${htmlText(asset.path)}</td><td>${asset.size}</td><td>${htmlText(asset.unicodeRanges.join(', '))}</td></tr>`,
      ),
    )
    .join('\n')
  const family = familyStack.map(value => cssString(value)).join(', ')

  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>${htmlText(options.fontFamily)} delivery preview</title>
  <link rel="stylesheet" href="${htmlAttribute(assetUrl(options.basePath, options.cssFile))}">
</head>
<body>
  <main>
    <h1>${htmlText(options.fontFamily)}</h1>
    <p style="font-family: ${htmlAttribute(family)}">${htmlText(options.testText)}</p>
    <p>${manifest.summary.requestCount} requests · ${manifest.summary.codePointCount} code points · ${manifest.summary.subsetBytes} subset bytes</p>
    <table><thead><tr><th>Asset</th><th>Bytes</th><th>Unicode ranges</th></tr></thead><tbody>
${rows}
    </tbody></table>
  </main>
</body>
</html>
`
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
    const stem = assetStem(asset)
    const group = groups.get(stem) ?? []
    group.push(asset)
    groups.set(stem, group)
  }

  return groups
}

function compareAssets(left: FontAsset, right: FontAsset): number {
  const pathOrder = assetStem(left).localeCompare(assetStem(right))
  if (pathOrder !== 0) {
    return pathOrder
  }

  return (
    (FORMAT_PRIORITY[left.format] ?? 99) -
      (FORMAT_PRIORITY[right.format] ?? 99) ||
    left.path.localeCompare(right.path)
  )
}

function assetStem(asset: FontAsset): string {
  const metadataStem = asset.meta[DELIVERY_STEM_META_KEY]

  return typeof metadataStem === 'string'
    ? metadataStem
    : removeExtension(asset.path)
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
    [DELIVERY_STEM_META_KEY]: _deliveryStem,
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

  return `${normalizedBase}${encodedAssetPath(path)}`
}

function encodedAssetPath(path: string): string {
  return path
    .split(/[\\/]/u)
    .map(segment => encodeURIComponent(segment))
    .join('/')
}

function cssString(value: string): string {
  return JSON.stringify(value)
}

function htmlAttribute(value: string): string {
  return value.replaceAll('&', '&amp;').replaceAll('"', '&quot;')
}

function htmlText(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
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
