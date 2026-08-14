import { readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import {
  autoDeliverySlices as browserAutoDeliverySlices,
  css as browserCss,
  modernWeb as browserModernWeb,
  optimizeBrowser,
  ttf2woff2 as browserTtf2woff2,
} from '@fontmin-rs/wasm'
import { describe, expect, it } from 'vitest'
import type { FontminDiagnosticCode } from '../src/diagnostics'
import { optimize } from '../src/optimize'
import {
  createRuntimeSelector,
  createWasmRuntime,
} from '../src/optimize-runtime'
import type { OptimizeRuntime } from '../src/optimize-runtime'
import {
  autoDeliverySlices,
  css,
  deliverySlices,
  ttf2woff2,
} from '../src/plugins'
import { fontminCompatPreset, modernWeb } from '../src/presets'
import type {
  AutoDeliveryOptions,
  FontAsset,
  FontInfo,
  FontminPlugin,
} from '../src/types'

const fixture = new URL(
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
  import.meta.url,
)
const cjkFixture = new URL(
  '../../../fixtures/fonts/ttf/noto-sans-sc-compact.ttf',
  import.meta.url,
)
const variableTtfFixture = new URL(
  '../../../fixtures/fonts/ttf/noto-sans-sc-variable-compact.ttf',
  import.meta.url,
)
const iconFixture = new URL(
  '../../../fixtures/fonts/otf/font-awesome-free-solid-900.otf',
  import.meta.url,
)
const svgFontFixture =
  '<svg xmlns="http://www.w3.org/2000/svg"><defs><font id="icons" horiz-adv-x="1000"><font-face font-family="SVG Icons" units-per-em="1000" ascent="850" descent="-150" /><glyph glyph-name="box" unicode="A" horiz-adv-x="1000" d="M100 100 L900 100 L900 900 L100 900 Z" /></font></defs></svg>'
const malformedManifest = new URL(
  '../../../fixtures/malformed/manifest.json',
  import.meta.url,
)

const fontFormats = new Set(['eot', 'otf', 'ttf', 'woff', 'woff2'])
const autoDeliveryOptions = {
  frequencyText: 'AB中文',
  languages: ['en', 'zh-Hans'],
  maxSlices: 8,
  measureFormat: 'ttf' as const,
  targetBytes: 2_000,
  tolerance: 0,
} satisfies AutoDeliveryOptions

interface MalformedManifest {
  cases: {
    encoding?: 'hex'
    expectedDiagnostic: {
      code: FontminDiagnosticCode
      message: string
    }
    operation: 'inspect' | 'otfToTtf' | 'subsetTtf'
    path: string
  }[]
  schemaVersion: 1
}

type MalformedManifestCase = MalformedManifest['cases'][number]

interface RuntimeContractCase {
  create: () => Promise<OptimizeRuntime>
  kind: OptimizeRuntime['kind']
}

const runtimeContractCases = [
  {
    async create() {
      return createRuntimeSelector('native').resolve()
    },
    kind: 'native',
  },
  {
    create: createWasmRuntime,
    kind: 'wasm',
  },
] satisfies RuntimeContractCase[]

async function conformanceRuntimes(): Promise<
  [native: OptimizeRuntime, wasm: OptimizeRuntime]
> {
  return Promise.all([
    createRuntimeSelector('native').resolve(),
    createWasmRuntime(),
  ])
}

function normalizeFontInfo(info: FontInfo) {
  return {
    format: info.format,
    metadata: {
      ...info.metadata,
      tables: info.metadata.tables.toSorted(),
    },
  }
}

function cssSources(contents: Uint8Array) {
  return [
    {
      contents,
      fileName: 'conformance.ttf',
      format: 'ttf' as const,
      unicodeRanges: ['U+0041-0042', 'U+4E00-9FFF'],
    },
  ]
}

function sfntTable(input: Uint8Array, tag: string) {
  const view = new DataView(input.buffer, input.byteOffset, input.byteLength)
  const tableCount = view.getUint16(4)

  for (let index = 0; index < tableCount; index += 1) {
    const recordOffset = 12 + index * 16
    const recordTag = new TextDecoder().decode(
      input.subarray(recordOffset, recordOffset + 4),
    )

    if (recordTag === tag) {
      const offset = view.getUint32(recordOffset + 8)
      const length = view.getUint32(recordOffset + 12)

      return input.subarray(offset, offset + length)
    }
  }

  throw new Error(`missing ${tag} table`)
}

function glyphZeroDataLength(input: Uint8Array) {
  const head = sfntTable(input, 'head')
  const loca = sfntTable(input, 'loca')
  const headView = new DataView(head.buffer, head.byteOffset, head.byteLength)
  const locaView = new DataView(loca.buffer, loca.byteOffset, loca.byteLength)

  if (headView.getInt16(50) === 0) {
    return (locaView.getUint16(2) - locaView.getUint16(0)) * 2
  }

  return locaView.getUint32(4) - locaView.getUint32(0)
}

async function captureDiagnostic(operation: () => Promise<unknown>) {
  try {
    await operation()
  } catch (error) {
    if (
      error instanceof Error &&
      'code' in error &&
      typeof error.code === 'string'
    ) {
      return { code: error.code, message: error.message }
    }
    throw new TypeError('runtime rejected without a diagnostic code', {
      cause: error,
    })
  }

  throw new Error('runtime unexpectedly accepted malformed input')
}

async function readMalformedInput(testCase: MalformedManifestCase) {
  const contents = await readFile(
    new URL(`../../../${testCase.path}`, import.meta.url),
  )

  if (testCase.encoding === undefined) {
    return contents
  }

  const hex = contents.toString('utf8').trim()
  if (!/^(?:[0-9a-f]{2})+$/u.test(hex)) {
    throw new Error(
      `${testCase.path} must contain complete lowercase hex bytes`,
    )
  }

  return Buffer.from(hex, 'hex')
}

function runMalformedOperation(
  runtime: OptimizeRuntime,
  testCase: MalformedManifestCase,
  input: Uint8Array,
) {
  if (testCase.operation === 'inspect') {
    return runtime.inspect(input)
  }
  if (testCase.operation === 'subsetTtf') {
    return runtime.subsetTtf(input, { text: 'A中' })
  }

  return runtime.otfToTtf(input, {})
}

async function normalizeAssets(assets: FontAsset[], runtime: OptimizeRuntime) {
  return Promise.all(
    assets.map(async asset => ({
      format: asset.format,
      path: asset.path,
      semantic: fontFormats.has(asset.format)
        ? normalizeFontInfo(await runtime.inspect(asset.contents))
        : new TextDecoder().decode(asset.contents),
      sourceFormat: asset.sourceFormat,
    })),
  )
}

async function runPipeline(
  kind: OptimizeRuntime['kind'],
  plugins: FontminPlugin[],
) {
  return optimize({
    cache: false,
    input: [fileURLToPath(cjkFixture)],
    plugins,
    runtime: kind,
  })
}

describe.each(runtimeContractCases)(
  '$kind OptimizeRuntime contract',
  testCase => {
    const inputPromise = readFile(fixture)
    const runtimePromise = testCase.create()

    it('returns inspectable formats, metadata, glyphs, and cmap data', async () => {
      const [input, runtime] = await Promise.all([inputPromise, runtimePromise])
      const originalInfo = await runtime.inspect(input)
      const subset = await runtime.subsetTtf(input, { text: 'AB' })
      const subsetInfo = await runtime.inspect(subset)
      const svg = await runtime.ttfToSvg(subset, {})
      const woff2 = await runtime.ttfToWoff2(subset, {})
      const woff2Info = await runtime.inspect(woff2)

      expect(new TextDecoder().decode(woff2.subarray(0, 4))).toBe('wOF2')
      expect(woff2Info).toMatchObject({
        format: 'woff2',
        metadata: {
          familyName: 'Roboto',
          glyphCount: subsetInfo.metadata.glyphCount,
          unitsPerEm: originalInfo.metadata.unitsPerEm,
        },
        size: woff2.length,
      })
      expect(subsetInfo.metadata.glyphCount).toBeLessThan(
        originalInfo.metadata.glyphCount,
      )
      expect(subsetInfo.metadata.tables).toContain('cmap')
      expect(svg).toContain('unicode="A"')
      expect(svg).toContain('unicode="B"')
      expect(svg).not.toContain('unicode="C"')
    })

    it('instances glyf variable fonts before the modern Web pipeline', async () => {
      const assets = await optimize({
        cache: false,
        input: [fileURLToPath(variableTtfFixture)],
        plugins: modernWeb({
          text: 'AB',
          variationCoordinates: { wght: 900 },
        }),
        runtime: testCase.kind,
      })
      const woff2 = assets.find(asset => asset.format === 'woff2')

      expect(woff2).toBeDefined()
      const runtime = await runtimePromise
      const ttf = await runtime.woff2ToTtf(woff2!.contents)
      const info = await runtime.inspect(ttf)

      expect(info.metadata.tables).not.toContain('fvar')
      expect(info.metadata.tables).not.toContain('gvar')
    })

    it('applies defaults and maps shared options', async () => {
      const [input, runtime] = await Promise.all([inputPromise, runtimePromise])
      const woff = await runtime.ttfToWoff(input, {})
      const untrimmed = await runtime.subsetTtf(input, {
        hinting: true,
        keepLayout: 'preserve',
        text: 'AB',
        trim: false,
      })
      const css = await runtime.generateFontFaceCss(
        [{ fileName: 'roboto.woff', format: 'woff' }],
        {
          fontDisplay: 'optional',
          fontFamily: 'Runtime Contract',
          local: false,
        },
      )

      expect(new TextDecoder().decode(woff.subarray(0, 4))).toBe('wOFF')
      expect(Buffer.from(untrimmed).equals(input)).toBe(true)
      expect(css).toContain("font-family: 'Runtime Contract'")
      expect(css).toContain('font-display: optional')
      expect(css).not.toContain("local('Runtime Contract')")
    })

    it('enforces observable subset policy semantics', async () => {
      const [input, runtime] = await Promise.all([inputPromise, runtimePromise])
      const [minimal, hinted, conservative] = await Promise.all([
        runtime.subsetTtf(input, {
          keepLayout: 'drop',
          keepNotdef: false,
          preserveHinting: false,
          text: 'Hello',
        }),
        runtime.subsetTtf(input, {
          keepLayout: 'drop',
          keepNotdef: true,
          preserveHinting: true,
          text: 'Hello',
        }),
        runtime.subsetTtf(input, {
          keepLayout: 'conservative',
          text: 'Hello',
        }),
      ])
      const [minimalInfo, hintedInfo, conservativeInfo] = await Promise.all([
        runtime.inspect(minimal),
        runtime.inspect(hinted),
        runtime.inspect(conservative),
      ])

      expect(minimal.length).toBeLessThan(hinted.length)
      expect(glyphZeroDataLength(minimal)).toBe(0)
      expect(glyphZeroDataLength(hinted)).toBeGreaterThan(0)
      expect(minimalInfo.metadata.tables).not.toContain('cvt ')
      expect(minimalInfo.metadata.tables).not.toContain('fpgm')
      expect(minimalInfo.metadata.tables).not.toContain('prep')
      expect(minimalInfo.metadata.tables).not.toContain('GDEF')
      expect(minimalInfo.metadata.tables).not.toContain('GPOS')
      expect(minimalInfo.metadata.tables).not.toContain('GSUB')
      expect(hintedInfo.metadata.tables).toContain('cvt ')
      expect(hintedInfo.metadata.tables).toContain('fpgm')
      expect(hintedInfo.metadata.tables).toContain('prep')
      expect(conservativeInfo.metadata.tables).toContain('GDEF')
      expect(conservativeInfo.metadata.tables).toContain('GPOS')
      expect(conservativeInfo.metadata.tables).toContain('GSUB')
      await expect(
        runtime.subsetTtf(input, {
          keepLayout: 'preserve',
          text: 'Hello',
        }),
      ).rejects.toThrow(
        'keepLayout preserve could not retain 31 contextual layout subtables',
      )
    })

    it('accepts documented compatibility hint options as no-ops', async () => {
      const [icon, runtime] = await Promise.all([
        readFile(iconFixture),
        runtimePromise,
      ])

      const [defaultSvg, compatibleSvg] = await Promise.all([
        runtime.svgFontToTtf(svgFontFixture, {}),
        runtime.svgFontToTtf(svgFontFixture, { hinting: true }),
      ])
      const [defaultOtf, compatibleOtf] = await Promise.all([
        runtime.otfToTtf(icon, {}),
        runtime.otfToTtf(icon, { preserveHinting: true }),
      ])

      expect(Buffer.from(compatibleSvg).equals(defaultSvg)).toBe(true)
      expect(Buffer.from(compatibleOtf).equals(defaultOtf)).toBe(true)
    })

    it('uses the same Error contract for business failures', async () => {
      const runtime = await runtimePromise
      const operation = runtime.ttfToWoff2(
        new TextEncoder().encode('not a font'),
        {},
      )

      await expect(operation).rejects.toBeInstanceOf(Error)
      await expect(operation).rejects.toThrow(
        'expected TrueType sfnt data for WOFF2 encoding',
      )
    })
  },
)

describe('runtime-specific option support contract', () => {
  it('supports function-valued CSS font families in native', async () => {
    const [input, runtime] = await Promise.all([
      readFile(fixture),
      createRuntimeSelector('native').resolve(),
    ])

    await expect(
      runtime.generateFontFaceCss(
        [{ contents: input, fileName: 'roboto.ttf', format: 'ttf' }],
        { fontFamily: () => 'Resolved Runtime Contract' },
      ),
    ).resolves.toContain("font-family: 'Resolved Runtime Contract'")
  })

  it('rejects function-valued CSS font families in WASM', async () => {
    const [input, runtime] = await Promise.all([
      readFile(fixture),
      createWasmRuntime(),
    ])

    await expect(
      runtime.generateFontFaceCss(
        [{ contents: input, fileName: 'roboto.ttf', format: 'ttf' }],
        { fontFamily: () => 'Resolved Runtime Contract' },
      ),
    ).rejects.toThrow(
      'fontmin-rs WASM generateFontFaceCss does not support option fontFamily',
    )
  })
})

describe('native and WASM semantic conformance', () => {
  it('produces byte-identical subset policy outputs', async () => {
    const [input, runtimes] = await Promise.all([
      readFile(fixture),
      conformanceRuntimes(),
    ])
    const [native, wasm] = runtimes

    for (const options of [
      {
        keepLayout: 'drop' as const,
        keepNotdef: false,
        preserveHinting: false,
        text: 'Hello',
      },
      {
        keepLayout: 'conservative' as const,
        keepNotdef: true,
        preserveHinting: true,
        text: 'Hello',
      },
    ]) {
      const [nativeOutput, wasmOutput] = await Promise.all([
        native.subsetTtf(input, options),
        wasm.subsetTtf(input, options),
      ])

      expect(Buffer.from(wasmOutput).equals(nativeOutput)).toBe(true)
    }
  })

  it('aligns every low-level built-in transform', async () => {
    const [input, icon, runtimes] = await Promise.all([
      readFile(cjkFixture),
      readFile(iconFixture),
      conformanceRuntimes(),
    ])
    const [native, wasm] = runtimes
    const subsetOptions = {
      missingGlyphs: 'error' as const,
      text: 'AB中文',
    }
    const [nativeSubset, wasmSubset] = await Promise.all([
      native.subsetTtf(input, subsetOptions),
      wasm.subsetTtf(input, subsetOptions),
    ])
    const [nativeSubsetInfo, wasmSubsetInfo] = await Promise.all([
      native.inspect(nativeSubset),
      wasm.inspect(wasmSubset),
    ])

    expect(normalizeFontInfo(wasmSubsetInfo)).toStrictEqual(
      normalizeFontInfo(nativeSubsetInfo),
    )

    for (const transform of [
      (runtime: OptimizeRuntime, data: Uint8Array) =>
        runtime.ttfToEot(data, {}),
      (runtime: OptimizeRuntime, data: Uint8Array) =>
        runtime.ttfToWoff(data, {}),
      (runtime: OptimizeRuntime, data: Uint8Array) =>
        runtime.ttfToWoff2(data, { quality: 9 }),
    ]) {
      const [nativeOutput, wasmOutput] = await Promise.all([
        transform(native, nativeSubset),
        transform(wasm, wasmSubset),
      ])
      const [nativeInfo, wasmInfo] = await Promise.all([
        native.inspect(nativeOutput),
        wasm.inspect(wasmOutput),
      ])

      expect(normalizeFontInfo(wasmInfo)).toStrictEqual(
        normalizeFontInfo(nativeInfo),
      )
    }

    const [nativeSvg, wasmSvg] = await Promise.all([
      native.ttfToSvg(nativeSubset, { fontFamily: 'Conformance CJK' }),
      wasm.ttfToSvg(wasmSubset, { fontFamily: 'Conformance CJK' }),
    ])

    expect(wasmSvg).toBe(nativeSvg)

    const [nativeSvgTtf, wasmSvgTtf] = await Promise.all([
      native.svgFontToTtf(nativeSvg, {}),
      wasm.svgFontToTtf(wasmSvg, {}),
    ])
    const icons = [
      {
        contents:
          '<svg viewBox="0 0 100 100"><path d="M0 0 L100 0 L100 100 Z"/></svg>',
        name: 'triangle',
        unicode: 0xe101,
      },
      {
        contents:
          '<svg viewBox="0 0 100 100"><path d="M0 0 L100 0 L100 100 L0 100 Z"/></svg>',
        name: 'square',
        unicode: 0xe102,
      },
    ]
    const [nativeIconTtf, wasmIconTtf] = await Promise.all([
      native.svgsToTtf(icons, { fontName: 'Conformance Icons' }),
      wasm.svgsToTtf(icons, { fontName: 'Conformance Icons' }),
    ])
    const [nativeOtfTtf, wasmOtfTtf] = await Promise.all([
      native.otfToTtf(icon, {}),
      wasm.otfToTtf(icon, {}),
    ])

    const fontPairs: [native: Uint8Array, wasm: Uint8Array][] = [
      [nativeSvgTtf, wasmSvgTtf],
      [nativeIconTtf, wasmIconTtf],
      [nativeOtfTtf, wasmOtfTtf],
    ]

    for (const [nativeOutput, wasmOutput] of fontPairs) {
      const [nativeInfo, wasmInfo] = await Promise.all([
        native.inspect(nativeOutput),
        wasm.inspect(wasmOutput),
      ])

      expect(normalizeFontInfo(wasmInfo)).toStrictEqual(
        normalizeFontInfo(nativeInfo),
      )
    }

    const cssOptions = {
      fontDisplay: 'swap' as const,
      fontFamily: 'Conformance CJK',
      local: false,
    }
    const [nativeCss, wasmCss] = await Promise.all([
      native.generateFontFaceCss(cssSources(nativeSubset), cssOptions),
      wasm.generateFontFaceCss(cssSources(wasmSubset), cssOptions),
    ])

    expect(wasmCss).toBe(nativeCss)
  })

  it('aligns every built-in preset and delivery pipeline', async () => {
    const [native, wasm] = await conformanceRuntimes()
    const cases: (() => FontminPlugin[])[] = [
      () =>
        modernWeb({
          fontDisplay: 'swap',
          fontFamily: 'Conformance CJK',
          local: false,
          text: 'AB中文',
        }),
      () =>
        fontminCompatPreset({
          fontDisplay: 'swap',
          fontFamily: 'Conformance CJK',
          local: false,
          text: 'AB中文',
        }),
      () => [
        deliverySlices([
          { name: 'latin', unicodeRanges: ['U+0041-0042'] },
          { name: 'cjk', unicodeRanges: ['U+4E2D', 'U+6587'] },
        ]),
        ttf2woff2(),
        css({ fontFamily: 'Conformance CJK', local: false }),
      ],
      () => [
        autoDeliverySlices(autoDeliveryOptions),
        ttf2woff2({ clone: false }),
        css({ fontFamily: 'Automatic CJK', local: false }),
      ],
    ]

    for (const createPlugins of cases) {
      const [nativeAssets, wasmAssets] = await Promise.all([
        runPipeline('native', createPlugins()),
        runPipeline('wasm', createPlugins()),
      ])

      await expect(normalizeAssets(wasmAssets, wasm)).resolves.toStrictEqual(
        await normalizeAssets(nativeAssets, native),
      )
    }
  })

  it('aligns Node and browser optimizer asset semantics', async () => {
    const [contents, runtime] = await Promise.all([
      readFile(cjkFixture),
      createWasmRuntime(),
    ])
    const options = {
      fontDisplay: 'swap' as const,
      fontFamily: 'Cross Pipeline CJK',
      local: false,
      text: 'AB中文',
    }
    const [nodeAssets, browserAssets] = await Promise.all([
      runPipeline('wasm', modernWeb(options)),
      optimizeBrowser({
        assets: [
          {
            contents,
            fileName: 'noto-sans-sc-compact.ttf',
          },
        ],
        plugins: browserModernWeb(options),
      }),
    ])
    const normalizePipelineAssets = (
      assets: {
        contents: Uint8Array
        fileName?: string
        format?: string
        path?: string
      }[],
    ) =>
      Promise.all(
        assets.map(async asset => ({
          format: asset.format,
          name: asset.path ?? asset.fileName,
          semantic:
            asset.format !== undefined && fontFormats.has(asset.format)
              ? normalizeFontInfo(await runtime.inspect(asset.contents))
              : new TextDecoder().decode(asset.contents),
        })),
      )

    await expect(normalizePipelineAssets(browserAssets)).resolves.toStrictEqual(
      await normalizePipelineAssets(nodeAssets),
    )

    const [nodeSlices, browserSlices] = await Promise.all([
      runPipeline('wasm', [
        autoDeliverySlices(autoDeliveryOptions),
        ttf2woff2({ clone: false }),
        css({ fontFamily: 'Automatic CJK', local: false }),
      ]),
      optimizeBrowser({
        assets: [
          {
            contents,
            fileName: 'noto-sans-sc-compact.ttf',
          },
        ],
        plugins: [
          browserAutoDeliverySlices(autoDeliveryOptions),
          browserTtf2woff2({ clone: false }),
          browserCss({ fontFamily: 'Automatic CJK', local: false }),
        ],
      }),
    ])

    expect(
      nodeSlices.filter(asset => asset.format === 'woff2').length,
    ).toBeGreaterThan(1)
    await expect(normalizePipelineAssets(browserSlices)).resolves.toStrictEqual(
      await normalizePipelineAssets(nodeSlices),
    )
  })

  it('returns matching stable diagnostics for the malformed corpus', async () => {
    const [native, wasm] = await conformanceRuntimes()
    const manifest = JSON.parse(
      await readFile(malformedManifest, 'utf8'),
    ) as MalformedManifest

    expect(manifest.schemaVersion).toBe(1)

    for (const testCase of manifest.cases) {
      const input = await readMalformedInput(testCase)
      const [nativeDiagnostic, wasmDiagnostic] = await Promise.all([
        captureDiagnostic(() => runMalformedOperation(native, testCase, input)),
        captureDiagnostic(() => runMalformedOperation(wasm, testCase, input)),
      ])

      expect(nativeDiagnostic).toStrictEqual(wasmDiagnostic)
      expect(nativeDiagnostic).toStrictEqual(testCase.expectedDiagnostic)
    }
  })
})
