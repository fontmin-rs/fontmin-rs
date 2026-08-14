import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { expect, it } from 'vitest'
import {
  analyzeCoverage,
  eotToTtf,
  extractCollectionFace,
  generateFontFaceCss,
  inspectCapabilities,
  inspectCollection,
  inspectFont,
  instantiateFont,
  otfToTtf,
  reduceVariationSpace,
  subsetTtf,
  subsetTtfWithReport,
  svgFontToTtf,
  svgsToTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
  validateWoff2,
  woff2ToTtf,
} from '../src-js/index.js'

const currentDir = import.meta.dirname
const fixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
)
const variableTtfFixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/noto-sans-sc-variable-compact.ttf',
)
const multiAxisVariableTtfFixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/estedad-variable.ttf',
)
const cffFixture = resolve(
  currentDir,
  '../../../fixtures/fonts/otf/source-sans-3-regular.otf',
)
const cff2Fixture = resolve(
  currentDir,
  '../../../fixtures/fonts/otf/source-serif-4-variable-roman.otf',
)
const homeSvg =
  '<svg viewBox="0 0 1000 1000"><path d="M100 500 L500 100 L900 500 L900 900 L100 900 Z"/></svg>'
const userSvg =
  '<svg viewBox="0 0 1000 1000"><path d="M500 100 C620 100 700 180 700 300 C700 420 620 500 500 500 C380 500 300 420 300 300 C300 180 380 100 500 100 Z M250 900 Q500 650 750 900 Z"/></svg>'
const advancedSvg =
  '<svg viewBox="0 0 1000 1000"><path d="M100 500 C200 100 300 100 400 500 S600 900 700 500 Q800 100 900 500 T100 500 A200 150 20 0 1 100 500 Z"/></svg>'
const svgFont =
  '<svg xmlns="http://www.w3.org/2000/svg"><defs><font id="icons" horiz-adv-x="1000"><font-face font-family="SVG Icons" units-per-em="1000" ascent="850" descent="-150" /><glyph glyph-name="home" unicode="&#xE101;" horiz-adv-x="1000" d="M100 100 L900 100 L900 900 L100 900 Z" /></font></defs></svg>'

function otfFromTtf(input: Buffer): Buffer {
  const otf = Buffer.from(input)

  otf.write('OTTO', 0, 'ascii')

  return otf
}

function fontCollection(fonts: Buffer[]): Buffer {
  const headerSize = 12 + fonts.length * 4
  const header = Buffer.alloc(headerSize)
  const parts: Buffer[] = [header]
  header.write('ttcf', 0, 'ascii')
  header.writeUInt32BE(65_536, 4)
  header.writeUInt32BE(fonts.length, 8)
  let offset = headerSize

  for (const [index, font] of fonts.entries()) {
    const padding = (4 - (offset % 4)) % 4
    if (padding > 0) {
      parts.push(Buffer.alloc(padding))
      offset += padding
    }
    header.writeUInt32BE(offset, 12 + index * 4)
    const face = Buffer.from(font)
    const tableCount = face.readUInt16BE(4)
    for (let tableIndex = 0; tableIndex < tableCount; tableIndex += 1) {
      const recordOffset = 12 + tableIndex * 16 + 8
      face.writeUInt32BE(offset + face.readUInt32BE(recordOffset), recordOffset)
    }
    parts.push(face)
    offset += face.length
  }

  return Buffer.concat(parts)
}

it('inspects and extracts TTC/OTC faces through napi', () => {
  const collection = fontCollection([
    readFileSync(fixture),
    readFileSync(cffFixture),
  ])
  const info = inspectCollection(collection)

  expect(info.faces).toHaveLength(2)
  expect(info.faces[0]?.metadata.familyName).toBe('Roboto')
  expect(info.faces[1]?.format).toBe('otf')
  expect(
    inspectFont(extractCollectionFace(collection, 1)).metadata.familyName,
  ).toBe('Source Sans 3')
})

it('reports color font capabilities through napi', () => {
  expect(inspectCapabilities(readFileSync(fixture)).color).toStrictEqual({
    isColorFont: false,
    technologies: [],
  })
})

function postVersion(input: Uint8Array): number {
  const view = new DataView(input.buffer, input.byteOffset, input.byteLength)
  const decoder = new TextDecoder()
  const tableCount = view.getUint16(4)

  for (let index = 0; index < tableCount; index += 1) {
    const recordOffset = 12 + index * 16
    if (
      decoder.decode(input.subarray(recordOffset, recordOffset + 4)) === 'post'
    ) {
      return view.getUint32(view.getUint32(recordOffset + 8))
    }
  }

  throw new Error('post table is missing')
}

function hasCmapRecord(
  input: Uint8Array,
  platformId: number,
  encodingId: number,
): boolean {
  const view = new DataView(input.buffer, input.byteOffset, input.byteLength)
  const decoder = new TextDecoder()
  const tableCount = view.getUint16(4)

  for (let index = 0; index < tableCount; index += 1) {
    const recordOffset = 12 + index * 16
    if (
      decoder.decode(input.subarray(recordOffset, recordOffset + 4)) !== 'cmap'
    ) {
      continue
    }
    const cmapOffset = view.getUint32(recordOffset + 8)
    const cmapRecordCount = view.getUint16(cmapOffset + 2)
    return Array.from({ length: cmapRecordCount }, (_, cmapIndex) => {
      const cmapRecordOffset = cmapOffset + 4 + cmapIndex * 8
      return (
        view.getUint16(cmapRecordOffset) === platformId &&
        view.getUint16(cmapRecordOffset + 2) === encodingId
      )
    }).some(Boolean)
  }

  return false
}

it('subsets a TTF buffer through napi', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { text: 'Hello' })

  expect(Buffer.isBuffer(output)).toBe(true)
  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('converts smooth SVG paths, arcs, and supplementary codepoints through napi', () => {
  const output = svgsToTtf([{ name: 'rocket', contents: advancedSvg }], {
    startUnicode: 0x1_f6_80,
  })
  const coverage = analyzeCoverage(output, { text: '🚀' })

  expect(ttfToSvg(output)).toContain('🚀')
  expect(coverage.supported).toStrictEqual([0x1_f6_80])
  expect(coverage.missing).toStrictEqual([])
})

it('subsets a TTF buffer by original glyph ID through napi', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { gids: [1] })

  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('returns subset mappings through napi', () => {
  const input = readFileSync(fixture)
  const result = subsetTtfWithReport(input, {
    gids: [1, 65_535],
    glyphNames: ['A', 'does.not.exist'],
    layoutFeatures: ['liga'],
    layoutLanguages: ['default'],
    layoutScripts: ['latn'],
    nameIds: [1],
    nameLanguages: [0x0409],
    dropTables: ['GPOS'],
    passThroughTables: ['gasp'],
    missingGlyphs: 'warn',
    retainGids: true,
    retainGlyphNames: true,
    retainLegacyCmap: true,
    retainSymbolCmap: true,
    text: 'A',
  })

  expect(Buffer.isBuffer(result.data)).toBe(true)
  expect(result.report.tablesRetained).not.toContain('GPOS')
  expect(result.report.tablesRetained).toContain('gasp')
  expect(postVersion(result.data)).toBe(0x0002_0000)
  expect(hasCmapRecord(result.data, 1, 0)).toBe(true)
  expect(result.report).toMatchObject({
    missingGids: [65_535],
    missingGlyphNames: ['does.not.exist'],
    originalSize: input.byteLength,
    requestedGids: [1, 65_535],
    requestedGlyphNames: ['A', 'does.not.exist'],
    subsetSize: result.data.byteLength,
    supportedGids: [1],
    supportedGlyphNames: ['A'],
  })
  expect(result.report.glyphNameToOldGid).toContainEqual({
    glyphName: 'A',
    oldGid: 38,
  })
  expect(result.report.oldToNew).toContainEqual({ newGid: 1, oldGid: 1 })
  expect(result.report.oldToNew).toContainEqual({ newGid: 38, oldGid: 38 })
  expect(result.report.unicodeToOldGid).toContainEqual({
    oldGid: 38,
    unicode: 0x41,
  })
  expect(result.report.newToOld).toHaveLength(result.report.glyphsRetained)
  expect(result.report.newToOld[2]).toBeNull()
})

it('analyzes coverage and enforces strict missing glyphs through napi', () => {
  const input = readFileSync(fixture)
  const report = analyzeCoverage(input, { text: 'A𠮷' })

  expect(report).toStrictEqual({
    coveragePercent: 50,
    missing: [134_071],
    requested: [0x41, 134_071],
    supported: [0x41],
  })
  expect(() =>
    subsetTtf(input, { missingGlyphs: 'error', text: 'A𠮷' }),
  ).toThrow('U+20BB7')
})

it('subsets a TTF buffer from Unicode ranges through napi', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { unicodeRanges: ['U+0041-0042'] })

  expect(output.byteLength).toBeLessThan(input.byteLength)
  expect(() => subsetTtf(input, { unicodeRanges: ['U+4??'] })).toThrow(
    'invalid Unicode range',
  )
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

it('instantiates a glyf variable font through napi', () => {
  const output = instantiateFont(readFileSync(variableTtfFixture), {
    variationCoordinates: { wght: 900 },
  })
  const info = inspectFont(output)

  expect(Buffer.isBuffer(output)).toBe(true)
  expect(info).toMatchObject({
    format: 'ttf',
    metadata: { glyphCount: 5 },
  })
  expect(info.metadata.tables).not.toContain('fvar')
  expect(info.metadata.tables).not.toContain('gvar')
  expect(() =>
    instantiateFont(readFileSync(variableTtfFixture), {
      variationCoordinates: { wght: 901 },
    }),
  ).toThrow('outside [100, 900]')
})

it('instantiates a CFF2 variable font through the unified napi API', () => {
  const output = instantiateFont(readFileSync(cff2Fixture), {
    variationCoordinates: { opsz: 14, wght: 700 },
  })
  const info = inspectFont(output)

  expect(info.format).toBe('ttf')
  expect(info.metadata.tables).toContain('glyf')
  expect(info.metadata.tables).not.toContain('CFF2')
  expect(info.metadata.tables).not.toContain('fvar')
})

it('reduces a variable design space through napi', () => {
  const output = reduceVariationSpace(
    readFileSync(multiAxisVariableTtfFixture),
    {
      pins: { wdth: 150 },
      ranges: { wght: { min: 300, max: 700, default: 500 } },
    },
  )
  const info = inspectFont(output)

  expect(Buffer.isBuffer(output)).toBe(true)
  expect(info.format).toBe('ttf')
  expect(info.metadata.tables).toContain('fvar')
  expect(info.metadata.tables).toContain('gvar')
})

it('converts glyf-backed OTF to TTF through napi', () => {
  const input = otfFromTtf(readFileSync(fixture))
  const output = otfToTtf(input)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(output.subarray(4)).toStrictEqual(input.subarray(4))
})

it('converts a real static CFF OTF to TTF through napi', () => {
  const output = otfToTtf(readFileSync(cffFixture))
  const info = inspectFont(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Source Sans 3')
  expect(info.metadata.tables).not.toContain('CFF ')
  expect(info.metadata.tables).toContain('glyf')
  expect(info.metadata.tables).toContain('GSUB')
  expect(info.metadata.tables).toContain('GPOS')
})

it('instantiates CFF2 coordinates through napi', () => {
  const output = otfToTtf(readFileSync(cff2Fixture), {
    variationCoordinates: { wght: 700, opsz: 14 },
  })
  const info = inspectFont(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Source Serif 4 Variable')
  expect(info.metadata.tables).toContain('glyf')
  expect(info.metadata.tables).not.toContain('CFF2')
  expect(info.metadata.tables).not.toContain('fvar')
  expect(info.metadata.tables).not.toContain('HVAR')
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

it('inspects WOFF2 table metadata through napi', () => {
  const input = readFileSync(fixture)
  const woff2 = ttfToWoff2(input)
  const info = inspectFont(woff2)

  expect(info.format).toBe('woff2')
  expect(info.size).toBe(woff2.byteLength)
  expect(info.metadata.familyName).toBe('Roboto')
  expect(info.metadata.fullName).toBe('Roboto Regular')
  expect(info.metadata.glyphCount).toBe(3387)
  expect(info.metadata.unitsPerEm).toBe(2048)
  expect(info.metadata.tables).toContain('cmap')
  expect(info.metadata.tables).toContain('name')
})

it('decodes WOFF2 to TTF through napi', () => {
  const input = readFileSync(fixture)
  const woff2 = ttfToWoff2(input)
  const output = woff2ToTtf(woff2)
  const info = inspectFont(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Roboto')
  expect(info.metadata.glyphCount).toBe(3387)
})

it('validates WOFF2 data through napi', () => {
  const input = readFileSync(fixture)
  const woff2 = ttfToWoff2(input)

  expect(() => validateWoff2(woff2)).not.toThrow()
  expect(() => validateWoff2(Buffer.from('not woff2'))).toThrow(
    'expected WOFF2 data',
  )
})

it('converts a TTF buffer to EOT through napi', () => {
  const input = readFileSync(fixture)
  const output = ttfToEot(input)

  expect(Buffer.isBuffer(output)).toBe(true)
  expect(output.readUInt32LE(0)).toBe(output.byteLength)
  expect(output.readUInt32LE(4)).toBe(input.byteLength)
  expect(output.subarray(8, 12)).toStrictEqual(
    Buffer.from([0x01, 0x00, 0x02, 0x00]),
  )
  expect(output.subarray(34, 36)).toStrictEqual(Buffer.from([0x4c, 0x50]))
  expect(output.subarray(output.byteLength - input.byteLength)).toStrictEqual(
    input,
  )
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
      { name: 'home', contents: homeSvg, unicode: 0xe1_01 },
      { name: 'user', contents: userSvg },
    ],
    {
      fontName: 'Icon Set',
      startUnicode: 0xe2_00,
      ascent: 850,
      descent: -150,
      normalize: true,
    },
  )
  const info = inspectFont(output)

  expect(Buffer.isBuffer(output)).toBe(true)
  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Icon Set')
  expect(info.metadata.glyphCount).toBe(3)
  expect(info.metadata.unitsPerEm).toBe(1000)
})

it('converts an SVG font to a TTF buffer through napi', () => {
  const output = svgFontToTtf(svgFont, { normalize: true, hinting: false })
  const info = inspectFont(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
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

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
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

it('inlines @font-face CSS sources through napi', () => {
  const css = generateFontFaceCss(
    [
      {
        contents: Buffer.from('woff-bytes'),
        fileName: 'roboto.woff',
        format: 'woff',
      },
    ],
    {
      base64: true,
      fontFamily: 'Roboto',
      local: false,
    },
  )

  expect(css).toContain(
    "url('data:font/woff;base64,d29mZi1ieXRlcw==') format('woff')",
  )
  expect(css).not.toContain('roboto.woff')
})

it('emits validated unicode ranges through napi', () => {
  const css = generateFontFaceCss(
    [{ fileName: 'roboto.woff2', format: 'woff2' }],
    { unicodeRanges: ['U+0020-007E'] },
  )

  expect(css).toContain('unicode-range: U+0020-007E;')
  expect(() =>
    generateFontFaceCss([{ fileName: 'roboto.woff2', format: 'woff2' }], {
      unicodeRanges: ['U+4??'],
    }),
  ).toThrow('invalid Unicode range')
})
