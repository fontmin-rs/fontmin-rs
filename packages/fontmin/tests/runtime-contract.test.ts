import { readFile } from 'node:fs/promises'
import { describe, expect, it } from 'vitest'
import {
  createRuntimeSelector,
  createWasmRuntime,
} from '../src/optimize-runtime'
import type { OptimizeRuntime } from '../src/optimize-runtime'

const fixture = new URL(
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
  import.meta.url,
)

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
