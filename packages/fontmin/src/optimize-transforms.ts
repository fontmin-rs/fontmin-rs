import { basename, extname } from 'node:path'
import {
  builtinPluginDescriptor,
  createBuiltinPlugin,
  withInternalCacheKey,
} from './builtin-plugin'
import { inspect } from './native'
import type { OptimizeRuntime, RuntimeSelector } from './optimize-runtime'
import {
  applyAssetTransform,
  applyFontConversion,
  flatMapAssets,
  normalizeDeliverySlices,
} from './runtime-neutral/optimize-policy'
import type { FontConversion } from './runtime-neutral/optimize-policy'
import type {
  AssetFormat,
  ConfigOutput,
  CssFontSource,
  CssGlyph,
  CssOptions,
  FontAsset,
  FontFormat,
  FontminCompatCssInfo,
  FontminCompatFontFamily,
  FontminGlyphTransform,
  FontminTtfObject,
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

interface OutputPathOptions {
  ext?: string
  fileName?: string
}

type CssPluginOptions = CssOptions &
  OutputPathOptions & {
    fontminCompatAsFileName?: boolean
    fontminCompatFontFamily?: FontminCompatFontFamily
  }

type CompatSubsetOptions = SubsetOptions & {
  fontminCompatUse?: FontminGlyphTransform
}

const DEFAULT_SVG_ICON_START_UNICODE = 57_345
const CSS_GLYPHS_META_KEY = 'cssGlyphs'
const CSS_UNICODE_RANGES_META_KEY = 'cssUnicodeRanges'
const FONTMIN_COMPAT_TTF_META_KEY = 'fontminCompatTtfObject'

const MIME_TYPES_BY_FORMAT: Record<CssFontSource['format'], string> = {
  eot: 'application/vnd.ms-fontobject',
  svg: 'image/svg+xml',
  ttf: 'font/ttf',
  woff: 'font/woff',
  woff2: 'font/woff2',
}

export async function generateAssets(
  initialAssets: FontAsset[],
  plugins: FontminPlugin[],
  context: PluginContext,
  runtime: RuntimeSelector,
  emittedAssets: FontAsset[],
): Promise<FontAsset[]> {
  let assets = initialAssets

  for (const plugin of plugins) {
    const descriptor = builtinPluginDescriptor(plugin, 'css')

    if (descriptor === undefined) {
      await plugin.generateBundle?.(assets, context)
      assets = appendAssets(assets, emittedAssets.splice(0))
      continue
    }

    const cssAsset = await runCss(
      assets,
      descriptor.options as CssOptions,
      await runtime.resolve(),
    )
    if (cssAsset !== undefined) {
      assets = appendAssets(assets, [cssAsset])
    }
  }

  return assets
}

function appendAssets(
  assets: FontAsset[],
  additions: FontAsset[],
): FontAsset[] {
  return [...assets, ...additions]
}

export function pluginsFromConfig(config: FontminConfig): FontminPlugin[] {
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

  const ttfOutput = outputs.find(output => output.format === 'ttf')
  if (ttfOutput?.fileName !== undefined || ttfOutput?.ext !== undefined) {
    plugins.push(
      createBuiltinPlugin('outputPath', {
        format: 'ttf',
        ...outputPathOptionsRecord(ttfOutput),
      }),
    )
  }

  if (requestedOutputs.includes('css')) {
    const cssOutput = outputs.find(output => output.format === 'css')

    plugins.push(
      createBuiltinPlugin('css', {
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
  if (output.format === 'css') {
    return undefined
  }

  if (output.format === 'ttf') {
    return undefined
  }

  const options = {
    clone: output.clone ?? true,
    ...outputPathOptionsRecord(output),
  }

  if (output.format === 'eot') {
    return createBuiltinPlugin('ttf2eot', options)
  }
  if (output.format === 'svg') {
    return createBuiltinPlugin('ttf2svg', options)
  }
  if (output.format === 'woff') {
    return createBuiltinPlugin('ttf2woff', options)
  }

  return createBuiltinPlugin('ttf2woff2', options)
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

function outputFilterPlugin(
  formats: OutputFormat[],
  enforce?: FontminPlugin['enforce'],
): FontminPlugin {
  const plugin: FontminPlugin = withInternalCacheKey(
    {
      name: 'fontmin:output-filter',
      generateBundle(assets) {
        const retainedAssets = assets.filter(asset => {
          const format = outputFormatFromAsset(asset)

          return format !== undefined && formats.includes(format)
        })

        assets.splice(0, assets.length, ...retainedAssets)
      },
    },
    `output-filter:${enforce ?? 'normal'}:${formats.join(',')}`,
  )

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

export async function transformAssets(
  assets: FontAsset[],
  plugin: FontminPlugin,
  context: PluginContext,
  runtime: RuntimeSelector,
): Promise<FontAsset[]> {
  const glyphDescriptor = builtinPluginDescriptor(plugin, 'glyph')
  if (glyphDescriptor !== undefined) {
    return flatMapAssets(assets, async asset =>
      runGlyph(
        asset,
        glyphDescriptor.options as SubsetOptions,
        await runtime.resolve(),
      ),
    )
  }

  const sliceDescriptor = builtinPluginDescriptor(plugin, 'unicodeSlices')
  if (sliceDescriptor !== undefined) {
    return flatMapAssets(assets, async asset =>
      runUnicodeSlices(asset, sliceDescriptor.options, await runtime.resolve()),
    )
  }

  const normalizeDescriptor = builtinPluginDescriptor(plugin, 'normalizeToTtf')
  if (normalizeDescriptor !== undefined) {
    return flatMapAssets(assets, async asset =>
      runNormalizeToTtf(
        asset,
        normalizeDescriptor.options,
        await runtime.resolve(),
      ),
    )
  }

  const descriptor = builtinPluginDescriptor(plugin)
  if (descriptor !== undefined) {
    const convertedAssets = await applyFontConversion(
      assets,
      descriptor.name,
      descriptor.options['clone'] !== false,
      asset => asset.format,
      async (asset, conversion) =>
        convertBuiltinAsset(
          asset,
          conversion,
          descriptor.options,
          await runtime.resolve(),
        ),
    )

    if (convertedAssets !== undefined) {
      return convertedAssets
    }
  }

  const iconDescriptor = builtinPluginDescriptor(plugin, 'svgs2ttf')
  if (iconDescriptor !== undefined) {
    return runSvgs2Ttf(assets, iconDescriptor.options, await runtime.resolve())
  }

  const outputPathDescriptor = builtinPluginDescriptor(plugin, 'outputPath')
  if (outputPathDescriptor !== undefined) {
    const format = outputPathDescriptor.options['format']

    return assets.map(asset => {
      if (asset.format !== format) {
        return asset
      }

      return {
        ...asset,
        path: outputPathForAsset(
          asset.path,
          String(format),
          outputPathDescriptor.options,
        ),
      }
    })
  }

  if (builtinPluginDescriptor(plugin, 'css') !== undefined) {
    return assets
  }

  if (plugin.transform === undefined) {
    return assets
  }

  return applyAssetTransform(assets, plugin.transform, context, asset => asset)
}

export async function runGlyph(
  asset: FontAsset,
  options: CompatSubsetOptions,
  runtime: OptimizeRuntime,
): Promise<FontAsset[]> {
  if (asset.format !== 'ttf') {
    return [asset]
  }
  const meta = withCssGlyphs(asset.meta, cssGlyphsFromSubsetOptions(options))
  let contents: Uint8Array = Buffer.from(
    await runtime.subsetTtf(asset.contents, runtimeSubsetOptions(options)),
  )
  let legacyTtfObject: unknown

  if (options.fontminCompatUse !== undefined) {
    const { transformLegacyTtf } = await import('./fonteditor-compat')
    const transformed = transformLegacyTtf(
      contents,
      options.fontminCompatUse,
      options.preserveHinting ?? true,
    )

    contents = transformed.contents
    legacyTtfObject = transformed.ttfObject
  }

  const subsetAsset: FontAsset = {
    path: replaceExtension(asset.path, 'ttf'),
    contents,
    format: 'ttf',
    sourceFormat: asset.sourceFormat,
    meta: {
      ...meta,
      ...(legacyTtfObject === undefined
        ? {}
        : { [FONTMIN_COMPAT_TTF_META_KEY]: legacyTtfObject }),
    },
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
    normalizeDeliverySlices(options['slices']).map(async slice => ({
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

async function runNormalizeToTtf(
  asset: FontAsset,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset[]> {
  if (asset.format === 'ttf') {
    return [asset]
  }

  let contents: Uint8Array

  if (asset.format === 'otf') {
    contents = await runtime.otfToTtf(asset.contents, otf2TtfOptions(options))
  } else if (asset.format === 'woff') {
    contents = await runtime.woffToTtf(asset.contents)
  } else if (asset.format === 'woff2') {
    contents = await runtime.woff2ToTtf(asset.contents)
  } else if (asset.format === 'eot') {
    contents = await runtime.eotToTtf(asset.contents)
  } else {
    throw new Error(`fontmin-rs cannot normalize ${asset.format} input to TTF`)
  }

  return [
    {
      path: replaceExtension(asset.path, 'ttf'),
      contents: Buffer.from(contents),
      format: 'ttf',
      sourceFormat: asset.sourceFormat,
      meta: convertedMeta(asset),
    },
  ]
}

function runtimeSubsetOptions(options: CompatSubsetOptions): SubsetOptions {
  const {
    clone: _clone,
    fontminCompatUse: _fontminCompatUse,
    ...runtimeOptions
  } = options

  return Object.fromEntries(
    Object.entries(runtimeOptions).filter(([, value]) => value !== undefined),
  ) as SubsetOptions
}

async function convertTtfToWoff(
  asset: FontAsset,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset | undefined> {
  if (asset.format !== 'ttf') {
    return undefined
  }

  return {
    path: outputPathForAsset(asset.path, 'woff', options),
    contents: Buffer.from(
      await runtime.ttfToWoff(asset.contents, woffOptions(options)),
    ),
    format: 'woff',
    sourceFormat: asset.sourceFormat,
    meta: convertedMeta(asset),
  }
}

async function convertTtfToWoff2(
  asset: FontAsset,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset | undefined> {
  if (asset.format !== 'ttf') {
    return undefined
  }

  return {
    path: outputPathForAsset(asset.path, 'woff2', options),
    contents: Buffer.from(
      await runtime.ttfToWoff2(asset.contents, woff2Options(options)),
    ),
    format: 'woff2',
    sourceFormat: asset.sourceFormat,
    meta: convertedMeta(asset),
  }
}

async function convertTtfToEot(
  asset: FontAsset,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset | undefined> {
  if (asset.format !== 'ttf') {
    return undefined
  }

  return {
    path: outputPathForAsset(asset.path, 'eot', options),
    contents: Buffer.from(
      await runtime.ttfToEot(asset.contents, eotOptions(options)),
    ),
    format: 'eot',
    sourceFormat: asset.sourceFormat,
    meta: convertedMeta(asset),
  }
}

async function convertTtfToSvg(
  asset: FontAsset,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset | undefined> {
  if (asset.format !== 'ttf') {
    return undefined
  }

  return {
    path: outputPathForAsset(asset.path, 'svg', options),
    contents: Buffer.from(
      await runtime.ttfToSvg(asset.contents, svgOptions(options)),
    ),
    format: 'svg',
    sourceFormat: asset.sourceFormat,
    meta: convertedMeta(asset),
  }
}

async function convertOtfToTtf(
  asset: FontAsset,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset | undefined> {
  if (asset.format !== 'otf') {
    return undefined
  }

  return {
    path: outputPathForAsset(asset.path, 'ttf', options),
    contents: Buffer.from(
      await runtime.otfToTtf(asset.contents, otf2TtfOptions(options)),
    ),
    format: 'ttf',
    sourceFormat: asset.sourceFormat,
    meta: convertedMeta(asset),
  }
}

async function convertSvgToTtf(
  asset: FontAsset,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset | undefined> {
  if (asset.format !== 'svg') {
    return undefined
  }

  return {
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
}

async function convertBuiltinAsset(
  asset: FontAsset,
  conversion: FontConversion,
  options: Record<string, unknown>,
  runtime: OptimizeRuntime,
): Promise<FontAsset> {
  let convertedAsset: FontAsset | undefined

  if (conversion.name === 'otf2ttf') {
    convertedAsset = await convertOtfToTtf(asset, options, runtime)
  } else if (conversion.name === 'svg2ttf') {
    convertedAsset = await convertSvgToTtf(asset, options, runtime)
  } else if (conversion.name === 'ttf2eot') {
    convertedAsset = await convertTtfToEot(asset, options, runtime)
  } else if (conversion.name === 'ttf2svg') {
    convertedAsset = await convertTtfToSvg(asset, options, runtime)
  } else if (conversion.name === 'ttf2woff') {
    convertedAsset = await convertTtfToWoff(asset, options, runtime)
  } else {
    convertedAsset = await convertTtfToWoff2(asset, options, runtime)
  }

  if (convertedAsset === undefined) {
    throw new Error(
      `fontmin-rs conversion ${conversion.name} received ${asset.format}`,
    )
  }

  return convertedAsset
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
    typeof options['fontName'] === 'string' ? options['fontName'] : 'iconfont'
  const icons = svgAssets.map((asset, index) => svgIconFromAsset(asset, index))
  const cssGlyphs = cssGlyphsFromSvgIcons(
    icons,
    typeof options['startUnicode'] === 'number'
      ? options['startUnicode']
      : DEFAULT_SVG_ICON_START_UNICODE,
  )
  const ttfAsset: FontAsset = {
    path:
      typeof options['fileName'] === 'string'
        ? options['fileName']
        : `${fontName}.ttf`,
    contents: Buffer.from(
      await runtime.svgsToTtf(icons, svgs2TtfOptions(options)),
    ),
    format: 'ttf',
    sourceFormat: firstSvg.sourceFormat,
    meta: {
      ...firstSvg.meta,
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
    meta: { ...firstAsset.meta },
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

  return {
    ...resolvedOptions,
    base64: false,
    fontPath: '',
  }
}

async function cssOptionsWithResolvedFontFamily(
  options: CssPluginOptions,
  source: FontAsset,
  runtime: OptimizeRuntime,
): Promise<CssOptions> {
  const publicOptions = publicCssOptions(options)
  const fontFile = basenameWithoutExtension(source.path)
  const compatFontFamily = options.fontminCompatFontFamily
  let resolvedCompatFamily: string | undefined

  if (compatFontFamily !== undefined) {
    const ttfObject = compatTtfObject(source)
    const info: FontminCompatCssInfo = {
      ...publicOptions,
      base64: publicOptions.base64 ?? '',
      fontFile,
      fontPath: publicOptions.fontPath ?? '',
      glyph: publicOptions.glyph ?? false,
      iconPrefix: publicOptions.iconPrefix ?? 'icon',
      local: publicOptions.local ?? false,
    }

    resolvedCompatFamily =
      compatFontFamily(structuredClone(info), ttfObject) ||
      ttfObject.name.fontFamily ||
      fontFile
  }

  if (options.fontminCompatAsFileName === true) {
    return {
      ...publicOptions,
      asFileName: false,
      fontFamily: fontFile,
    }
  }

  if (resolvedCompatFamily !== undefined) {
    return {
      ...publicOptions,
      fontFamily: resolvedCompatFamily,
    }
  }

  if (typeof publicOptions.fontFamily !== 'function') {
    return publicOptions
  }

  return {
    ...publicOptions,
    fontFamily: publicOptions.fontFamily(
      await runtime.inspect(source.contents),
    ),
  }
}

function publicCssOptions(options: CssPluginOptions): CssOptions {
  const {
    fontminCompatAsFileName: _fontminCompatAsFileName,
    fontminCompatFontFamily: _fontminCompatFontFamily,
    ...publicOptions
  } = options

  return publicOptions
}

function compatTtfObject(source: FontAsset): FontminTtfObject {
  const value = source.meta[FONTMIN_COMPAT_TTF_META_KEY]

  if (
    typeof value === 'object' &&
    value !== null &&
    'name' in value &&
    typeof value.name === 'object' &&
    value.name !== null
  ) {
    return value as FontminTtfObject
  }

  return { name: {} } as FontminTtfObject
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

export function detectFormat(input: Uint8Array): FontFormat {
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

export function extensionForFormat(format: FontFormat): string {
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
  const normalizedExtension = normalizeExtension(extension)
  const currentExtension = extname(path)

  if (currentExtension === '') {
    return `${path}.${normalizedExtension}`
  }

  return `${path.slice(0, -currentExtension.length)}.${normalizedExtension}`
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

export function sortPlugins(plugins: FontminPlugin[]): FontminPlugin[] {
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

export function woff2FallbacksFromPlugins(
  plugins: FontminPlugin[],
): NonNullable<Ttf2Woff2Options['fallback']>[] {
  return plugins.flatMap(plugin => {
    const descriptor = builtinPluginDescriptor(plugin, 'ttf2woff2')

    if (descriptor === undefined) {
      return []
    }

    const fallback = descriptor.options['fallback']

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

function normalizeExtension(extension: string): string {
  const normalized = extension.replace(/^\.+/u, '')

  if (
    normalized.length === 0 ||
    normalized === '..' ||
    normalized.includes('/') ||
    normalized.includes('\\')
  ) {
    throw new Error(`output extension must be a file extension: ${extension}`)
  }

  return normalized
}
