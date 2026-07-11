import { readFile } from 'node:fs/promises'
import { beforeAll, expect, it } from 'vitest'
import {
  eotToTtf,
  generateFontFaceCss,
  initWasm,
  inspect,
  otfToTtf,
  svgFontToTtf,
  svgsToTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
  woff2ToTtf,
  woffToTtf,
} from '../src/index'

const fixture = new URL(
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
  import.meta.url,
)
const wasm = new URL(
  '../src/generated/fontmin_wasm_core_bg.wasm',
  import.meta.url,
)
const otfFixture = new URL(
  '../../../fixtures/fonts/otf/source-sans-3-regular.otf',
  import.meta.url,
)

beforeAll(async () => {
  await initWasm(await readFile(wasm))
})

it('converts and inspects fonts after WASM initialization', async () => {
  const woff2 = await ttfToWoff2(await readFile(fixture))
  const info = await inspect(woff2)

  expect(new TextDecoder().decode(woff2.subarray(0, 4))).toBe('wOF2')
  expect(info.format).toBe('woff2')
  expect(info.metadata.familyName).toBe('Roboto')
})

it('runs every supported conversion without a native binding', async () => {
  const ttf = await readFile(fixture)
  const woff = await ttfToWoff(ttf)
  const woff2 = await ttfToWoff2(ttf)
  const eot = await ttfToEot(ttf)
  const svg = await ttfToSvg(ttf)
  const ttfFromSvg = await svgFontToTtf(
    '<svg><defs><font horiz-adv-x="1000"><font-face font-family="Icons" units-per-em="1000" /><glyph unicode="&#xE001;" d="M0 0 L100 0 L100 100 Z" /></font></defs></svg>',
  )
  const iconFont = await svgsToTtf([
    {
      contents:
        '<svg viewBox="0 0 1000 1000"><path d="M0 0 L1000 0 L1000 1000 Z" /></svg>',
      name: 'triangle',
      unicode: 0xe0_01,
    },
  ])
  const ttfFromOtf = await otfToTtf(await readFile(otfFixture))

  expect(new TextDecoder().decode(woff.subarray(0, 4))).toBe('wOFF')
  expect(new TextDecoder().decode(woff2.subarray(0, 4))).toBe('wOF2')
  await expect(woffToTtf(woff)).resolves.toBeInstanceOf(Uint8Array)
  await expect(woff2ToTtf(woff2)).resolves.toBeInstanceOf(Uint8Array)
  await expect(eotToTtf(eot)).resolves.toBeInstanceOf(Uint8Array)
  expect(svg).toContain('<svg')
  expect(ttfFromSvg).toBeInstanceOf(Uint8Array)
  expect(iconFont).toBeInstanceOf(Uint8Array)
  expect(ttfFromOtf).toBeInstanceOf(Uint8Array)
})

it('embeds in-memory font contents when generating Base64 CSS', async () => {
  const css = await generateFontFaceCss(
    [
      {
        contents: await readFile(fixture),
        fileName: 'roboto.ttf',
        format: 'ttf',
      },
    ],
    {
      asFileName: false,
      base64: true,
      fontDisplay: 'swap',
      fontFamily: 'Roboto Embedded',
      fontPath: './',
      glyph: false,
      iconPrefix: 'icon',
      local: false,
      target: 'css',
    },
  )

  expect(css).toContain('data:font/ttf;base64,')
  expect(css).toContain("font-family: 'Roboto Embedded'")
})

it('emits unicode ranges through the WASM API', async () => {
  const css = await generateFontFaceCss(
    [{ fileName: 'roboto.woff2', format: 'woff2' }],
    { unicodeRanges: ['U+0020-007E'] },
  )

  expect(css).toContain('unicode-range: U+0020-007E;')
})

it('rejects invalid unicode ranges through the WASM API', async () => {
  await expect(
    generateFontFaceCss([{ fileName: 'roboto.woff2', format: 'woff2' }], {
      unicodeRanges: ['U+4??'],
    }),
  ).rejects.toThrow('invalid Unicode range: U+4??')
})
