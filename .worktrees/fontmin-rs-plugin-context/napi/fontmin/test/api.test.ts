import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { expect, it } from 'vitest'
import {
  eotToTtf,
  generateFontFaceCss,
  inspectFont,
  otfToTtf,
  subsetTtf,
  svgFontToTtf,
  svgsToTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
} from '../src-js/index.js'

const currentDir = import.meta.dirname
const fixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
)
const homeSvg =
  '<svg viewBox="0 0 1000 1000"><path d="M100 500 L500 100 L900 500 L900 900 L100 900 Z"/></svg>'
const userSvg =
  '<svg viewBox="0 0 1000 1000"><path d="M500 100 C620 100 700 180 700 300 C700 420 620 500 500 500 C380 500 300 420 300 300 C300 180 380 100 500 100 Z M250 900 Q500 650 750 900 Z"/></svg>'
const svgFont =
  '<svg xmlns="http://www.w3.org/2000/svg"><defs><font id="icons" horiz-adv-x="1000"><font-face font-family="SVG Icons" units-per-em="1000" ascent="850" descent="-150" /><glyph glyph-name="home" unicode="&#xE101;" horiz-adv-x="1000" d="M100 100 L900 100 L900 900 L100 900 Z" /></font></defs></svg>'

function otfFromTtf(input: Buffer): Buffer {
  const otf = Buffer.from(input)

  otf.write('OTTO', 0, 'ascii')

  return otf
}

it('subsets a TTF buffer through napi', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { text: 'Hello' })

  expect(Buffer.isBuffer(output)).toBe(true)
  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('inspects TTF metadata through napi', () => {
  const input = readFileSync(fixture)
  const info = inspectFont(input)

  expect(info).toMatchObject({
    format: 'ttf',
    size: input.byteLength,
    metadata: {
      familyName: 'Roboto',
      subfamilyName: 'Regular',
      fullName: 'Roboto Regular',
      postScriptName: 'Roboto-Regular',
      glyphCount: 3387,
      unitsPerEm: 2048,
      ascender: 2146,
      descender: -555,
    },
  })
  expect(info.metadata.tables).toContain('name')
})

it('inspects OTF metadata through napi', () => {
  const input = otfFromTtf(readFileSync(fixture))
  const info = inspectFont(input)

  expect(info).toMatchObject({
    format: 'otf',
    size: input.byteLength,
    metadata: {
      familyName: 'Roboto',
      subfamilyName: 'Regular',
      fullName: 'Roboto Regular',
      postScriptName: 'Roboto-Regular',
      glyphCount: 3387,
      unitsPerEm: 2048,
      ascender: 2146,
      descender: -555,
    },
  })
  expect(info.metadata.tables).toContain('name')
})

it('reports unsupported OTF to TTF conversion through napi', () => {
  const input = otfFromTtf(readFileSync(fixture))

  expect(() => otfToTtf(input)).toThrow('unsupported font format: otf to ttf')
})

it('converts a TTF buffer to WOFF through napi', () => {
  const input = readFileSync(fixture)
  const output = ttfToWoff(input)

  expect(Buffer.isBuffer(output)).toBe(true)
  expect(output.subarray(0, 4).toString('ascii')).toBe('wOFF')
  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('converts a TTF buffer to WOFF2 through napi', () => {
  const input = readFileSync(fixture)
  const output = ttfToWoff2(input)
  const declaredLength = output.readUInt32BE(8)

  expect(Buffer.isBuffer(output)).toBe(true)
  expect(output.subarray(0, 4).toString('ascii')).toBe('wOF2')
  expect(declaredLength).toBe(output.byteLength)
  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('converts a TTF buffer to EOT through napi', () => {
  const input = readFileSync(fixture)
  const output = ttfToEot(input)

  expect(Buffer.isBuffer(output)).toBe(true)
  expect(output.readUInt32LE(0)).toBe(output.byteLength)
  expect(output.readUInt32LE(4)).toBe(input.byteLength)
  expect(output.subarray(8, 12)).toEqual(Buffer.from([0x01, 0x00, 0x02, 0x00]))
  expect(output.subarray(34, 36)).toEqual(Buffer.from([0x4c, 0x50]))
  expect(output.subarray(output.byteLength - input.byteLength)).toEqual(input)
})

it('converts a TTF buffer to SVG through napi', () => {
  const input = readFileSync(fixture)
  const svg = ttfToSvg(input)

  expect(svg.startsWith('<svg')).toBe(true)
  expect(svg).toContain('<font ')
  expect(svg).toContain('font-family="Roboto"')
  expect(svg).toContain('unicode="A"')
  expect(svg).toContain('d="M')
})

it('combines SVG icons into a TTF buffer through napi', () => {
  const output = svgsToTtf(
    [
      { name: 'home', contents: homeSvg, unicode: 0xe101 },
      { name: 'user', contents: userSvg },
    ],
    {
      fontName: 'Icon Set',
      startUnicode: 0xe200,
      ascent: 850,
      descent: -150,
      normalize: true,
    },
  )
  const info = inspectFont(output)

  expect(Buffer.isBuffer(output)).toBe(true)
  expect(output.subarray(0, 4)).toEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Icon Set')
  expect(info.metadata.glyphCount).toBe(3)
  expect(info.metadata.unitsPerEm).toBe(1000)
})

it('converts an SVG font to a TTF buffer through napi', () => {
  const output = svgFontToTtf(svgFont, { normalize: true, hinting: false })
  const info = inspectFont(output)

  expect(output.subarray(0, 4)).toEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('SVG Icons')
  expect(info.metadata.glyphCount).toBe(2)
  expect(info.metadata.unitsPerEm).toBe(1000)
})

it('decodes an EOT buffer to TTF through napi', () => {
  const input = readFileSync(fixture)
  const eot = ttfToEot(input)
  const output = eotToTtf(eot)
  const info = inspectFont(eot)

  expect(output.subarray(0, 4)).toEqual(Buffer.from([0, 1, 0, 0]))
  expect(output.byteLength).toBe(input.byteLength)
  expect(info.format).toBe('eot')
  expect(info.metadata.fullName).toBe('Roboto Regular')
})

it('generates @font-face CSS through napi', () => {
  const css = generateFontFaceCss(
    [
      { fileName: 'roboto.woff', format: 'woff' },
      { fileName: 'roboto.woff2', format: 'woff2' },
    ],
    {
      fontFamily: 'Roboto',
      fontPath: './fonts',
      local: true,
      fontDisplay: 'swap',
    },
  )

  expect(css).toContain('@font-face')
  expect(css).toContain("font-family: 'Roboto';")
  expect(css).toContain("local('Roboto')")
  expect(css).toContain("url('./fonts/roboto.woff') format('woff')")
  expect(css).toContain("url('./fonts/roboto.woff2') format('woff2')")
})
