import {
  applyAssetTransform,
  flatMapAssets,
} from '../../../packages/fontmin/src/runtime-neutral/optimize-policy'
import type { CssOptions } from '../types'
import {
  generateFontFaceCss,
  subsetTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
  otfToTtf,
  svgFontToTtf,
  svgsToTtf,
} from './native'
import { normalizeDeliverySlices } from './plugins'
import type {
  BrowserPlugin,
  BrowserPluginContext,
  DeliverySlicesOptions,
  GlyphOptions,
  Otf2TtfPluginOptions,
  Svg2TtfPluginOptions,
  Svgs2TtfPluginOptions,
  Ttf2EotPluginOptions,
  Ttf2SvgPluginOptions,
  Ttf2Woff2PluginOptions,
  Ttf2WoffPluginOptions,
} from './plugins'

export interface BrowserAsset {
  contents: Uint8Array
  fileName: string
  format?: string
  unicodeRanges?: string[]
}

export interface BrowserOptimizeConfig {
  assets: BrowserAsset[]
  plugins?: BrowserPlugin[]
}

type FormattedBrowserAsset = BrowserAsset & { format: string }

export async function optimizeBrowser(
  config: BrowserOptimizeConfig,
): Promise<BrowserAsset[]> {
  let assets: FormattedBrowserAsset[] = config.assets.map(formatAsset)

  for (const plugin of config.plugins ?? []) {
    if (plugin.name === 'glyph') {
      const options = optionsOf<GlyphOptions>(plugin)
      assets = await flatMapAssets(assets, async asset => {
        if (asset.format !== 'ttf') {
          return [asset]
        }

        const subsetAsset = {
          ...asset,
          contents: await subsetTtf(asset.contents, options),
        }

        return options.clone === true ? [asset, subsetAsset] : [subsetAsset]
      })

      continue
    }

    if (plugin.name === 'unicodeSlices') {
      const slices = normalizeDeliverySlices(
        optionsOf<DeliverySlicesOptions>(plugin),
      )
      assets = await flatMapAssets(assets, async asset => {
        if (asset.format !== 'ttf') {
          return [asset]
        }

        return Promise.all(
          slices.map(async slice => ({
            ...asset,
            contents: await subsetTtf(asset.contents, {
              missingGlyphs: 'ignore',
              unicodeRanges: slice.unicodeRanges,
            }),
            fileName: appendFileNameSuffix(asset.fileName, slice.name),
            unicodeRanges: slice.unicodeRanges,
          })),
        )
      })

      continue
    }

    if (plugin.name === 'css') {
      const options = optionsOf<CssOptions>(plugin)
      const css = await generateFontFaceCss(
        assets
          .filter(asset =>
            ['eot', 'svg', 'ttf', 'woff', 'woff2'].includes(asset.format ?? ''),
          )
          .map(asset => {
            const source = {
              contents: asset.contents,
              fileName: asset.fileName,
              format: asset.format as 'eot' | 'svg' | 'ttf' | 'woff' | 'woff2',
            }

            return asset.unicodeRanges === undefined
              ? source
              : { ...source, unicodeRanges: asset.unicodeRanges }
          }),
        options,
      )
      const firstFont = assets.find(asset =>
        ['eot', 'svg', 'ttf', 'woff', 'woff2'].includes(asset.format ?? ''),
      )
      if (firstFont !== undefined) {
        assets.push({
          contents: new TextEncoder().encode(css),
          fileName: replaceExtension(
            firstFont.fileName,
            options.target ?? 'css',
          ),
          format: 'css',
        })
      }
      continue
    }

    if (plugin.name === 'svgs2ttf') {
      const icons = assets
        .filter(asset => asset.format === 'svg')
        .map(asset => ({
          contents: new TextDecoder().decode(asset.contents),
          name: asset.fileName.replace(/\.[^.]+$/u, ''),
        }))
      if (icons.length > 0) {
        const options = optionsOf<Svgs2TtfPluginOptions>(plugin)
        const fontName = options.fontName ?? 'iconfont'
        const ttfAsset: FormattedBrowserAsset = {
          contents: await svgsToTtf(icons, options),
          fileName: `${toKebabCase(fontName)}.ttf`,
          format: 'ttf',
        }

        assets =
          options.clone === true
            ? [...assets, ttfAsset]
            : [...assets.filter(asset => asset.format !== 'svg'), ttfAsset]
      }
      continue
    }

    if (plugin.transform !== undefined) {
      const emitted: FormattedBrowserAsset[] = []
      const context: BrowserPluginContext = {
        diagnostics: [],
        emitFile(asset) {
          emitted.push(formatAsset(asset))
        },
        warn(message) {
          context.diagnostics.push({
            level: 'warn',
            message: message instanceof Error ? message.message : message,
          })
        },
      }
      const transformed = await applyAssetTransform(
        assets,
        plugin.transform,
        context,
        formatAsset,
      )
      assets = [...transformed, ...emitted]
      continue
    }

    const clone = optionsOf<{ clone?: boolean }>(plugin).clone !== false
    const convertedAssets: FormattedBrowserAsset[] = []
    const additions: FormattedBrowserAsset[] = []

    for (const asset of assets) {
      const convertedAsset = await convert(asset, plugin)

      if (convertedAsset === undefined) {
        convertedAssets.push(asset)
      } else if (clone) {
        convertedAssets.push(asset)
        additions.push(convertedAsset)
      } else {
        convertedAssets.push(convertedAsset)
      }
    }

    assets = [...convertedAssets, ...additions]
  }

  return assets
}

async function convert(
  asset: FormattedBrowserAsset,
  plugin: BrowserPlugin,
): Promise<FormattedBrowserAsset | undefined> {
  if (plugin.name === 'ttf2woff' && asset.format === 'ttf') {
    return converted(
      asset,
      'woff',
      await ttfToWoff(asset.contents, optionsOf<Ttf2WoffPluginOptions>(plugin)),
    )
  }
  if (plugin.name === 'ttf2woff2' && asset.format === 'ttf') {
    return converted(
      asset,
      'woff2',
      await ttfToWoff2(
        asset.contents,
        optionsOf<Ttf2Woff2PluginOptions>(plugin),
      ),
    )
  }
  if (plugin.name === 'ttf2eot' && asset.format === 'ttf') {
    return converted(
      asset,
      'eot',
      await ttfToEot(asset.contents, optionsOf<Ttf2EotPluginOptions>(plugin)),
    )
  }
  if (plugin.name === 'ttf2svg' && asset.format === 'ttf') {
    return converted(
      asset,
      'svg',
      new TextEncoder().encode(
        await ttfToSvg(asset.contents, optionsOf<Ttf2SvgPluginOptions>(plugin)),
      ),
    )
  }
  if (plugin.name === 'otf2ttf' && asset.format === 'otf') {
    return converted(
      asset,
      'ttf',
      await otfToTtf(asset.contents, optionsOf<Otf2TtfPluginOptions>(plugin)),
    )
  }
  if (plugin.name === 'svg2ttf' && asset.format === 'svg') {
    return converted(
      asset,
      'ttf',
      await svgFontToTtf(
        new TextDecoder().decode(asset.contents),
        optionsOf<Svg2TtfPluginOptions>(plugin),
      ),
    )
  }

  return undefined
}

function optionsOf<Options extends object>(plugin: BrowserPlugin): Options {
  return (plugin.options ?? {}) as Options
}

function converted(
  asset: FormattedBrowserAsset,
  format: string,
  contents: Uint8Array,
): FormattedBrowserAsset {
  return {
    ...asset,
    contents,
    fileName: replaceExtension(asset.fileName, format),
    format,
  }
}

function formatAsset(asset: BrowserAsset): FormattedBrowserAsset {
  return { ...asset, format: asset.format ?? formatOf(asset.fileName) }
}

function formatOf(fileName: string): string {
  return fileName.split('.').pop()?.toLowerCase() ?? 'unknown'
}

function replaceExtension(fileName: string, extension: string): string {
  return `${fileName.replace(/\.[^.]+$/u, '')}.${extension}`
}

function appendFileNameSuffix(fileName: string, suffix: string): string {
  const extension =
    fileName.match(/(?<extension>\.[^.]+)$/u)?.groups?.['extension'] ?? ''
  const baseName =
    extension === '' ? fileName : fileName.slice(0, -extension.length)

  return `${baseName}-${suffix}${extension}`
}

function toKebabCase(value: string): string {
  return value
    .trim()
    .replaceAll(/(?<lower>[a-z])(?<upper>[A-Z])/gu, '$<lower>-$<upper>')
    .replaceAll(/\s+/gu, '-')
    .toLowerCase()
}
