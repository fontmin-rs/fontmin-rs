import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { inflateSync } from 'node:zlib'
import { expect, it, vi } from 'vitest'
import {
  FontminDiagnosticError,
  analyzeCoverage,
  createTtfSubsetPlan,
  eotToTtf,
  extractCollectionFace,
  inspect,
  inspectCapabilities,
  inspectCollection,
  instantiateFont,
  otfToTtf,
  reduceVariationSpace,
  subsetTtf,
  subsetTtfWithPlan,
  subsetTtfWithReport,
  svgFontToTtf,
  svgsToTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
  ttfToWoff2Async,
  validateWoff2,
  woff2ToTtf,
  woffToTtf,
} from '../src/index'
import {
  colrFont,
  fixture,
  fontCollection,
  cffFixture,
  cff2Fixture,
  homeSvg,
  userSvg,
  svgFont,
  otfFromTtf,
  hasCmapRecord,
  multiAxisVariableTtfFixture,
  variableTtfFixture,
} from './api-fixtures'

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

it('inspects and extracts TTC/OTC faces through the public package api', () => {
  const collection = fontCollection([
    readFileSync(fixture),
    readFileSync(cffFixture),
  ])
  const info = inspectCollection(collection)

  expect(info.faces).toHaveLength(2)
  expect(info.faces[0]?.metadata.familyName).toBe('Roboto')
  expect(info.faces[1]?.format).toBe('otf')
  expect(
    inspect(extractCollectionFace(collection, 1)).metadata.familyName,
  ).toBe('Source Sans 3')
})

it('reports structured color font subset capabilities', () => {
  const report = inspectCapabilities(colrFont(readFileSync(fixture), 1))

  expect(report.color).toStrictEqual({
    isColorFont: true,
    subsetSupport: 'passthrough',
    technologies: [
      {
        detail:
          'COLR v1 paint graphs are retained verbatim; use retained GIDs for safe output',
        subsetSupport: 'passthrough',
        tables: ['COLR', 'CPAL'],
        technology: 'colr-cpal',
        version: 1,
      },
    ],
  })
})

it('subsets through the public package api', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { text: 'Hello' })

  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('creates and reuses subset plans through the public package api', () => {
  const input = readFileSync(fixture)
  const options = { gids: [2], text: 'Hello' }
  const plan = createTtfSubsetPlan(input, options)
  const planned = subsetTtfWithPlan(input, plan)
  const direct = subsetTtfWithReport(input, options)

  expect(plan.sourceSha256).toHaveLength(64)
  expect(plan.planSha256).toHaveLength(64)
  expect(plan.options.keepLayout).toBe('conservative')
  expect(planned).toStrictEqual(direct)
})

it('subsets by original glyph ID through the public package api', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { gids: [1] })

  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('returns subset mappings through the public package api', () => {
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

it('reports requested, supported, and missing code points', () => {
  const report = analyzeCoverage(readFileSync(fixture), { text: 'A𠮷' })

  expect(report).toStrictEqual({
    coveragePercent: 50,
    missing: [134_071],
    requested: [0x41, 134_071],
    supported: [0x41],
  })
})

it('returns stable diagnostic codes for malformed native input', () => {
  let diagnostic: unknown

  try {
    inspect(Buffer.from('not-a-font'))
  } catch (error) {
    diagnostic = error
  }

  expect(diagnostic).toBeInstanceOf(FontminDiagnosticError)
  expect(diagnostic).toMatchObject({
    code: 'fontmin::invalid_font',
    message: 'invalid font data: unknown font format',
    name: 'FontminDiagnosticError',
  })
})

it('warns by default and supports strict missing glyph handling', () => {
  const input = readFileSync(fixture)
  const warning = vi.spyOn(process, 'emitWarning').mockImplementation(() => {})

  try {
    expect(subsetTtf(input, { text: 'A𠮷' }).byteLength).toBeLessThan(
      input.byteLength,
    )
    expect(warning).toHaveBeenCalledWith(expect.stringContaining('U+20BB7'), {
      code: 'FONTMIN_MISSING_GLYPHS',
    })
    expect(() =>
      subsetTtf(input, { missingGlyphs: 'error', text: 'A𠮷' }),
    ).toThrow('U+20BB7')

    warning.mockClear()
    expect(() =>
      subsetTtf(input, { missingGlyphs: 'ignore', text: 'A𠮷' }),
    ).not.toThrow()
    expect(warning).not.toHaveBeenCalled()
  } finally {
    warning.mockRestore()
  }
})

it('subsets from Unicode ranges through the public package api', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { unicodeRanges: ['U+0041-0042'] })

  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('subsets with text loaded from textFile through the public package api', () => {
  const dir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-'))
  const textFile = resolve(dir, 'glyphs.txt')
  const input = readFileSync(fixture)

  try {
    writeFileSync(textFile, 'Hello')

    const output = subsetTtf(input, { textFile })
    const expected = subsetTtf(input, { text: 'Hello' })

    expect(output).toStrictEqual(expected)
  } finally {
    rmSync(dir, { force: true, recursive: true })
  }
})

it('keeps original font data when trim is disabled through the public package api', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { text: 'Hello', trim: false })

  expect(output.byteLength).toBe(input.byteLength)
  expect(Buffer.compare(output, input)).toBe(0)
})

it('inspects TTF metadata through the public package api', () => {
  const input = readFileSync(fixture)
  const info = inspect(input)

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

it('inspects OTF metadata through the public package api', () => {
  const input = otfFromTtf(readFileSync(fixture))
  const info = inspect(input)

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

it('instantiates glyf and CFF2 variable fonts through the public package api', () => {
  const glyfOutput = instantiateFont(readFileSync(variableTtfFixture), {
    variationCoordinates: { wght: 900 },
  })
  const cff2Output = instantiateFont(readFileSync(cff2Fixture), {
    variationCoordinates: { opsz: 14, wght: 700 },
  })

  for (const output of [glyfOutput, cff2Output]) {
    const info = inspect(output)

    expect(Buffer.isBuffer(output)).toBe(true)
    expect(info.format).toBe('ttf')
    expect(info.metadata.tables).toContain('glyf')
    expect(info.metadata.tables).not.toContain('fvar')
  }
  expect(inspect(glyfOutput).metadata.tables).not.toContain('gvar')
  expect(inspect(cff2Output).metadata.tables).not.toContain('CFF2')
})

it('reduces a variable design space through the public package api', () => {
  const output = reduceVariationSpace(
    readFileSync(multiAxisVariableTtfFixture),
    {
      axes: {
        wdth: 150,
        wght: { min: 300, max: 700, default: 500 },
      },
    },
  )
  const info = inspect(output)

  expect(Buffer.isBuffer(output)).toBe(true)
  expect(info.format).toBe('ttf')
  expect(info.metadata.tables).toContain('fvar')
  expect(info.metadata.tables).toContain('gvar')
}, 30_000)

it('converts glyf-backed OTF to TTF through the public package api', () => {
  const input = otfFromTtf(readFileSync(fixture))
  const output = otfToTtf(input)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(output.subarray(4)).toStrictEqual(input.subarray(4))
})

it('converts a real static CFF OTF to TTF through the public package api', () => {
  const output = otfToTtf(readFileSync(cffFixture))
  const info = inspect(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Source Sans 3')
  expect(info.metadata.tables).not.toContain('CFF ')
  expect(info.metadata.tables).toContain('glyf')
  expect(info.metadata.tables).toContain('GSUB')
  expect(info.metadata.tables).toContain('GPOS')
})

it('instantiates CFF2 coordinates through the public package api', () => {
  const output = otfToTtf(readFileSync(cff2Fixture), {
    variationCoordinates: { wght: 700, opsz: 14 },
  })
  const info = inspect(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Source Serif 4 Variable')
  expect(info.metadata.tables).toContain('glyf')
  expect(info.metadata.tables).not.toContain('CFF2')
  expect(info.metadata.tables).not.toContain('fvar')
  expect(info.metadata.tables).not.toContain('HVAR')
})

it('converts TTF to WOFF through the public package api', () => {
  const input = readFileSync(fixture)
  const output = ttfToWoff(input)

  expect(output.subarray(0, 4).toString('ascii')).toBe('wOFF')
  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('converts TTF to WOFF with metadata and private data through the public package api', () => {
  const input = readFileSync(fixture)
  const metadata =
    '<?xml version="1.0" encoding="UTF-8"?><metadata version="1.0" />'
  const privateData = Buffer.from('fontmin-rs private data')
  const output = ttfToWoff(input, { metadata, privateData })
  const metaOffset = output.readUInt32BE(24)
  const metaLength = output.readUInt32BE(28)
  const metaOriginalLength = output.readUInt32BE(32)
  const privateOffset = output.readUInt32BE(36)
  const privateLength = output.readUInt32BE(40)
  const decoded = woffToTtf(output)

  expect(output.subarray(0, 4).toString('ascii')).toBe('wOFF')
  expect(output.readUInt32BE(8)).toBe(output.byteLength)
  expect(metaOffset % 4).toBe(0)
  expect(privateOffset % 4).toBe(0)
  expect(metaOriginalLength).toBe(Buffer.byteLength(metadata))
  expect(
    inflateSync(output.subarray(metaOffset, metaOffset + metaLength)).toString(
      'utf8',
    ),
  ).toBe(metadata)
  expect(
    output.subarray(privateOffset, privateOffset + privateLength),
  ).toStrictEqual(privateData)
  expect(decoded.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
})

it('decodes WOFF to TTF through the public package api', () => {
  const input = readFileSync(fixture)
  const woff = ttfToWoff(input)
  const output = woffToTtf(woff)
  const info = inspect(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Roboto')
})

it('inspects WOFF metadata through the public package api', () => {
  const input = readFileSync(fixture)
  const woff = ttfToWoff(input)
  const info = inspect(woff)

  expect(info.format).toBe('woff')
  expect(info.size).toBe(woff.byteLength)
  expect(info.metadata.fullName).toBe('Roboto Regular')
})

it('converts TTF to WOFF2 through the public package api', () => {
  const input = readFileSync(fixture)
  const output = ttfToWoff2(input)
  const declaredLength = output.readUInt32BE(8)

  expect(output.subarray(0, 4).toString('ascii')).toBe('wOF2')
  expect(declaredLength).toBe(output.byteLength)
  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('inspects WOFF2 table metadata through the public package api', () => {
  const input = readFileSync(fixture)
  const woff2 = ttfToWoff2(input)
  const info = inspect(woff2)

  expect(info.format).toBe('woff2')
  expect(info.size).toBe(woff2.byteLength)
  expect(info.metadata.familyName).toBe('Roboto')
  expect(info.metadata.fullName).toBe('Roboto Regular')
  expect(info.metadata.glyphCount).toBe(3387)
  expect(info.metadata.unitsPerEm).toBe(2048)
  expect(info.metadata.tables).toContain('cmap')
  expect(info.metadata.tables).toContain('name')
})

it('decodes WOFF2 to TTF through the public package api', () => {
  const input = readFileSync(fixture)
  const woff2 = ttfToWoff2(input)
  const output = woff2ToTtf(woff2)
  const info = inspect(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Roboto')
  expect(info.metadata.glyphCount).toBe(3387)
})

it('validates WOFF2 data through the public package api', () => {
  const input = readFileSync(fixture)
  const woff2 = ttfToWoff2(input)

  expect(() => validateWoff2(woff2)).not.toThrow()
  expect(() => validateWoff2(Buffer.from('not woff2'))).toThrow(
    'expected WOFF2 data',
  )
})

it('uses native WOFF2 fallback modes through the public package api', () => {
  const input = readFileSync(fixture)

  for (const fallback of ['native', 'auto'] as const) {
    const output = ttfToWoff2(input, { fallback })

    expect(output.subarray(0, 4).toString('ascii')).toBe('wOF2')
  }
})

it('reports unavailable non-native WOFF2 fallback modes through the public package api', () => {
  const input = readFileSync(fixture)

  expect(() => ttfToWoff2(input, { fallback: 'wasm' })).toThrow(
    'WOFF2 fallback `wasm` is asynchronous; use ttfToWoff2Async() instead.',
  )
  expect(() => ttfToWoff2(input, { fallback: 'js' })).toThrow(
    'WOFF2 fallback `js` is not available',
  )
})

it('encodes WOFF2 through the asynchronous WASM fallback without caller setup', async () => {
  const input = readFileSync(fixture)
  const output = await ttfToWoff2Async(input, { fallback: 'wasm' })

  expect(output.subarray(0, 4).toString('ascii')).toBe('wOF2')
  expect(woff2ToTtf(output).subarray(0, 4)).toStrictEqual(
    Buffer.from([0, 1, 0, 0]),
  )
  await expect(ttfToWoff2Async(input, { fallback: 'js' })).rejects.toThrow(
    'WOFF2 fallback `js` is not available',
  )
})

it('labels WOFF2 WASM encoding failures', async () => {
  await expect(
    ttfToWoff2Async(Buffer.from('not a font'), { fallback: 'wasm' }),
  ).rejects.toThrow('WOFF2 WASM fallback failed')
})

it('converts TTF to EOT through the public package api', () => {
  const input = readFileSync(fixture)
  const output = ttfToEot(input)

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

it('converts TTF to SVG through the public package api', () => {
  const input = readFileSync(fixture)
  const svg = ttfToSvg(input)

  expect(svg.startsWith('<svg')).toBe(true)
  expect(svg).toContain('<font ')
  expect(svg).toContain('font-family="Roboto"')
  expect(svg).toContain('unicode="A"')
  expect(svg).toContain('d="M')
})

it('combines SVG icons into a TTF through the public package api', () => {
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
  const info = inspect(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Icon Set')
  expect(info.metadata.glyphCount).toBe(3)
  expect(info.metadata.unitsPerEm).toBe(1000)
})

it('converts an SVG font to a TTF through the public package api', () => {
  const output = svgFontToTtf(svgFont, { normalize: true, hinting: false })
  const info = inspect(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('SVG Icons')
  expect(info.metadata.glyphCount).toBe(2)
  expect(info.metadata.unitsPerEm).toBe(1000)
})

it('decodes EOT to TTF through the public package api', () => {
  const input = readFileSync(fixture)
  const eot = ttfToEot(input)
  const output = eotToTtf(eot)
  const info = inspect(eot)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(output.byteLength).toBe(input.byteLength)
  expect(info.format).toBe('eot')
  expect(info.metadata.fullName).toBe('Roboto Regular')
})
