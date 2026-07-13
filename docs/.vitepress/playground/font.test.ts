import { describe, expect, it, vi } from 'vitest'
import {
  detectInputFormat,
  createDeliverySlices,
  isSupportedInputFile,
  parseUnicodeRanges,
  processFont,
} from './font'
import type { BrowserWasmApi } from './font'

const bytes = (...values: number[]) => new Uint8Array(values)

function createWasm() {
  return {
    eotToTtf: vi.fn<BrowserWasmApi['eotToTtf']>().mockResolvedValue(bytes(4)),
    generateFontFaceCss: vi
      .fn<BrowserWasmApi['generateFontFaceCss']>()
      .mockResolvedValue('@font-face {}'),
    initWasm: vi.fn<BrowserWasmApi['initWasm']>().mockResolvedValue(),
    otfToTtf: vi.fn<BrowserWasmApi['otfToTtf']>().mockResolvedValue(bytes(5)),
    subsetTtf: vi
      .fn<BrowserWasmApi['subsetTtf']>()
      .mockImplementation(async input => input),
    svgFontToTtf: vi
      .fn<BrowserWasmApi['svgFontToTtf']>()
      .mockResolvedValue(bytes(6)),
    ttfToEot: vi.fn<BrowserWasmApi['ttfToEot']>().mockResolvedValue(bytes(7)),
    ttfToSvg: vi.fn<BrowserWasmApi['ttfToSvg']>().mockResolvedValue('<svg />'),
    ttfToWoff: vi.fn<BrowserWasmApi['ttfToWoff']>().mockResolvedValue(bytes(8)),
    ttfToWoff2: vi
      .fn<BrowserWasmApi['ttfToWoff2']>()
      .mockResolvedValue(bytes(9)),
    woff2ToTtf: vi
      .fn<BrowserWasmApi['woff2ToTtf']>()
      .mockResolvedValue(bytes(2)),
    woffToTtf: vi.fn<BrowserWasmApi['woffToTtf']>().mockResolvedValue(bytes(3)),
  }
}

describe('detectInputFormat', () => {
  it.each(['ttf', 'woff', 'woff2', 'eot', 'otf', 'svg'])(
    'accepts %s files',
    extension => {
      expect(detectInputFormat(`font.${extension}`)).toBe(extension)
    },
  )

  it('rejects unsupported input extensions', () => {
    expect(() => detectInputFormat('font.zip')).toThrow(
      'Unsupported input format: .zip.',
    )
  })
})

describe('isSupportedInputFile', () => {
  it('accepts supported font extensions and rejects unknown files', () => {
    expect(isSupportedInputFile('font.woff2')).toBe(true)
    expect(isSupportedInputFile('font.bin')).toBe(false)
  })
})

describe('parseUnicodeRanges', () => {
  it('splits valid CSS descriptors while preserving their source spelling', () => {
    expect(parseUnicodeRanges('U+0020-007E, u+4e00-9fff')).toStrictEqual([
      'U+0020-007E',
      'u+4e00-9fff',
    ])
  })

  it.each(['U+4??', 'U+110000', 'U+007E-0020', 'U+0041; color:red'])(
    'rejects invalid descriptor %s',
    value => {
      expect(() => parseUnicodeRanges(value)).toThrow(
        `Invalid Unicode range: ${value}.`,
      )
    },
  )
})

describe('createDeliverySlices', () => {
  it('creates enabled presets in a stable order', () => {
    expect(createDeliverySlices(new Set(['cjk', 'latin']), '')).toStrictEqual([
      { name: 'latin', unicodeRanges: ['U+0000-00FF'] },
      { name: 'cjk', unicodeRanges: ['U+4E00-9FFF'] },
    ])
  })

  it('validates enabled custom ranges', () => {
    expect(() => createDeliverySlices(new Set(['custom']), 'U+4??')).toThrow(
      'Invalid Unicode range: U+4??.',
    )
  })
})

describe('processFont', () => {
  it('normalizes WOFF2, subsets it, and emits selected outputs', async () => {
    const wasm = createWasm()

    const outputs = await processFont(
      {
        contents: bytes(1),
        fileName: 'demo.woff2',
        formats: new Set(['woff2', 'css']),
        text: 'Hello',
        unicodeRanges: ['U+0020-007E'],
      },
      wasm,
    )

    expect(wasm.initWasm).toHaveBeenCalledTimes(1)
    expect(wasm.woff2ToTtf).toHaveBeenCalledWith(bytes(1))
    expect(wasm.subsetTtf).toHaveBeenCalledWith(bytes(2), {
      basicText: false,
      keepNotdef: true,
      layout: 'conservative',
      preserveHinting: false,
      text: 'Hello',
      trim: true,
      unicodes: [],
    })
    expect(wasm.ttfToWoff2).toHaveBeenCalledWith(bytes(2), {})
    expect(wasm.generateFontFaceCss).toHaveBeenCalledWith(
      [
        {
          contents: bytes(9),
          fileName: 'demo.woff2',
          format: 'woff2',
        },
      ],
      {
        asFileName: false,
        base64: false,
        fontDisplay: 'swap',
        fontFamily: 'demo',
        fontPath: './',
        glyph: false,
        iconPrefix: 'icon',
        local: false,
        target: 'css',
        unicodeRanges: ['U+0020-007E'],
      },
    )
    expect(outputs.map(output => output.fileName)).toStrictEqual([
      'demo.woff2',
      'demo.css',
    ])
  })

  it('emits named delivery slices with their own CSS Unicode ranges', async () => {
    const wasm = createWasm()

    const outputs = await processFont(
      {
        contents: bytes(1),
        deliverySlices: [
          { name: 'latin', unicodeRanges: ['U+0000-00FF'] },
          { name: 'cjk', unicodeRanges: ['U+4E00-9FFF'] },
        ],
        fileName: 'demo.ttf',
        formats: new Set(['woff2', 'css']),
        text: 'Hello',
      },
      wasm,
    )

    expect(wasm.subsetTtf).toHaveBeenNthCalledWith(1, bytes(1), {
      basicText: false,
      keepNotdef: true,
      layout: 'conservative',
      preserveHinting: false,
      text: 'Hello',
      trim: true,
      unicodeRanges: ['U+0000-00FF'],
      unicodes: [],
    })
    expect(wasm.subsetTtf).toHaveBeenNthCalledWith(2, bytes(1), {
      basicText: false,
      keepNotdef: true,
      layout: 'conservative',
      preserveHinting: false,
      text: 'Hello',
      trim: true,
      unicodeRanges: ['U+4E00-9FFF'],
      unicodes: [],
    })
    expect(wasm.generateFontFaceCss).toHaveBeenCalledWith(
      [
        {
          contents: bytes(9),
          fileName: 'demo-latin.woff2',
          format: 'woff2',
          unicodeRanges: ['U+0000-00FF'],
        },
        {
          contents: bytes(9),
          fileName: 'demo-cjk.woff2',
          format: 'woff2',
          unicodeRanges: ['U+4E00-9FFF'],
        },
      ],
      expect.not.objectContaining({ unicodeRanges: expect.any(Array) }),
    )
    expect(outputs.map(output => output.fileName)).toStrictEqual([
      'demo-latin.woff2',
      'demo-cjk.woff2',
      'demo.css',
    ])
  })

  it.each([
    ['ttf', 'subsetTtf'],
    ['woff', 'woffToTtf'],
    ['woff2', 'woff2ToTtf'],
    ['eot', 'eotToTtf'],
    ['otf', 'otfToTtf'],
    ['svg', 'svgFontToTtf'],
  ] as const)('normalizes %s input through %s', async (extension, method) => {
    const wasm = createWasm()

    await processFont(
      {
        contents: bytes(1),
        fileName: `demo.${extension}`,
        formats: new Set(['ttf']),
        text: 'A',
      },
      wasm,
    )

    expect(wasm[method]).toHaveBeenCalled()
  })

  it.each(['', '   '])('rejects missing character text', async text => {
    await expect(
      processFont(
        {
          contents: bytes(1),
          fileName: 'demo.ttf',
          formats: new Set(['woff2']),
          text,
        },
        createWasm(),
      ),
    ).rejects.toThrow('Enter at least one character to subset.')
  })

  it('rejects CSS as the only selected output', async () => {
    await expect(
      processFont(
        {
          contents: bytes(1),
          fileName: 'demo.ttf',
          formats: new Set(['css']),
          text: 'A',
        },
        createWasm(),
      ),
    ).rejects.toThrow('Select at least one font output when generating CSS.')
  })
})
