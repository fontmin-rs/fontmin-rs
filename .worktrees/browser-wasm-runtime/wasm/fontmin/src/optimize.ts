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
import type { BrowserPlugin } from './plugins'
import type { BrowserPluginContext } from './plugins'

export interface BrowserAsset {
  contents: Uint8Array
  fileName: string
  format?: string
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
                contents: await subsetTtf(asset.contents, plugin.options),
              }
            : asset,
        ),
      )
      continue
    }

    if (plugin.name === 'css') {
      const css = await generateFontFaceCss(
        assets
          .filter(asset =>
            ['eot', 'svg', 'ttf', 'woff', 'woff2'].includes(asset.format ?? ''),
          )
          .map(asset => ({
            contents: asset.contents,
            fileName: asset.fileName,
            format: asset.format as 'eot' | 'svg' | 'ttf' | 'woff' | 'woff2',
          })),
        plugin.options,
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
        const fontName = String(plugin.options?.['fontName'] ?? 'iconfont')
        assets.push({
          contents: await svgsToTtf(icons, plugin.options),
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
        if (result === null) continue
        if (result === undefined) transformed.push(asset)
        else {
          transformed.push(
            ...(Array.isArray(result) ? result : [result]).map(formatAsset),
          )
        }
      }
      assets = transformed.concat(emitted)
      continue
    }

    const additions: FormattedBrowserAsset[] = []
    for (const asset of assets) {
      const converted = await convert(asset, plugin)
      if (converted !== undefined) additions.push(converted)
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
      await ttfToWoff(asset.contents, plugin.options),
    )
  }
  if (plugin.name === 'ttf2woff2' && asset.format === 'ttf') {
    return converted(
      asset,
      'woff2',
      await ttfToWoff2(asset.contents, plugin.options),
    )
  }
  if (plugin.name === 'ttf2eot' && asset.format === 'ttf') {
    return converted(
      asset,
      'eot',
      await ttfToEot(asset.contents, plugin.options),
    )
  }
  if (plugin.name === 'ttf2svg' && asset.format === 'ttf') {
    return converted(
      asset,
      'svg',
      new TextEncoder().encode(await ttfToSvg(asset.contents, plugin.options)),
    )
  }
  if (plugin.name === 'otf2ttf' && asset.format === 'otf') {
    return converted(
      asset,
      'ttf',
      await otfToTtf(asset.contents, plugin.options),
    )
  }
  if (plugin.name === 'svg2ttf' && asset.format === 'svg') {
    return converted(
      asset,
      'ttf',
      await svgFontToTtf(
        new TextDecoder().decode(asset.contents),
        plugin.options,
      ),
    )
  }

  return undefined
}

function converted(
  asset: FormattedBrowserAsset,
  format: string,
  contents: Uint8Array,
): FormattedBrowserAsset {
  return {
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

function toKebabCase(value: string): string {
  return value
    .trim()
    .replace(/([a-z])([A-Z])/gu, '$1-$2')
    .replace(/\s+/gu, '-')
    .toLowerCase()
}
