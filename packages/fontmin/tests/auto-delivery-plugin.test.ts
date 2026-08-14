import { expect, it } from 'vitest'
import { autoDeliverySlices, optimize } from '../src/index'
import type { AutoDeliveryOptions, FontAsset } from '../src/index'
import { unicodeCodePointsFromSfnt } from '../src/runtime-neutral/sfnt-unicode'
import { cjkFixture } from './api-fixtures'

const autoDelivery = {
  frequencyText: 'AB中文',
  languages: ['en', 'zh-Hans'],
  maxSlices: 8,
  measureFormat: 'ttf' as const,
  targetBytes: 2_000,
  tolerance: 0,
} satisfies AutoDeliveryOptions

interface AutoDeliveryMetadata {
  estimatedBytes: number
  languages: string[]
  measureFormat: string
  targetBytes: number
  tolerance: number
}

it('creates measured language-aware slices through the public plugin', async () => {
  const assets = await optimize({
    cache: false,
    input: [cjkFixture],
    plugins: [autoDeliverySlices(autoDelivery)],
  })

  expect(assets.length).toBeGreaterThan(1)
  expect(assets.length).toBeLessThanOrEqual(autoDelivery.maxSlices)
  expect(assets.every(asset => asset.format === 'ttf')).toBe(true)
  expect(new Set(assets.map(asset => asset.path)).size).toBe(assets.length)

  const covered = new Set<number>()
  for (const asset of assets) {
    const metadata = requiredAutoDeliveryMetadata(asset)
    const unicodeRanges = requiredUnicodeRanges(asset)

    expect(asset.path).toMatch(/noto-sans-sc-compact-.+\.ttf$/u)
    expect(metadata).toMatchObject({
      estimatedBytes: asset.contents.byteLength,
      languages: ['en', 'zh-Hans'],
      measureFormat: 'ttf',
      targetBytes: 2_000,
      tolerance: 0,
    })
    expect(unicodeRanges.length).toBeGreaterThan(0)
    for (const codePoint of unicodeCodePointsFromSfnt(asset.contents)) {
      covered.add(codePoint)
    }
  }

  for (const character of 'AB中文') {
    const codePoint = character.codePointAt(0)
    expect(codePoint === undefined ? false : covered.has(codePoint)).toBe(true)
  }
})

it('runs top-level auto delivery before configured WOFF2 and CSS outputs', async () => {
  const assets = await optimize({
    autoDelivery,
    cache: false,
    css: { fontFamily: 'Automatic CJK', local: false },
    input: [cjkFixture],
    outputs: ['woff2', 'css'],
  })
  const fonts = assets.filter(asset => asset.format === 'woff2')
  const styles = assets.filter(asset => asset.format === 'css')

  expect(fonts.length).toBeGreaterThan(1)
  expect(styles).toHaveLength(1)
  expect(
    assets.every(asset => asset.format === 'woff2' || asset.format === 'css'),
  ).toBe(true)

  const css = new TextDecoder().decode(styles[0]!.contents)
  expect(css.match(/@font-face/gmu)).toHaveLength(fonts.length)
  expect(css.match(/unicode-range:/gmu)).toHaveLength(fonts.length)
  for (const font of fonts) {
    expect(css).toContain(font.path)
  }
})

function requiredAutoDeliveryMetadata(asset: FontAsset): AutoDeliveryMetadata {
  const metadata = asset.meta['autoDelivery']

  if (typeof metadata !== 'object' || metadata === null) {
    throw new TypeError(`missing auto delivery metadata for ${asset.path}`)
  }

  return metadata as AutoDeliveryMetadata
}

function requiredUnicodeRanges(asset: FontAsset): string[] {
  const ranges = asset.meta['cssUnicodeRanges']

  if (
    !Array.isArray(ranges) ||
    !ranges.every(range => typeof range === 'string')
  ) {
    throw new TypeError(`missing unicode ranges for ${asset.path}`)
  }

  return ranges
}
