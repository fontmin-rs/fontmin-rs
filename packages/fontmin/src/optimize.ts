import { createHash } from 'node:crypto'
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { basename, dirname, extname, join, resolve } from 'node:path'
import { glob } from 'tinyglobby'
import { inspect } from './native'
import {
  createRuntimeSelector,
  resolvePipelineRuntimeMode,
} from './optimize-runtime'
import type { OptimizeRuntime, RuntimeSelector } from './optimize-runtime'
import type {
  AssetFormat,
  CacheOptions,
  ConfigOutput,
  DeliverySlice,
  CssFontSource,
  CssGlyph,
  CssOptions,
  FontAsset,
  FontFormat,
  FontminConfig,
  FontminPlugin,
  Otf2TtfOptions,
  OutputConfig,
  OutputFormat,
  PluginContext,
  SubsetOptions,
  Svg2TtfOptions,
  SvgIcon,
  Svgs2TtfOptions,
  Ttf2EotOptions,
  Ttf2SvgOptions,
  Ttf2Woff2Options,
  WoffOptions,
} from './types'

type BuiltinPlugin = FontminPlugin & {
  native: NonNullable<FontminPlugin['native']>
}

interface NormalizedCacheOptions {
  dir: string
  enabled: boolean
}

interface OutputPathOptions {
  ext?: string
  fileName?: string
}

type CssPluginOptions = CssOptions & OutputPathOptions

interface CacheAssetRecord {
  fileName: string
  format: AssetFormat
  meta: Record<string, unknown>
  path: string
  sourceFormat: FontFormat
}

interface CacheManifest {
  assets: CacheAssetRecord[]
  key: string
  runtime: CacheRuntimeIdentity
  version: string
}

interface CacheRuntimeIdentity {
  requested: RuntimeSelector['requested']
  resolved: OptimizeRuntime['kind'] | null
}

interface CacheIndex {
  entries: Record<
    string,
    {
      assets: string[]
      updatedAt: string
    }
  >
  version: string
}

const CACHE_SCHEMA_VERSION = 'v1'
const FONTMIN_VERSION = '0.1.0-beta.2'
const DEFAULT_CACHE_DIR = 'node_modules/.cache/fontmin-rs'
const DEFAULT_SVG_ICON_START_UNICODE = 57_345
const CSS_GLYPHS_META_KEY = 'cssGlyphs'
const CSS_UNICODE_RANGES_META_KEY = 'cssUnicodeRanges'

const MIME_TYPES_BY_FORMAT: Record<CssFontSource['format'], string> = {
  eot: 'application/vnd.ms-fontobject',
  svg: 'image/svg+xml',
  ttf: 'font/ttf',
  woff: 'font/woff',
  woff2: 'font/woff2',
}

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

  for (const plugin of plugins) {
    if (plugin.buildStart !== undefined) {
      await plugin.buildStart(context)
    }
  }

  let assets = await loadInputAssets(config.input ?? [], cwd)
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

    for (const plugin of plugins) {
      if (isBuiltin(plugin, 'css')) {
        const cssAsset = await runCss(
          assets,
          plugin.native.options as CssOptions,
          await runtime.resolve(),
        )
        if (cssAsset !== undefined) {
          assets = [...assets, cssAsset]
        }
      } else {
        if (plugin.generateBundle !== undefined) {
          await plugin.generateBundle(assets, context)
        }
        assets = [...assets, ...emittedAssets.splice(0)]
      }
    }

    if (cacheKey !== undefined && cacheRuntime !== undefined) {
      await writeCachedAssets(cacheOptions.dir, cacheKey, cacheRuntime, assets)
    }
  } else {
    assets = cachedAssets
  }

  if (config.outDir !== undefined) {
    const outDir = resolve(cwd, config.outDir)

    if (config.clean === true) {
      await rm(outDir, { recursive: true, force: true })
    }

    await writeAssets(outDir, assets)
  }

  for (const plugin of plugins) {
    if (plugin.buildEnd !== undefined) {
      await plugin.buildEnd(context)
    }
  }

  return assets
}

function createPluginContext(
  cwd: string,
  emittedAssets: FontAsset[],
): PluginContext {
  const diagnostics: PluginContext['diagnostics'] = []

  return {
    cwd,
    diagnostics,
    emitFile(asset) {
      emittedAssets.push(asset)
    },
    readFile(path) {
      return readFile(resolve(cwd, path))
    },
    resolve(path) {
      return resolve(cwd, path)
    },
    warn(message) {
      diagnostics.push({
        level: 'warn',
        message: warningMessage(message),
      })
    },
    async writeFile(path, contents) {
      const filePath = resolve(cwd, path)

      await mkdir(dirname(filePath), { recursive: true })
      await writeFile(filePath, contents)
    },
  }
}

function warningMessage(message: string | Error): string {
  return typeof message === 'string' ? message : message.message
}

async function resolveConfigTextFile(
  config: FontminConfig,
  cwd: string,
): Promise<FontminConfig> {
  if (config.subset === undefined) {
    return config
  }

  const subset = await resolveSubsetTextFile(config.subset, cwd)

  if (subset === config.subset) {
    return config
  }

  return {
    ...config,
    subset,
  }
}

async function resolvePluginTextFiles(
  plugins: FontminPlugin[],
  cwd: string,
): Promise<FontminPlugin[]> {
  const resolvedPlugins: FontminPlugin[] = []

  for (const plugin of plugins) {
    resolvedPlugins.push(await resolvePluginTextFile(plugin, cwd))
  }

  return resolvedPlugins
}

async function resolvePluginTextFile(
  plugin: FontminPlugin,
  cwd: string,
): Promise<FontminPlugin> {
  if (!isBuiltin(plugin, 'glyph')) {
    return plugin
  }

  const options = await resolveSubsetTextFile(
    plugin.native.options as SubsetOptions,
    cwd,
  )

  if (options === plugin.native.options) {
    return plugin
  }

  return {
    ...plugin,
    native: {
      ...plugin.native,
      options: options as Record<string, unknown>,
    },
  }
}

async function resolveSubsetTextFile(
  options: SubsetOptions,
  cwd: string,
): Promise<SubsetOptions> {
  if (options.textFile === undefined) {
    return options
  }

  const fileText = await readFile(resolve(cwd, options.textFile), 'utf8')
  const { textFile: _textFile, ...resolvedOptions } = options

  return {
    ...resolvedOptions,
    text: mergeSubsetText(options.text, fileText),
  }
}

function mergeSubsetText(text: string | undefined, fileText: string): string {
  return text === undefined ? fileText : `${text}${fileText}`
}

function pluginsFromConfig(config: FontminConfig): FontminPlugin[] {
  const plugins = [...(config.plugins ?? [])]

  if (config.outputs === undefined) {
    return plugins
  }

  const outputs = config.outputs.map(normalizeOutputConfig)
  const requestedOutputs = outputs.map(output => output.format)
  const fontOutputs = requestedOutputs.filter(format => format !== 'css')

  for (const output of outputs) {
    const plugin = outputPluginFromConfig(output)

    if (plugin !== undefined) {
      plugins.push(plugin)
    }
  }

  if (fontOutputs.length > 0) {
    plugins.push(outputFilterPlugin(fontOutputs))
  }

  if (requestedOutputs.includes('css')) {
    const cssOutput = outputs.find(output => output.format === 'css')

    plugins.push(
      builtinPlugin('css', {
        ...cssOptionsRecord(config.css),
        ...outputPathOptionsRecord(cssOutput),
      }),
      outputFilterPlugin(requestedOutputs, 'post'),
    )
  }

  return plugins
}

function normalizeOutputConfig(output: ConfigOutput): OutputConfig {
  if (typeof output === 'string') {
    return {
      clone: true,
      format: output,
    }
  }

  const config: OutputConfig = {
    clone: output.clone ?? true,
    format: output.format,
  }

  if (output.ext !== undefined) {
    config.ext = output.ext
  }
  if (output.fileName !== undefined) {
    config.fileName = output.fileName
  }

  return config
}

function outputPluginFromConfig(
  output: OutputConfig,
): FontminPlugin | undefined {
  if (output.format === 'ttf' || output.format === 'css') {
    return undefined
  }

  const options = {
    clone: output.clone ?? true,
    ...outputPathOptionsRecord(output),
  }

  if (output.format === 'eot') {
    return builtinPlugin('ttf2eot', options)
  }
  if (output.format === 'svg') {
    return builtinPlugin('ttf2svg', options)
  }
  if (output.format === 'woff') {
    return builtinPlugin('ttf2woff', options)
  }

  return builtinPlugin('ttf2woff2', options)
}

function outputPathOptionsRecord(
  output: OutputConfig | undefined,
): Record<string, unknown> {
  const record: Record<string, unknown> = {}

  if (output?.ext !== undefined) {
    record['ext'] = output.ext
  }
  if (output?.fileName !== undefined) {
    record['fileName'] = output.fileName
  }

  return record
}

function builtinPlugin(
  name: string,
  options: Record<string, unknown>,
): FontminPlugin {
  return {
    name: `fontmin:${name}`,
    native: {
      kind: 'builtin',
      name,
      options,
    },
  }
}

function outputFilterPlugin(
  formats: OutputFormat[],
  enforce?: FontminPlugin['enforce'],
): FontminPlugin {
  const plugin: FontminPlugin = {
    name: 'fontmin:output-filter',
    generateBundle(assets) {
      const retainedAssets = assets.filter(asset => {
        const format = outputFormatFromAsset(asset)

        return format !== undefined && formats.includes(format)
      })

      assets.splice(0, assets.length, ...retainedAssets)
    },
  }

  if (enforce !== undefined) {
    plugin.enforce = enforce
  }

  return plugin
}

function outputFormatFromAsset(asset: FontAsset): OutputFormat | undefined {
  if (asset.format === 'unknown' || asset.format === 'otf') {
    return undefined
  }

  return asset.format
}

function cssOptionsRecord(
  options: CssOptions | undefined,
): Record<string, unknown> {
  const record: Record<string, unknown> = {}

  if (options?.base64 !== undefined) {
    record['base64'] = options.base64
  }
  if (options?.asFileName !== undefined) {
    record['asFileName'] = options.asFileName
  }
  if (options?.fontDisplay !== undefined) {
    record['fontDisplay'] = options.fontDisplay
  }
  if (options?.fontFamily !== undefined) {
    record['fontFamily'] = options.fontFamily
  }
  if (options?.fontPath !== undefined) {
    record['fontPath'] = options.fontPath
  }
  if (options?.glyph !== undefined) {
    record['glyph'] = options.glyph
  }
  if (options?.iconPrefix !== undefined) {
    record['iconPrefix'] = options.iconPrefix
  }
  if (options?.local !== undefined) {
    record['local'] = options.local
  }
  if (options?.target !== undefined) {
    record['target'] = options.target
  }
  if (options?.unicodeRanges !== undefined) {
    record['unicodeRanges'] = options.unicodeRanges
  }

  return record
}

async function readCachedAssets(
  cacheDir: string,
  key: string,
  runtime: CacheRuntimeIdentity,
): Promise<FontAsset[] | undefined> {
  let manifest: CacheManifest

  try {
    manifest = JSON.parse(
      await readFile(cacheManifestPath(cacheDir, key), 'utf8'),
    ) as CacheManifest
  } catch {
    return undefined
  }

  if (
    manifest.version !== CACHE_SCHEMA_VERSION ||
    manifest.key !== key ||
    manifest.runtime?.requested !== runtime.requested ||
    manifest.runtime.resolved !== runtime.resolved
  ) {
    return undefined
  }

  const entryDir = cacheEntryDir(cacheDir, key)
  const assets: FontAsset[] = []

  try {
    for (const record of manifest.assets) {
      const contents = await readFile(join(entryDir, record.fileName))

      assets.push({
        path: record.path,
        contents,
        format: record.format,
        sourceFormat: record.sourceFormat,
        meta: {
          ...record.meta,
          cache: {
            hit: true,
            key,
          },
        },
      })
    }
  } catch {
    return undefined
  }

  return assets
}

async function writeCachedAssets(
  cacheDir: string,
  key: string,
  runtime: CacheRuntimeIdentity,
  assets: FontAsset[],
): Promise<void> {
  const entryDir = cacheEntryDir(cacheDir, key)
  const records: CacheAssetRecord[] = []

  await mkdir(entryDir, { recursive: true })

  for (const [index, asset] of assets.entries()) {
    const fileName = `${String(index).padStart(3, '0')}.${asset.format}`

    await writeFile(join(entryDir, fileName), asset.contents)
    records.push({
      fileName,
      format: asset.format,
      meta: asset.meta,
      path: asset.path,
      sourceFormat: asset.sourceFormat,
    })
  }

  await writeFile(
    cacheManifestPath(cacheDir, key),
    `${JSON.stringify(
      {
        assets: records,
        key,
        runtime,
        version: CACHE_SCHEMA_VERSION,
      } satisfies CacheManifest,
      undefined,
      2,
    )}\n`,
  )
  await updateCacheIndex(cacheDir, key, records)
}

async function updateCacheIndex(
  cacheDir: string,
  key: string,
  assets: CacheAssetRecord[],
): Promise<void> {
  const indexPath = cacheIndexPath(cacheDir)
  let index: CacheIndex = {
    entries: {},
    version: CACHE_SCHEMA_VERSION,
  }

  try {
    index = JSON.parse(await readFile(indexPath, 'utf8')) as CacheIndex
  } catch {
    // A missing or corrupted cache index can be rebuilt from the next writes.
  }

  if (index.version !== CACHE_SCHEMA_VERSION) {
    index = {
      entries: {},
      version: CACHE_SCHEMA_VERSION,
    }
  }

  index.entries[key] = {
    assets: assets.map(asset => asset.path),
    updatedAt: new Date().toISOString(),
  }

  await mkdir(dirname(indexPath), { recursive: true })
  await writeFile(indexPath, `${JSON.stringify(index, undefined, 2)}\n`)
}

async function loadInputAssets(
  inputs: (string | Uint8Array)[],
  cwd: string,
): Promise<FontAsset[]> {
  if (inputs.length === 0) {
    throw new Error('fontmin-rs optimize requires at least one input')
  }

  const assets: FontAsset[] = []

  for (const input of inputs) {
    if (typeof input === 'string') {
      const inputPaths = await expandInputPath(input, cwd)

      for (const inputPath of inputPaths) {
        const contents = await readFile(inputPath)
        const format = detectFormat(contents)

        assets.push({
          path: basename(inputPath),
          contents,
          format,
          sourceFormat: format,
          meta: { inputPath },
        })
      }
    } else {
      const contents = Buffer.from(input)
      const format = detectFormat(contents)

      assets.push({
        path: `fontmin.${extensionForFormat(format)}`,
        contents,
        format,
        sourceFormat: format,
        meta: {},
      })
    }
  }

  return assets
}

async function expandInputPath(input: string, cwd: string): Promise<string[]> {
  if (!isGlobPattern(input)) {
    return [resolve(cwd, input)]
  }

  const matches = await glob(input, {
    absolute: true,
    cwd,
    onlyFiles: true,
  })

  if (matches.length === 0) {
    throw new Error(`fontmin-rs input glob matched no files: ${input}`)
  }

  return matches.sort((left, right) => left.localeCompare(right))
}

function isGlobPattern(path: string): boolean {
  return /[*?[\]{}]/u.test(path)
}

async function transformAssets(
  assets: FontAsset[],
  plugin: FontminPlugin,
  context: PluginContext,
  runtime: RuntimeSelector,
): Promise<FontAsset[]> {
  if (isBuiltin(plugin, 'glyph')) {
    return flatMapAssets(assets, async asset =>
      runGlyph(
        asset,
        plugin.native.options as SubsetOptions,
        await runtime.resolve(),
      ),
    )
  }

  if (isBuiltin(plugin, 'unicodeSlices')) {
    return flatMapAssets(assets, async asset =>
      runUnicodeSlices(asset, plugin.native.options, await runtime.resolve()),
    )
  }

  if (isBuiltin(plugin, 'otf2ttf')) {
    return flatMapAssets(assets, async asset =>
      runOtf2Ttf(asset, plugin.native.options, await runtime.resolve()),
    )
  }

  if (isBuiltin(plugin, 'ttf2woff')) {
    return flatMapAssets(assets, async asset =>
      runTtf2Woff(asset, plugin.native.options, await runtime.resolve()),
    )
  }

  if (isBuiltin(plugin, 'ttf2woff2')) {
    return flatMapAssets(assets, async asset =>
      runTtf2Woff2(asset, plugin.native.options, await runtime.resolve()),
    )
  }

  if (isBuiltin(plugin, 'ttf2eot')) {
    return flatMapAssets(assets, async asset =>
      runTtf2Eot(asset, plugin.native.options, await runtime.resolve()),
    )
  }

  if (isBuiltin(plugin, 'ttf2svg')) {
    return flatMapAssets(assets, async asset =>
      runTtf2Svg(asset, plugin.native.options, await runtime.resolve()),
    )
  }

  if (isBuiltin(plugin, 'svg2ttf')) {
    return flatMapAssets(assets, async asset =>
      runSvg2Ttf(asset, plugin.native.options, await runtime.resolve()),
    )
  }

  if (isBuiltin(plugin, 'svgs2ttf')) {
    return runSvgs2Ttf(assets, plugin.native.options, await runtime.resolve())
  }

  if (isBuiltin(plugin, 'css')) {
    return assets
  }

  if (plugin.transform === undefined) {
    return assets
  }

  const transformedAssets: FontAsset[] = []

  for (const asset of assets) {
    const result = await plugin.transform(asset, context)

    if (result === undefined) {
      transformedAssets.push(asset)
    } else if (Array.isArray(result)) {
      transformedAssets.push(...result)
    } else if (result !== null) {
      transformedAssets.push(result)
    }
  }

  return transformedAssets
}

async function flatMapAssets(
  assets: FontAsset[],
  transform: (asset: FontAsset) => Promise<FontAsset[]>,
): Promise<FontAsset[]> {
  const transformed: FontAsset[] = []

  for (const asset of assets) {
    transformed.push(...(await transform(asset)))
  }

  return transformed
}

async function runGlyph(
  asset: FontAsset,
  options: SubsetOptions,
  runtime: OptimizeRuntime,
): Promise<FontAsset[]> {
  if (asset.format !== 'ttf') {
    return [asset]
  }
  const meta = withCssGlyphs(asset.meta, cssGlyphsFromSubsetOptions(options))
  const subsetAsset: FontAsset = {
    path: replaceExtension(asset.path, 'ttf'),
    contents: Buffer.from(
      await runtime.subsetTtf(asset.contents, runtimeSubsetOptions(options)),
    ),
    format: 'ttf',
    sourceFormat: asset.sourceFormat,
    meta,
  }

  return options.clone === true ? [asset, subsetAsset] : [subsetAsset]
}

async function runUnicodeSlices(
  asset: FontAsset,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset[]> {
  if (asset.format !== 'ttf') {
    return [asset]
  }

  return Promise.all(
    deliverySlicesFromOptions(options).map(async slice => ({
      path: appendAssetSuffix(asset.path, slice.name),
      contents: Buffer.from(
        await runtime.subsetTtf(asset.contents, {
          missingGlyphs: 'ignore',
          unicodeRanges: slice.unicodeRanges,
        }),
      ),
      format: 'ttf' as const,
      sourceFormat: asset.sourceFormat,
      meta: {
        ...asset.meta,
        [CSS_UNICODE_RANGES_META_KEY]: slice.unicodeRanges,
      },
    })),
  )
}

function deliverySlicesFromOptions(
  options: Record<string, unknown>,
): DeliverySlice[] {
  const values = options['slices']

  if (!Array.isArray(values) || values.length === 0) {
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

function runtimeSubsetOptions(options: SubsetOptions): SubsetOptions {
  const { clone: _clone, ...runtimeOptions } = options

  return Object.fromEntries(
    Object.entries(runtimeOptions).filter(([, value]) => value !== undefined),
  ) as SubsetOptions
}

async function runTtf2Woff(
  asset: FontAsset,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset[]> {
  if (asset.format !== 'ttf') {
    return [asset]
  }

  const woffAsset: FontAsset = {
    path: outputPathForAsset(asset.path, 'woff', options),
    contents: Buffer.from(
      await runtime.ttfToWoff(asset.contents, woffOptions(options)),
    ),
    format: 'woff',
    sourceFormat: asset.sourceFormat,
    meta: convertedMeta(asset),
  }

  return options['clone'] === false ? [woffAsset] : [asset, woffAsset]
}

async function runTtf2Woff2(
  asset: FontAsset,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset[]> {
  if (asset.format !== 'ttf') {
    return [asset]
  }

  const woff2Asset: FontAsset = {
    path: outputPathForAsset(asset.path, 'woff2', options),
    contents: Buffer.from(
      await runtime.ttfToWoff2(asset.contents, woff2Options(options)),
    ),
    format: 'woff2',
    sourceFormat: asset.sourceFormat,
    meta: convertedMeta(asset),
  }

  return options['clone'] === false ? [woff2Asset] : [asset, woff2Asset]
}

async function runTtf2Eot(
  asset: FontAsset,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset[]> {
  if (asset.format !== 'ttf') {
    return [asset]
  }

  const eotAsset: FontAsset = {
    path: outputPathForAsset(asset.path, 'eot', options),
    contents: Buffer.from(
      await runtime.ttfToEot(asset.contents, eotOptions(options)),
    ),
    format: 'eot',
    sourceFormat: asset.sourceFormat,
    meta: convertedMeta(asset),
  }

  return options['clone'] === false ? [eotAsset] : [asset, eotAsset]
}

async function runTtf2Svg(
  asset: FontAsset,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset[]> {
  if (asset.format !== 'ttf') {
    return [asset]
  }

  const svgAsset: FontAsset = {
    path: outputPathForAsset(asset.path, 'svg', options),
    contents: Buffer.from(
      await runtime.ttfToSvg(asset.contents, svgOptions(options)),
    ),
    format: 'svg',
    sourceFormat: asset.sourceFormat,
    meta: convertedMeta(asset),
  }

  return options['clone'] === false ? [svgAsset] : [asset, svgAsset]
}

async function runOtf2Ttf(
  asset: FontAsset,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset[]> {
  if (asset.format !== 'otf') {
    return [asset]
  }

  const ttfAsset: FontAsset = {
    path: outputPathForAsset(asset.path, 'ttf', options),
    contents: Buffer.from(
      await runtime.otfToTtf(asset.contents, otf2TtfOptions(options)),
    ),
    format: 'ttf',
    sourceFormat: asset.sourceFormat,
    meta: convertedMeta(asset),
  }

  return options['clone'] === false ? [ttfAsset] : [asset, ttfAsset]
}

async function runSvg2Ttf(
  asset: FontAsset,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset[]> {
  if (asset.format !== 'svg') {
    return [asset]
  }

  const ttfAsset: FontAsset = {
    path: outputPathForAsset(asset.path, 'ttf', options),
    contents: Buffer.from(
      await runtime.svgFontToTtf(
        Buffer.from(asset.contents).toString('utf8'),
        svg2TtfOptions(options),
      ),
    ),
    format: 'ttf',
    sourceFormat: asset.sourceFormat,
    meta: convertedMeta(asset),
  }

  return options['clone'] === false ? [ttfAsset] : [asset, ttfAsset]
}

async function runSvgs2Ttf(
  assets: FontAsset[],
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset[]> {
  const svgAssets = assets.filter(asset => asset.format === 'svg')

  if (svgAssets.length === 0) {
    return assets
  }

  const nonSvgAssets = assets.filter(asset => asset.format !== 'svg')
  const firstSvg = svgAssets[0]

  if (firstSvg === undefined) {
    return assets
  }

  const fontName =
    typeof options['fontName'] === 'string'
      ? options['fontName']
      : basenameWithoutExtension(firstSvg.path)
  const icons = svgAssets.map((asset, index) => svgIconFromAsset(asset, index))
  const cssGlyphs = cssGlyphsFromSvgIcons(
    icons,
    typeof options['startUnicode'] === 'number'
      ? options['startUnicode']
      : DEFAULT_SVG_ICON_START_UNICODE,
  )
  const ttfAsset: FontAsset = {
    path: `${fontName}.ttf`,
    contents: Buffer.from(
      await runtime.svgsToTtf(icons, svgs2TtfOptions(options)),
    ),
    format: 'ttf',
    sourceFormat: firstSvg.sourceFormat,
    meta: {
      [CSS_GLYPHS_META_KEY]: cssGlyphs,
      sourcePaths: svgAssets.map(asset => asset.path),
    },
  }

  return options['clone'] === true
    ? [...assets, ttfAsset]
    : [...nonSvgAssets, ttfAsset]
}

function convertedMeta(asset: FontAsset): Record<string, unknown> {
  return {
    ...asset.meta,
    sourcePath: asset.path,
  }
}

async function runCss(
  assets: FontAsset[],
  options: CssPluginOptions,
  runtime: OptimizeRuntime,
): Promise<FontAsset | undefined> {
  const sourceAssets = assets.filter(asset => isCssSourceFormat(asset.format))
  const sources = sourceAssets.flatMap(asset =>
    cssSourceFromAsset(asset, options.base64 === true),
  )
  const firstAsset = sourceAssets[0]

  if (sources.length === 0 || firstAsset === undefined) {
    return undefined
  }

  const css = await runtime.generateFontFaceCss(
    sources,
    await cssOptionsForSources(options, firstAsset, runtime),
  )

  return {
    path: outputPathForAsset(
      firstAsset.path,
      cssTargetExtension(options.target),
      options,
    ),
    contents: Buffer.from(css),
    format: 'css',
    sourceFormat: firstAsset.sourceFormat,
    meta: {},
  }
}

function cssSourceFromAsset(
  asset: FontAsset,
  inline: boolean,
): CssFontSource[] {
  if (!isCssSourceFormat(asset.format)) {
    return []
  }

  const source: CssFontSource = {
    fileName: inline ? dataUrlForAsset(asset) : asset.path,
    format: asset.format,
  }
  const glyphs = cssGlyphsFromAsset(asset)

  if (glyphs.length > 0) {
    source.glyphs = glyphs
  }
  const unicodeRanges = asset.meta[CSS_UNICODE_RANGES_META_KEY]

  if (
    Array.isArray(unicodeRanges) &&
    unicodeRanges.every(range => typeof range === 'string')
  ) {
    source.unicodeRanges = unicodeRanges
  }

  return [source]
}

async function cssOptionsForSources(
  options: CssOptions,
  source: FontAsset,
  runtime: OptimizeRuntime,
): Promise<CssOptions> {
  const resolvedOptions = await cssOptionsWithResolvedFontFamily(
    options,
    source,
    runtime,
  )

  if (resolvedOptions.base64 !== true) {
    return resolvedOptions
  }

  const inlineOptions: CssOptions = {
    fontPath: '',
  }

  if (resolvedOptions.fontFamily !== undefined) {
    inlineOptions.fontFamily = resolvedOptions.fontFamily
  }
  if (resolvedOptions.glyph !== undefined) {
    inlineOptions.glyph = resolvedOptions.glyph
  }
  if (resolvedOptions.iconPrefix !== undefined) {
    inlineOptions.iconPrefix = resolvedOptions.iconPrefix
  }
  if (resolvedOptions.asFileName !== undefined) {
    inlineOptions.asFileName = resolvedOptions.asFileName
  }
  if (resolvedOptions.local !== undefined) {
    inlineOptions.local = resolvedOptions.local
  }
  if (resolvedOptions.fontDisplay !== undefined) {
    inlineOptions.fontDisplay = resolvedOptions.fontDisplay
  }

  return inlineOptions
}

async function cssOptionsWithResolvedFontFamily(
  options: CssOptions,
  source: FontAsset,
  runtime: OptimizeRuntime,
): Promise<CssOptions> {
  if (typeof options.fontFamily !== 'function') {
    return options
  }

  return {
    ...options,
    fontFamily: options.fontFamily(await runtime.inspect(source.contents)),
  }
}

function cssTargetExtension(target: CssOptions['target']): string {
  return target ?? 'css'
}

function outputPathForAsset(
  path: string,
  defaultExtension: string,
  options: OutputPathOptions,
): string {
  if (options.fileName !== undefined) {
    return options.fileName
  }

  return replaceExtension(path, options.ext ?? defaultExtension)
}

function cssGlyphsFromSubsetOptions(options: SubsetOptions): CssGlyph[] {
  const seen = new Set<number>()
  const glyphs: CssGlyph[] = []

  for (const character of options.text ?? '') {
    const unicode = character.codePointAt(0)

    if (unicode !== undefined && !seen.has(unicode)) {
      seen.add(unicode)
      glyphs.push({ unicode })
    }
  }

  for (const unicode of options.unicodes ?? []) {
    if (!seen.has(unicode)) {
      seen.add(unicode)
      glyphs.push({ unicode })
    }
  }

  return glyphs
}

function cssGlyphsFromSvgIcons(
  icons: SvgIcon[],
  startUnicode: number,
): CssGlyph[] {
  let nextUnicode = startUnicode
  const seen = new Set<number>()
  const glyphs: CssGlyph[] = []

  for (const icon of icons) {
    let unicode = icon.unicode

    if (unicode === undefined) {
      while (seen.has(nextUnicode)) {
        nextUnicode += 1
      }

      unicode = nextUnicode
      nextUnicode += 1
    }

    seen.add(unicode)
    glyphs.push({
      name: icon.name,
      unicode,
    })
  }

  return glyphs
}

function withCssGlyphs(
  meta: Record<string, unknown>,
  glyphs: CssGlyph[],
): Record<string, unknown> {
  if (glyphs.length === 0) {
    return meta
  }

  return {
    ...meta,
    [CSS_GLYPHS_META_KEY]: glyphs,
  }
}

function cssGlyphsFromAsset(asset: FontAsset): CssGlyph[] {
  const glyphs = asset.meta[CSS_GLYPHS_META_KEY]

  if (!Array.isArray(glyphs)) {
    return []
  }

  return glyphs.flatMap(glyph => {
    if (
      typeof glyph !== 'object' ||
      glyph === null ||
      !('unicode' in glyph) ||
      typeof glyph.unicode !== 'number'
    ) {
      return []
    }

    const cssGlyph: CssGlyph = {
      unicode: glyph.unicode,
    }

    if ('name' in glyph && typeof glyph.name === 'string') {
      cssGlyph.name = glyph.name
    }

    return [cssGlyph]
  })
}

function dataUrlForAsset(asset: FontAsset): string {
  if (!isCssSourceFormat(asset.format)) {
    throw new Error(`cannot inline ${asset.format} asset in CSS`)
  }

  const encoded = Buffer.from(asset.contents).toString('base64')

  return `data:${mimeTypeForFormat(asset.format)};base64,${encoded}`
}

function svgIconFromAsset(asset: FontAsset, index: number): SvgIcon {
  const icon: SvgIcon = {
    contents: Buffer.from(asset.contents).toString('utf8'),
    name: basenameWithoutExtension(asset.path) || `glyph-${index + 1}`,
  }
  const { unicode } = asset.meta

  if (typeof unicode === 'number') {
    icon.unicode = unicode
  }

  return icon
}

function mimeTypeForFormat(format: CssFontSource['format']): string {
  return MIME_TYPES_BY_FORMAT[format]
}

function detectFormat(input: Uint8Array): FontFormat {
  const bytes = Buffer.from(input)

  if (bytes.subarray(0, 4).equals(Buffer.from([0, 1, 0, 0]))) {
    return 'ttf'
  }

  if (bytes.subarray(0, 4).toString('ascii') === 'true') {
    return 'ttf'
  }

  if (bytes.subarray(0, 4).toString('ascii') === 'OTTO') {
    return 'otf'
  }

  if (bytes.subarray(0, 4).toString('ascii') === 'wOFF') {
    return 'woff'
  }

  if (bytes.subarray(0, 4).toString('ascii') === 'wOF2') {
    return 'woff2'
  }

  if (looksLikeEot(bytes)) {
    return 'eot'
  }

  if (looksLikeSvg(bytes)) {
    return 'svg'
  }

  try {
    return inspect(bytes).format
  } catch {
    return 'unknown'
  }
}

function extensionForFormat(format: FontFormat): string {
  return format === 'unknown' ? 'bin' : format
}

function looksLikeEot(bytes: Buffer): boolean {
  if (bytes.byteLength < 12) {
    return false
  }

  const version = bytes.subarray(8, 12)

  return (
    version.equals(Buffer.from([0x01, 0x00, 0x02, 0x00])) ||
    version.equals(Buffer.from([0x02, 0x00, 0x02, 0x00]))
  )
}

function looksLikeSvg(bytes: Buffer): boolean {
  const prefix = bytes.subarray(0, 512).toString('utf8').trimStart()

  return (
    prefix.startsWith('<svg') ||
    (prefix.startsWith('<?xml') && prefix.includes('<svg'))
  )
}

function isBuiltin(
  plugin: FontminPlugin,
  name: string,
): plugin is BuiltinPlugin {
  return (
    plugin.native !== undefined &&
    plugin.native.kind === 'builtin' &&
    plugin.native.name === name
  )
}

function isCssSourceFormat(
  format: AssetFormat,
): format is CssFontSource['format'] {
  return (
    format === 'ttf' ||
    format === 'woff' ||
    format === 'woff2' ||
    format === 'eot' ||
    format === 'svg'
  )
}

function replaceExtension(path: string, extension: string): string {
  const currentExtension = extname(path)

  if (currentExtension === '') {
    return `${path}.${extension}`
  }

  return `${path.slice(0, -currentExtension.length)}.${extension}`
}

function appendAssetSuffix(path: string, suffix: string): string {
  const currentExtension = extname(path)

  return currentExtension === ''
    ? `${path}-${suffix}`
    : `${path.slice(0, -currentExtension.length)}-${suffix}${currentExtension}`
}

function basenameWithoutExtension(path: string): string {
  const currentExtension = extname(path)

  return currentExtension === ''
    ? basename(path)
    : basename(path, currentExtension)
}

function sortPlugins(plugins: FontminPlugin[]): FontminPlugin[] {
  const pre: FontminPlugin[] = []
  const normal: FontminPlugin[] = []
  const post: FontminPlugin[] = []

  for (const plugin of plugins) {
    if (plugin.enforce === 'pre') {
      pre.push(plugin)
    } else if (plugin.enforce === 'post') {
      post.push(plugin)
    } else {
      normal.push(plugin)
    }
  }

  return [...pre, ...normal, ...post]
}

function woff2FallbacksFromPlugins(
  plugins: FontminPlugin[],
): NonNullable<Ttf2Woff2Options['fallback']>[] {
  return plugins.flatMap(plugin => {
    if (!isBuiltin(plugin, 'ttf2woff2')) {
      return []
    }

    const fallback = plugin.native.options['fallback']

    return isWoff2Fallback(fallback) ? [fallback] : []
  })
}

function woffOptions(options: Record<string, unknown>): WoffOptions {
  const nativeOptions: WoffOptions = {}

  if (typeof options['deflate'] === 'boolean') {
    nativeOptions.deflate = options['deflate']
  }
  if (typeof options['compressionLevel'] === 'number') {
    nativeOptions.compressionLevel = options['compressionLevel']
  }

  return nativeOptions
}

function woff2Options(options: Record<string, unknown>): Ttf2Woff2Options {
  const nativeOptions: Ttf2Woff2Options = {}

  if (typeof options['quality'] === 'number') {
    nativeOptions.quality = options['quality']
  }
  return nativeOptions
}

function isWoff2Fallback(
  value: unknown,
): value is NonNullable<Ttf2Woff2Options['fallback']> {
  return (
    value === 'native' || value === 'wasm' || value === 'js' || value === 'auto'
  )
}

function eotOptions(options: Record<string, unknown>): Ttf2EotOptions {
  const nativeOptions: Ttf2EotOptions = {}

  if (typeof options['version'] === 'number') {
    nativeOptions.version = options['version']
  }

  return nativeOptions
}

function otf2TtfOptions(options: Record<string, unknown>): Otf2TtfOptions {
  const nativeOptions: Otf2TtfOptions = {}

  if (typeof options['preserveHinting'] === 'boolean') {
    nativeOptions.preserveHinting = options['preserveHinting']
  }

  const variationCoordinates = options['variationCoordinates']
  if (
    variationCoordinates !== null &&
    typeof variationCoordinates === 'object' &&
    !Array.isArray(variationCoordinates)
  ) {
    nativeOptions.variationCoordinates = Object.fromEntries(
      Object.entries(variationCoordinates).filter(
        ([, value]) => typeof value === 'number' && Number.isFinite(value),
      ),
    )
  }

  return nativeOptions
}

function svgOptions(options: Record<string, unknown>): Ttf2SvgOptions {
  const nativeOptions: Ttf2SvgOptions = {}

  if (typeof options['fontFamily'] === 'string') {
    nativeOptions.fontFamily = options['fontFamily']
  }

  return nativeOptions
}

function svg2TtfOptions(options: Record<string, unknown>): Svg2TtfOptions {
  const nativeOptions: Svg2TtfOptions = {}

  if (typeof options['hinting'] === 'boolean') {
    nativeOptions.hinting = options['hinting']
  }
  if (typeof options['normalize'] === 'boolean') {
    nativeOptions.normalize = options['normalize']
  }

  return nativeOptions
}

function svgs2TtfOptions(options: Record<string, unknown>): Svgs2TtfOptions {
  const nativeOptions: Svgs2TtfOptions = {}

  if (typeof options['fontName'] === 'string') {
    nativeOptions.fontName = options['fontName']
  }
  if (typeof options['startUnicode'] === 'number') {
    nativeOptions.startUnicode = options['startUnicode']
  }
  if (typeof options['ascent'] === 'number') {
    nativeOptions.ascent = options['ascent']
  }
  if (typeof options['descent'] === 'number') {
    nativeOptions.descent = options['descent']
  }
  if (typeof options['normalize'] === 'boolean') {
    nativeOptions.normalize = options['normalize']
  }

  return nativeOptions
}

function cacheEntryDir(cacheDir: string, key: string): string {
  return join(cacheRoot(cacheDir), key.slice(0, 2), key.slice(2, 4), key)
}

function cacheIndexPath(cacheDir: string): string {
  return join(cacheRoot(cacheDir), 'index.json')
}

function cacheManifestPath(cacheDir: string, key: string): string {
  return join(cacheEntryDir(cacheDir, key), 'index.json')
}

function cacheRoot(cacheDir: string): string {
  return join(cacheDir, CACHE_SCHEMA_VERSION)
}

function cacheKeyForAssets(
  assets: FontAsset[],
  config: FontminConfig,
  plugins: FontminPlugin[],
  runtime: CacheRuntimeIdentity,
): string {
  return sha256(
    stableStringify({
      clean: config.clean,
      fontminVersion: FONTMIN_VERSION,
      inputs: assets.map(asset => ({
        format: asset.format,
        hash: sha256(asset.contents),
        path: asset.path,
        sourceFormat: asset.sourceFormat,
      })),
      plugins: plugins.map(plugin => ({
        enforce: plugin.enforce,
        name: plugin.name,
        native: plugin.native,
      })),
      preserveOriginal: config.preserveOriginal,
      runtime,
      schema: CACHE_SCHEMA_VERSION,
      subset: config.subset,
    }),
  )
}

async function cacheRuntimeIdentity(
  config: FontminConfig,
  plugins: FontminPlugin[],
  runtime: RuntimeSelector,
): Promise<CacheRuntimeIdentity> {
  const usesRuntime =
    config.subset !== undefined ||
    plugins.some(plugin => plugin.native?.kind === 'builtin')
  const resolved = usesRuntime ? await runtime.resolve() : undefined

  return {
    requested: runtime.requested,
    resolved: resolved?.kind ?? null,
  }
}

function isCacheablePipeline(plugins: FontminPlugin[]): boolean {
  return plugins.every(plugin => {
    return (
      plugin.native?.kind === 'builtin' &&
      plugin.buildStart === undefined &&
      plugin.transform === undefined &&
      plugin.generateBundle === undefined &&
      plugin.buildEnd === undefined
    )
  })
}

function normalizeCacheOptions(
  options: boolean | CacheOptions | undefined,
  cwd: string,
): NormalizedCacheOptions {
  if (options === undefined || options === false) {
    return {
      dir: resolve(cwd, DEFAULT_CACHE_DIR),
      enabled: false,
    }
  }

  if (options === true) {
    return {
      dir: resolve(cwd, DEFAULT_CACHE_DIR),
      enabled: true,
    }
  }

  return {
    dir: resolve(cwd, options.dir ?? DEFAULT_CACHE_DIR),
    enabled: options.enabled ?? true,
  }
}

function sha256(input: string | Uint8Array): string {
  return createHash('sha256').update(input).digest('hex')
}

function stableStringify(value: unknown): string {
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value)
  }

  if (Array.isArray(value)) {
    return `[${value.map(item => stableStringify(item)).join(',')}]`
  }

  const entries = Object.entries(value)
    .filter(([, entryValue]) => entryValue !== undefined)
    .sort(([left], [right]) => left.localeCompare(right))

  return `{${entries
    .map(([key, entryValue]) => {
      return `${JSON.stringify(key)}:${stableStringify(entryValue)}`
    })
    .join(',')}}`
}

async function writeAssets(outDir: string, assets: FontAsset[]): Promise<void> {
  for (const asset of assets) {
    const outputPath = join(outDir, asset.path)

    await mkdir(dirname(outputPath), { recursive: true })
    await writeFile(outputPath, asset.contents)
  }
}
