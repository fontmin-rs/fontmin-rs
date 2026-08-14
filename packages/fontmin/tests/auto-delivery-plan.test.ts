import { readFile } from 'node:fs/promises'
import { expect, it } from 'vitest'
import {
  detectDeliveryLanguages,
  planAutoDeliverySlices,
} from '../src/runtime-neutral/auto-delivery'
import { unicodeCodePointsFromSfnt } from '../src/runtime-neutral/sfnt-unicode'
import { fixture, variableTtfFixture } from './api-fixtures'

it('detects language presets while assigning contextual Han once', () => {
  expect(detectDeliveryLanguages('Hello Καλημέρα Привет')).toStrictEqual([
    'en',
    'el',
    'ru',
  ])
  expect(detectDeliveryLanguages('日本語かな')).toStrictEqual(['ja'])
  expect(detectDeliveryLanguages('中文')).toStrictEqual(['zh-Hans'])
  expect(detectDeliveryLanguages('한글漢字')).toStrictEqual(['ko'])
  expect(detectDeliveryLanguages('注音ㄅㄆ中文')).toStrictEqual(['zh-Hant'])
})

it('prioritizes frequent business text and splits by measured byte size', async () => {
  const supported = [...'ABCDEF中文字体测试分包'].map(
    character => character.codePointAt(0) ?? 0,
  )
  const plan = await planAutoDeliverySlices(
    supported,
    {
      frequencyText: '中中中A中文',
      languages: ['en', 'zh-Hans'],
      maxSlices: 8,
      targetBytes: 400,
      tolerance: 0.1,
    },
    async codePoints => 100 + codePoints.length * 100,
  )

  expect(plan.languages).toStrictEqual(['en', 'zh-Hans'])
  expect(plan.slices[0]).toMatchObject({
    codePoints: [0x00_41, 0x4e_2d, 0x65_87],
    name: 'priority',
    unicodeRanges: ['U+0041', 'U+4E2D', 'U+6587'],
  })
  expect(plan.slices).toHaveLength(5)
  expect(plan.slices.every(slice => slice.estimatedBytes <= 400)).toBe(true)
  expect(new Set(plan.slices.flatMap(slice => slice.codePoints)).size).toBe(
    new Set(supported).size,
  )
})

it('merges undersized neighboring chunks after byte-based splitting', async () => {
  const plan = await planAutoDeliverySlices(
    [...'ABCDEFGH'].map(character => character.codePointAt(0) ?? 0),
    { languages: ['en'], maxSlices: 8, targetBytes: 300, tolerance: 0.2 },
    async codePoints =>
      codePoints.length === 4 && codePoints[0] === 0x00_43
        ? 300
        : codePoints.length * 100,
  )

  expect(plan.slices.map(slice => slice.codePoints.length)).toStrictEqual([
    2, 4, 2,
  ])
  expect(plan.slices.map(slice => slice.name)).toStrictEqual([
    'latin-1',
    'latin-2',
    'latin-3',
  ])
})

it('reads BMP and supplementary cmap coverage from real fixtures', async () => {
  const [roboto, variable] = await Promise.all([
    readFile(fixture),
    readFile(variableTtfFixture),
  ])
  const robotoCodePoints = unicodeCodePointsFromSfnt(roboto)
  const variableCodePoints = unicodeCodePointsFromSfnt(variable)

  expect(robotoCodePoints).toContain(0x00_41)
  expect(robotoCodePoints).toContain(0x20_ac)
  expect(variableCodePoints).toContain(0x4e_2d)
  expect(variableCodePoints).toContain(0x65_87)
  expect(robotoCodePoints).toStrictEqual(
    [...robotoCodePoints].toSorted((left, right) => left - right),
  )
})

it('rejects invalid limits and unmatched language coverage', async () => {
  await expect(
    planAutoDeliverySlices(
      [0x00_41],
      { languages: ['en'], targetBytes: 0 },
      async () => 1,
    ),
  ).rejects.toThrow('targetBytes must be a positive integer')
  await expect(
    planAutoDeliverySlices([0x00_41], { languages: ['ja'] }, async () => 1),
  ).rejects.toThrow('matched no supported code points')
})
