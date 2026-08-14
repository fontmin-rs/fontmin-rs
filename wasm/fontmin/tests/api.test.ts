import { readFile } from 'node:fs/promises'
import { beforeAll, expect, it, vi } from 'vitest'
import {
  FontminDiagnosticError,
  analyzeCoverage,
  eotToTtf,
  generateFontFaceCss,
  initWasm,
  inspect,
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
  woff2ToTtf,
  woffToTtf,
} from '../src/index'

const fixture = new URL(
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
  import.meta.url,
)
const variableTtfFixture = new URL(
  '../../../fixtures/fonts/ttf/noto-sans-sc-variable-compact.ttf',
  import.meta.url,
)
const multiAxisVariableTtfFixture = new URL(
  '../../../fixtures/fonts/ttf/estedad-variable.ttf',
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

it('instantiates a glyf variable font through WASM', async () => {
  const output = await instantiateFont(await readFile(variableTtfFixture), {
    variationCoordinates: { wght: 900 },
  })
  const info = await inspect(output)

  expect(output).toBeInstanceOf(Uint8Array)
  expect(info).toMatchObject({ format: 'ttf', metadata: { glyphCount: 5 } })
  expect(info.metadata.tables).not.toContain('fvar')
  expect(info.metadata.tables).not.toContain('gvar')
})

it('reduces a variable design space through WASM', async () => {
  const output = await reduceVariationSpace(
    await readFile(multiAxisVariableTtfFixture),
    {
      axes: {
        wdth: 150,
        wght: { min: 300, max: 700, default: 500 },
      },
    },
  )
  const info = await inspect(output)

  expect(output).toBeInstanceOf(Uint8Array)
  expect(info.format).toBe('ttf')
  expect(info.metadata.tables).toContain('fvar')
  expect(info.metadata.tables).toContain('gvar')
})

it('returns stable diagnostic codes for malformed WASM input', async () => {
  const operation = inspect(new TextEncoder().encode('not-a-font'))

  await expect(operation).rejects.toBeInstanceOf(FontminDiagnosticError)
  await expect(operation).rejects.toMatchObject({
    code: 'fontmin::invalid_font',
    message: 'invalid font data: unknown font format',
    name: 'FontminDiagnosticError',
  })
})

it('analyzes coverage and applies missing glyph policies', async () => {
  const input = await readFile(fixture)
  const report = await analyzeCoverage(input, { text: 'A𠮷' })
  const warning = vi.spyOn(console, 'warn').mockImplementation(() => {})

  expect(report).toStrictEqual({
    coveragePercent: 50,
    missing: [134_071],
    requested: [0x41, 134_071],
    supported: [0x41],
  })

  try {
    await expect(subsetTtf(input, { text: 'A𠮷' })).resolves.toBeInstanceOf(
      Uint8Array,
    )
    expect(warning).toHaveBeenCalledWith(expect.stringContaining('U+20BB7'))
    await expect(
      subsetTtf(input, { missingGlyphs: 'error', text: 'A𠮷' }),
    ).rejects.toThrow('U+20BB7')
  } finally {
    warning.mockRestore()
  }
})

it('subsets by original glyph ID without a Unicode selector', async () => {
  const input = await readFile(fixture)
  const output = await subsetTtf(input, { gids: [1] })

  expect(output).toBeInstanceOf(Uint8Array)
  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('returns subset mappings through the WASM API', async () => {
  const input = await readFile(fixture)
  const result = await subsetTtfWithReport(input, {
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

  expect(result.data).toBeInstanceOf(Uint8Array)
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

it('rejects invalid unicode ranges in subset options', async () => {
  const ttf = await readFile(fixture)

  await expect(subsetTtf(ttf, { unicodeRanges: ['U+4??'] })).rejects.toThrow(
    'invalid WASM options: configuration error: invalid Unicode range: U+4??',
  )
})

it('rejects invalid option types instead of using defaults', async () => {
  const ttf = await readFile(fixture)

  await expect(
    // @ts-expect-error quality must be a number
    ttfToWoff2(ttf, { quality: 'high' }),
  ).rejects.toThrow('invalid WASM options: invalid type: string')
})

it('rejects invalid option enums instead of using defaults', async () => {
  const ttf = await readFile(fixture)

  await expect(
    // @ts-expect-error layout must be a supported subset mode
    subsetTtf(ttf, { layout: 'aggressive', text: 'A' }),
  ).rejects.toThrow('invalid WASM options: unknown variant `aggressive`')
})
