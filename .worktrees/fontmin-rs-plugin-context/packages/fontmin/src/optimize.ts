import { createHash } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { basename, dirname, extname, join, resolve } from 'node:path'
import { glob } from 'tinyglobby'
import {
  generateFontFaceCss,
  inspect,
  otfToTtf,
  subsetTtf,
  svgFontToTtf,
  svgsToTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
} from './native'
import type {
  AssetFormat,
  CacheOptions,
  ConfigOutput,
  CssFontSource,
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
  version: string
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
const FONTMIN_VERSION = '0.0.0'
const DEFAULT_CACHE_DIR = 'node_modules/.cache/fontmin-rs'

const MIME_TYPES_BY_FORMAT: Record<CssFontSource['format'], string> = {
  eot: 'application/vnd.ms-fontobject',
  svg: 'image/svg+xml',
  ttf: 'font/ttf',
  woff: 'font/woff',
  woff2: 'font/woff2',
}

export async function optimize(config: FontminConfig): Promise<FontAsset[]> {
  const cwd = config.cwd === undefined ? process.cwd() : config.cwd
  config = await resolveConfigTextFile(config, cwd)
  const plugins = sortPlugins(
    await resolvePluginTextFiles(pluginsFromConfig(config), cwd),
  )
  const cacheOptions = normalizeCacheOptions(config.cache, cwd)
  const emittedAssets: FontAsset[] = []
  const context = createPluginContext(cwd, emittedAssets)

  for (const plugin of plugins) {
    if (plugin.buildStart !== undefined) {
      await plugin.buildStart(context)
    }
  }

  let assets = await loadInputAssets(config.input ?? [], cwd)
  const cacheKey =
    cacheOptions.enabled && isCacheablePipeline(plugins)
      ? cacheKeyForAssets(assets, config, plugins)
      : undefined
  const cachedAssets =
    cacheKey === undefined
      ? undefined
      : await readCachedAssets(cacheOptions.dir, cacheKey)

  if (cachedAssets !== undefined) {
    assets = cachedAssets
  } else {
    const subset = config.subset

    if (subset !== undefined) {
      assets = assets.map(asset => runGlyph(asset, subset))
    }

    for (const plugin of plugins) {
      assets = await transformAssets(assets, plugin, context)
      assets = [...assets, ...emittedAssets.splice(0)]
    }

    for (const plugin of plugins) {
      if (isBuiltin(plugin, 'css')) {
        const cssAsset = runCss(assets, plugin.native.options as CssOptions)
        if (cssAsset !== undefined) {
          assets = assets.concat(cssAsset)
        }
      } else {
        if (plugin.generateBundle !== undefined) {
          await plugin.generateBundle(assets, context)
        }
        assets = [...assets, ...emittedAssets.splice(0)]
      }
    }

    if (cacheKey !== undefined) {
      await writeCachedAssets(cacheOptions.dir, cacheKey, assets)
    }
  }

  if (config.outDir !== undefined) {
    await writeAssets(resolve(cwd, config.outDir), assets)
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

  return {
    ...options,
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
    plugins.push(builtinPlugin('css', cssOptionsRecord(config.css)))
    plugins.push(outputFilterPlugin(requestedOutputs, 'post'))
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

  return record
}

async function readCachedAssets(
  cacheDir: string,
  key: string,
): Promise<FontAsset[] | undefined> {
  let manifest: CacheManifest

  try {
    manifest = JSON.parse(
      await readFile(cacheManifestPath(cacheDir, key), 'utf8'),
    ) as CacheManifest
  } catch {
    return undefined
  }

  if (manifest.version !== CACHE_SCHEMA_VERSION || manifest.key !== key) {
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
  return /[*?[\]{}]/.test(path)
}

async function transformAssets(
  assets: FontAsset[],
  plugin: FontminPlugin,
  context: PluginContext,
): Promise<FontAsset[]> {
  if (isBuiltin(plugin, 'glyph')) {
    return assets.map(asset => runGlyph(asset, plugin.native.options))
  }

  if (isBuiltin(plugin, 'otf2ttf')) {
    return assets.flatMap(asset => runOtf2Ttf(asset, plugin.native.options))
  }

  if (isBuiltin(plugin, 'ttf2woff')) {
    return assets.flatMap(asset => runTtf2Woff(asset, plugin.native.options))
  }

  if (isBuiltin(plugin, 'ttf2woff2')) {
    return assets.flatMap(asset => runTtf2Woff2(asset, plugin.native.options))
  }

  if (isBuiltin(plugin, 'ttf2eot')) {
    return assets.flatMap(asset => runTtf2Eot(asset, plugin.native.options))
  }

  if (isBuiltin(plugin, 'ttf2svg')) {
    return assets.flatMap(asset => runTtf2Svg(asset, plugin.native.options))
  }

  if (isBuiltin(plugin, 'svg2ttf')) {
    return assets.flatMap(asset => runSvg2Ttf(asset, plugin.native.options))
  }

  if (isBuiltin(plugin, 'svgs2ttf')) {
    return runSvgs2Ttf(assets, plugin.native.options)
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

function runGlyph(asset: FontAsset, options: SubsetOptions): FontAsset {
  if (asset.format !== 'ttf') {
    return asset
  }

  return {
    path: replaceExtension(asset.path, 'ttf'),
    contents: subsetTtf(asset.contents, options),
    format: 'ttf',
    sourceFormat: asset.sourceFormat,
    meta: asset.meta,
  }
}

function runTtf2Woff(
  asset: FontAsset,
  options: Record<string, unknown>,
): FontAsset[] {
  if (asset.format !== 'ttf') {
    return [asset]
  }

  const woffAsset: FontAsset = {
    path: replaceExtension(asset.path, 'woff'),
    contents: ttfToWoff(asset.contents, woffOptions(options)),
    format: 'woff',
    sourceFormat: asset.sourceFormat,
    meta: { sourcePath: asset.path },
  }

  return options['clone'] === false ? [woffAsset] : [asset, woffAsset]
}

function runTtf2Woff2(
  asset: FontAsset,
  options: Record<string, unknown>,
): FontAsset[] {
  if (asset.format !== 'ttf') {
    return [asset]
  }

  const woff2Asset: FontAsset = {
    path: replaceExtension(asset.path, 'woff2'),
    contents: ttfToWoff2(asset.contents, woff2Options(options)),
    format: 'woff2',
    sourceFormat: asset.sourceFormat,
    meta: { sourcePath: asset.path },
  }

  return options['clone'] === false ? [woff2Asset] : [asset, woff2Asset]
}

function runTtf2Eot(
  asset: FontAsset,
  options: Record<string, unknown>,
): FontAsset[] {
  if (asset.format !== 'ttf') {
    return [asset]
  }

  const eotAsset: FontAsset = {
    path: replaceExtension(asset.path, 'eot'),
    contents: ttfToEot(asset.contents, eotOptions(options)),
    format: 'eot',
    sourceFormat: asset.sourceFormat,
    meta: { sourcePath: asset.path },
  }

  return options['clone'] === false ? [eotAsset] : [asset, eotAsset]
}

function runTtf2Svg(
  asset: FontAsset,
  options: Record<string, unknown>,
): FontAsset[] {
  if (asset.format !== 'ttf') {
    return [asset]
  }

  const svgAsset: FontAsset = {
    path: replaceExtension(asset.path, 'svg'),
    contents: Buffer.from(ttfToSvg(asset.contents, svgOptions(options))),
    format: 'svg',
    sourceFormat: asset.sourceFormat,
    meta: { sourcePath: asset.path },
  }

  return options['clone'] === false ? [svgAsset] : [asset, svgAsset]
}

function runOtf2Ttf(
  asset: FontAsset,
  options: Record<string, unknown>,
): FontAsset[] {
  if (asset.format !== 'otf') {
    return [asset]
  }

  const ttfAsset: FontAsset = {
    path: replaceExtension(asset.path, 'ttf'),
    contents: otfToTtf(asset.contents, otf2TtfOptions(options)),
    format: 'ttf',
    sourceFormat: asset.sourceFormat,
    meta: { sourcePath: asset.path },
  }

  return options['clone'] === false ? [ttfAsset] : [asset, ttfAsset]
}

function runSvg2Ttf(
  asset: FontAsset,
  options: Record<string, unknown>,
): FontAsset[] {
  if (asset.format !== 'svg') {
    return [asset]
  }

  const ttfAsset: FontAsset = {
    path: replaceExtension(asset.path, 'ttf'),
    contents: svgFontToTtf(
      Buffer.from(asset.contents).toString('utf8'),
      svg2TtfOptions(options),
    ),
    format: 'ttf',
    sourceFormat: asset.sourceFormat,
    meta: { sourcePath: asset.path },
  }

  return options['clone'] === false ? [ttfAsset] : [asset, ttfAsset]
}

function runSvgs2Ttf(
  assets: FontAsset[],
  options: Record<string, unknown>,
): FontAsset[] {
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
  const ttfAsset: FontAsset = {
    path: `${fontName}.ttf`,
    contents: svgsToTtf(
      svgAssets.map((asset, index) => svgIconFromAsset(asset, index)),
      svgs2TtfOptions(options),
    ),
    format: 'ttf',
    sourceFormat: firstSvg.sourceFormat,
    meta: {
      sourcePaths: svgAssets.map(asset => asset.path),
    },
  }

  return options['clone'] === true
    ? [...assets, ttfAsset]
    : [...nonSvgAssets, ttfAsset]
}

function runCss(
  assets: FontAsset[],
  options: CssOptions,
): FontAsset | undefined {
  const sources = assets.flatMap(asset =>
    cssSourceFromAsset(asset, options.base64 === true),
  )
  const firstAsset = assets[0]

  if (sources.length === 0 || firstAsset === undefined) {
    return undefined
  }

  const css = generateFontFaceCss(sources, cssOptionsForSources(options))

  return {
    path: replaceExtension(firstAsset.path, 'css'),
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

  return [
    {
      fileName: inline ? dataUrlForAsset(asset) : asset.path,
      format: asset.format,
    },
  ]
}

function cssOptionsForSources(options: CssOptions): CssOptions {
  if (options.base64 !== true) {
    return options
  }

  const inlineOptions: CssOptions = {
    fontPath: '',
  }

  if (options.fontFamily !== undefined) {
    inlineOptions.fontFamily = options.fontFamily
  }
  if (options.local !== undefined) {
    inlineOptions.local = options.local
  }
  if (options.fontDisplay !== undefined) {
    inlineOptions.fontDisplay = options.fontDisplay
  }

  return inlineOptions
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
  const unicode = asset.meta['unicode']

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
      schema: CACHE_SCHEMA_VERSION,
      subset: config.subset,
    }),
  )
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
