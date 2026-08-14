import { createHash } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import { expect, it } from 'vitest'
import { optimize, webDelivery } from '../src/index'
import type { FontAsset, WebDeliveryManifest } from '../src/index'
import { fixture } from './api-fixtures'

it('generates a subset delivery bundle and preserves the full input fallback', async () => {
  const input = await readFile(fixture)
  const assets = await optimize({
    input: [fixture],
    outputs: ['woff2'],
    subset: { text: 'Hello' },
    webDelivery: {
      basePath: '/assets/fonts',
      fontFamily: 'Roboto Web',
    },
  })
  const paths = assets.map(asset => asset.path).toSorted()
  const subset = requiredAsset(assets, 'roboto-regular.woff2')
  const fallback = requiredAsset(assets, 'roboto-regular-fallback.ttf')
  const css = assetText(assets, 'fontmin-delivery.css')
  const preload = assetText(assets, 'fontmin-preload.html')
  const manifest = JSON.parse(
    assetText(assets, 'fontmin-manifest.json'),
  ) as WebDeliveryManifest

  expect(paths).toStrictEqual([
    'fontmin-delivery.css',
    'fontmin-manifest.json',
    'fontmin-preload.html',
    'roboto-regular-fallback.ttf',
    'roboto-regular.woff2',
  ])
  expect(subset.contents.byteLength).toBeLessThan(input.byteLength)
  expect(fallback.contents).toStrictEqual(input)
  expect(css).toContain('font-family: "Roboto Web Subset";')
  expect(css).toContain('font-family: "Roboto Web Fallback";')
  expect(css).toContain('url("/assets/fonts/roboto-regular.woff2")')
  expect(css).toContain('url("/assets/fonts/roboto-regular-fallback.ttf")')
  expect(css).toContain('unicode-range: U+0048, U+0065, U+006C, U+006F;')
  expect(css).toContain(
    'font-family: "Roboto Web Subset", "Roboto Web Fallback";',
  )
  expect(preload).toContain('href="/assets/fonts/roboto-regular.woff2"')
  expect(preload).toContain('type="font/woff2"')
  expect(preload).not.toContain('fallback')
  expect(manifest.schemaVersion).toBe(1)
  expect(manifest.fontFamily).toBe('Roboto Web')
  expect(manifest.summary).toStrictEqual({
    codePointCount: 4,
    fallbackBytes: input.byteLength,
    requestCount: 1,
    sourceBytes: input.byteLength,
    subsetBytes: subset.contents.byteLength,
    subsetCount: 1,
  })
  expect(manifest.sources).toHaveLength(1)
  expect(manifest.sources[0]).toMatchObject({
    fallback: {
      format: 'ttf',
      path: 'roboto-regular-fallback.ttf',
      preload: false,
      sha256: sha256(input),
      size: input.byteLength,
      unicodeRanges: [],
    },
    sourceFormat: 'ttf',
    sourcePath: 'roboto-regular.ttf',
    subsets: [
      {
        format: 'woff2',
        path: 'roboto-regular.woff2',
        preload: true,
        sha256: sha256(subset.contents),
        size: subset.contents.byteLength,
        unicodeRanges: ['U+0048', 'U+0065', 'U+006C', 'U+006F'],
      },
    ],
  })
  for (const asset of assets) {
    expect(asset.meta).not.toHaveProperty('fontminWebDeliveryOriginalAsset')
    expect(asset.meta).not.toHaveProperty('fontminWebDeliverySourceId')
  }
})

it('hashes delivery fonts and emits an inspectable preview page', async () => {
  const input = await readFile(fixture)
  const assets = await optimize({
    input: [fixture],
    outputs: ['woff2', 'css'],
    subset: { text: 'Hello' },
    webDelivery: {
      basePath: '/assets/fonts',
      fontFamily: 'Roboto Hashed',
      hashFileNames: true,
      hashLength: 12,
      testHtmlFile: 'fontmin-preview.html',
      testText: 'Hello preview',
    },
  })
  const manifest = JSON.parse(
    assetText(assets, 'fontmin-manifest.json'),
  ) as WebDeliveryManifest
  const subset = manifest.sources[0]?.subsets[0]
  const fallback = manifest.sources[0]?.fallback

  expect(subset?.path).toMatch(/^roboto-regular\.[0-9a-f]{12}\.woff2$/u)
  expect(fallback?.path).toMatch(
    /^roboto-regular-fallback\.[0-9a-f]{12}\.ttf$/u,
  )
  expect(subset?.path).toContain(subset?.sha256.slice(0, 12))
  expect(fallback?.path).toContain(fallback?.sha256.slice(0, 12))
  expect(manifest.testHtml).toBe('fontmin-preview.html')
  expect(manifest.summary).toMatchObject({
    codePointCount: 4,
    fallbackBytes: input.byteLength,
    requestCount: 1,
    sourceBytes: input.byteLength,
    subsetCount: 1,
  })

  const deliveryCss = assetText(assets, 'fontmin-delivery.css')
  const pipelineCss = new TextDecoder().decode(
    assets.find(
      asset => asset.format === 'css' && asset.path !== 'fontmin-delivery.css',
    )?.contents ?? new Uint8Array(),
  )
  const preview = assetText(assets, 'fontmin-preview.html')
  expect(deliveryCss).toContain(`/assets/fonts/${subset?.path}`)
  expect(deliveryCss).toContain(`/assets/fonts/${fallback?.path}`)
  expect(pipelineCss).toContain(subset?.path)
  expect(pipelineCss).not.toContain("url('./roboto-regular.woff2')")
  expect(preview).toContain('/assets/fonts/fontmin-delivery.css')
  expect(preview).toContain('Hello preview')
  expect(preview).toContain(subset?.path)
  expect(assets.some(asset => asset.path === 'roboto-regular.woff2')).toBe(
    false,
  )
  expect(assets.every(asset => !('fontminWebDeliveryStem' in asset.meta))).toBe(
    true,
  )
})

it('supports a delivery plugin without a fallback or preload', async () => {
  const assets = await optimize({
    input: [fixture],
    plugins: [
      ...webDelivery({
        fallback: false,
        fontFamily: 'Roboto Static',
        preload: false,
      }),
    ],
  })
  const css = assetText(assets, 'fontmin-delivery.css')
  const preload = assetText(assets, 'fontmin-preload.html')
  const manifest = JSON.parse(
    assetText(assets, 'fontmin-manifest.json'),
  ) as WebDeliveryManifest

  expect(assets.some(asset => asset.path.includes('-fallback.'))).toBe(false)
  expect(css).toContain('font-family: "Roboto Static Subset";')
  expect(css).not.toContain('Roboto Static Fallback')
  expect(preload).toBe('\n')
  expect(manifest.sources[0]?.fallback).toBeUndefined()
  expect(manifest.sources[0]?.subsets[0]?.preload).toBe(false)
})

it('rejects empty delivery names', () => {
  expect(() => webDelivery({ cssFile: ' ', fontFamily: 'Roboto' })).toThrow(
    'webDelivery cssFile must not be empty',
  )
  expect(() => webDelivery({ fontFamily: ' ' })).toThrow(
    'webDelivery fontFamily must not be empty',
  )
  expect(() => webDelivery({ fontFamily: 'Roboto', hashLength: 5 })).toThrow(
    'webDelivery hashLength must be an integer in [6, 64]',
  )
})

function assetText(assets: FontAsset[], path: string): string {
  return new TextDecoder().decode(requiredAsset(assets, path).contents)
}

function requiredAsset(assets: FontAsset[], path: string): FontAsset {
  const asset = assets.find(candidate => candidate.path === path)

  if (asset === undefined) {
    throw new Error(`missing asset: ${path}`)
  }

  return asset
}

function sha256(contents: Uint8Array): string {
  return createHash('sha256').update(contents).digest('hex')
}
