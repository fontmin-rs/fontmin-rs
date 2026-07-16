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
      assets = await Promise.all(
        assets.map(async asset =>
          asset.format === 'ttf'
            ? {
                ...asset,
                contents: await subsetTtf(
                  asset.contents,
                  optionsOf<GlyphOptions>(plugin),
                ),
              }
            : asset,
        ),
      )
      continue
    }

    if (plugin.name === 'unicodeSlices') {
      const slices = normalizeDeliverySlices(
        optionsOf<DeliverySlicesOptions>(plugin),
      )
      const slicedAssets: FormattedBrowserAsset[] = []

      for (const asset of assets) {
        if (asset.format !== 'ttf') {
          slicedAssets.push(asset)
          continue
        }

        for (const slice of slices) {
          slicedAssets.push({
            ...asset,
            contents: await subsetTtf(asset.contents, {
              missingGlyphs: 'ignore',
              unicodeRanges: slice.unicodeRanges,
            }),
            fileName: appendFileNameSuffix(asset.fileName, slice.name),
            unicodeRanges: slice.unicodeRanges,
          })
        }
      }

      assets = slicedAssets
      continue
    }

    if (plugin.name === 'css') {
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
        optionsOf<CssOptions>(plugin),
      )
      const firstFont = assets.find(asset => asset.format === 'ttf')
      if (firstFont !== undefined) {
        assets.push({
          contents: new TextEncoder().encode(css),
          fileName: replaceExtension(firstFont.fileName, 'css'),
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
        assets.push({
          contents: await svgsToTtf(icons, options),
          fileName: `${toKebabCase(fontName)}.ttf`,
          format: 'ttf',
        })
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
      const transformed: FormattedBrowserAsset[] = []
      for (const asset of assets) {
        const result = await plugin.transform(asset, context)
        if (result === null) {
          continue
        }
        if (result === undefined) {
          transformed.push(asset)
        } else {
          transformed.push(
            ...(Array.isArray(result) ? result : [result]).map(formatAsset),
          )
        }
      }
      assets = transformed.concat(emitted)
      continue
    }

    if (
      plugin.name === 'otf2ttf' &&
      optionsOf<Otf2TtfPluginOptions>(plugin).clone === false
    ) {
      assets = await Promise.all(
        assets.map(async asset => (await convert(asset, plugin)) ?? asset),
      )
      continue
    }

    const additions: FormattedBrowserAsset[] = []
    for (const asset of assets) {
      const converted = await convert(asset, plugin)
      if (converted !== undefined) {
        additions.push(converted)
      }
    }
    assets = assets.concat(additions)
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
  return { ...asset, format: formatOf(asset.fileName) }
}

function formatOf(fileName: string): string {
  return fileName.split('.').pop()?.toLowerCase() ?? 'unknown'
}

function replaceExtension(fileName: string, extension: string): string {
  return `${fileName.replace(/\.[^.]+$/u, '')}.${extension}`
}

function appendFileNameSuffix(fileName: string, suffix: string): string {
  const extension = fileName.match(/(\.[^.]+)$/u)?.[1] ?? ''
  const baseName =
    extension === '' ? fileName : fileName.slice(0, -extension.length)

  return `${baseName}-${suffix}${extension}`
}

function toKebabCase(value: string): string {
  return value
    .trim()
    .replaceAll(/([a-z])([A-Z])/gu, '$1-$2')
    .replaceAll(/\s+/gu, '-')
    .toLowerCase()
}
