import { readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { optimize } from '../src/optimize'
import {
  createRuntimeSelector,
  createWasmRuntime,
} from '../src/optimize-runtime'
import type { OptimizeRuntime } from '../src/optimize-runtime'
import { css, deliverySlices, ttf2woff2 } from '../src/plugins'
import { fontminCompatPreset, modernWeb } from '../src/presets'
import type { FontAsset, FontInfo, FontminPlugin } from '../src/types'

const fixture = new URL(
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
  import.meta.url,
)
const cjkFixture = new URL(
  '../../../fixtures/fonts/ttf/noto-sans-sc-compact.ttf',
  import.meta.url,
)
const iconFixture = new URL(
  '../../../fixtures/fonts/otf/font-awesome-free-solid-900.otf',
  import.meta.url,
)
const malformedManifest = new URL(
  '../../../fixtures/malformed/manifest.json',
  import.meta.url,
)

const fontFormats = new Set(['eot', 'otf', 'ttf', 'woff', 'woff2'])

interface MalformedManifest {
  cases: {
    encoding?: 'hex'
    operation: 'inspect' | 'otfToTtf'
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

async function captureErrorMessage(operation: () => Promise<unknown>) {
  try {
    await operation()
  } catch (error) {
    if (error instanceof Error) {
      return error.message
    }
    throw new TypeError('runtime rejected with a non-Error value', {
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

  it('returns matching stable diagnostics for the malformed corpus', async () => {
    const [native, wasm] = await conformanceRuntimes()
    const manifest = JSON.parse(
      await readFile(malformedManifest, 'utf8'),
    ) as MalformedManifest

    expect(manifest.schemaVersion).toBe(1)

    for (const testCase of manifest.cases) {
      const input = await readMalformedInput(testCase)
      const [nativeMessage, wasmMessage] = await Promise.all([
        captureErrorMessage(() =>
          runMalformedOperation(native, testCase, input),
        ),
        captureErrorMessage(() => runMalformedOperation(wasm, testCase, input)),
      ])

      expect(wasmMessage).toBe(nativeMessage)
    }
  })
})
