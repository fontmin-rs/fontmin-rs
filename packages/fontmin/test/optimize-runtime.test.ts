import { describe, expect, it, vi } from 'vitest'
import { NativeBindingLoadError } from '../src/native-loader'
import {
  createWasmRuntime,
  createRuntimeSelector,
  resolvePipelineRuntimeMode,
  type OptimizeRuntime,
} from '../src/optimize-runtime'
import type { WasmRuntime } from '../src/wasm-fallback'

function runtime(kind: 'native' | 'wasm'): OptimizeRuntime {
  return {
    kind,
    generateFontFaceCss: vi.fn<OptimizeRuntime['generateFontFaceCss']>(),
    inspect: vi.fn<OptimizeRuntime['inspect']>(),
    otfToTtf: vi.fn<OptimizeRuntime['otfToTtf']>(),
    subsetTtf: vi.fn<OptimizeRuntime['subsetTtf']>(),
    svgFontToTtf: vi.fn<OptimizeRuntime['svgFontToTtf']>(),
    svgsToTtf: vi.fn<OptimizeRuntime['svgsToTtf']>(),
    ttfToEot: vi.fn<OptimizeRuntime['ttfToEot']>(),
    ttfToSvg: vi.fn<OptimizeRuntime['ttfToSvg']>(),
    ttfToWoff: vi.fn<OptimizeRuntime['ttfToWoff']>(),
    ttfToWoff2: vi.fn<OptimizeRuntime['ttfToWoff2']>(),
  }
}

function wasmRuntime(): WasmRuntime {
  return {
    eotToTtf: vi.fn<WasmRuntime['eotToTtf']>(),
    generateFontFaceCss: vi.fn<WasmRuntime['generateFontFaceCss']>(),
    initWasm: vi.fn<WasmRuntime['initWasm']>(),
    inspect: vi.fn<WasmRuntime['inspect']>(),
    otfToTtf: vi.fn<WasmRuntime['otfToTtf']>(),
    subsetTtf: vi.fn<WasmRuntime['subsetTtf']>(),
    svgFontToTtf: vi.fn<WasmRuntime['svgFontToTtf']>(),
    svgsToTtf: vi.fn<WasmRuntime['svgsToTtf']>(),
    ttfToEot: vi.fn<WasmRuntime['ttfToEot']>(),
    ttfToSvg: vi.fn<WasmRuntime['ttfToSvg']>(),
    ttfToWoff: vi.fn<WasmRuntime['ttfToWoff']>(),
    ttfToWoff2: vi.fn<WasmRuntime['ttfToWoff2']>(),
    validateWoff2: vi.fn<WasmRuntime['validateWoff2']>(),
    woff2ToTtf: vi.fn<WasmRuntime['woff2ToTtf']>(),
    woffToTtf: vi.fn<WasmRuntime['woffToTtf']>(),
  }
}

describe('optimize runtime selection', () => {
  it('memoizes one explicit WASM adapter', async () => {
    const wasm = runtime('wasm')
    const loadWasm = vi.fn<() => Promise<OptimizeRuntime>>(async () => wasm)
    const selector = createRuntimeSelector('wasm', {
      loadNative: vi.fn<() => OptimizeRuntime>(),
      loadWasm,
    })

    expect(await selector.resolve()).toBe(wasm)
    expect(await selector.resolve()).toBe(wasm)
    expect(loadWasm).toHaveBeenCalledOnce()
  })

  it('auto falls back only when the native binding cannot load', async () => {
    const wasm = runtime('wasm')
    const selector = createRuntimeSelector('auto', {
      loadNative() {
        throw new NativeBindingLoadError(new Error('missing artifact'))
      },
      loadWasm: async () => wasm,
    })

    expect((await selector.resolve()).kind).toBe('wasm')
  })

  it('auto preserves non-load native failures', async () => {
    const failure = new Error('native setup failed')
    const selector = createRuntimeSelector('auto', {
      loadNative() {
        throw failure
      },
      loadWasm: vi.fn<() => Promise<OptimizeRuntime>>(),
    })

    await expect(selector.resolve()).rejects.toBe(failure)
  })

  it('does not switch runtime after a selected native operation fails', async () => {
    const native = runtime('native')
    const failure = new Error('invalid font')
    vi.mocked(native.ttfToWoff2).mockRejectedValue(failure)
    const loadWasm = vi.fn<() => Promise<OptimizeRuntime>>()
    const selector = createRuntimeSelector('auto', {
      loadNative: () => native,
      loadWasm,
    })

    const selected = await selector.resolve()
    await expect(selected.ttfToWoff2(new Uint8Array(), {})).rejects.toBe(
      failure,
    )
    expect(loadWasm).not.toHaveBeenCalled()
  })

  it('derives a legacy pipeline mode and rejects conflicts', () => {
    expect(resolvePipelineRuntimeMode(undefined, ['wasm'])).toBe('wasm')
    expect(() => resolvePipelineRuntimeMode('native', ['wasm'])).toThrow(
      'runtime `native` conflicts with WOFF2 fallback `wasm`',
    )
    expect(() =>
      resolvePipelineRuntimeMode(undefined, ['auto', 'wasm']),
    ).toThrow('conflicting WOFF2 fallback modes')
  })
})

describe('WASM optimize runtime adapter', () => {
  it('rejects a function-valued CSS font family before calling WASM', async () => {
    const wasm = wasmRuntime()
    const adapter = await createWasmRuntime(async () => wasm)

    await expect(
      adapter.generateFontFaceCss([], { fontFamily: () => 'Roboto' }),
    ).rejects.toThrow(
      'fontmin-rs WASM generateFontFaceCss does not support option fontFamily',
    )
    expect(wasm.generateFontFaceCss).not.toHaveBeenCalled()
  })

  it('rejects textFile before calling WASM subsetting', async () => {
    const wasm = wasmRuntime()
    const adapter = await createWasmRuntime(async () => wasm)

    await expect(
      adapter.subsetTtf(new Uint8Array(), { textFile: 'glyphs.txt' }),
    ).rejects.toThrow(
      'fontmin-rs WASM subsetTtf does not support option textFile',
    )
    expect(wasm.subsetTtf).not.toHaveBeenCalled()
  })

  it('translates subset aliases at the WASM boundary', async () => {
    const wasm = wasmRuntime()
    const adapter = await createWasmRuntime(async () => wasm)
    const input = new Uint8Array()

    await adapter.subsetTtf(input, {
      hinting: true,
      keepLayout: 'preserve',
    })

    expect(wasm.subsetTtf).toHaveBeenCalledWith(input, {
      layout: 'preserve',
      preserveHinting: true,
    })
  })
})
