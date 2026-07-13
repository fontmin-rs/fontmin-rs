import { describe, expect, it, vi } from 'vitest'
import { NativeBindingLoadError } from '../src/native-loader'
import {
  createRuntimeSelector,
  resolvePipelineRuntimeMode,
} from '../src/optimize-runtime'
import type { OptimizeRuntime } from '../src/optimize-runtime'

function runtime(kind: 'native' | 'wasm'): OptimizeRuntime {
  return {
    kind,
    async generateFontFaceCss() {
      return ''
    },
    async inspect() {
      return {
        format: 'unknown',
        metadata: {
          ascender: 0,
          descender: 0,
          glyphCount: 0,
          tables: [],
          unitsPerEm: 0,
        },
        size: 0,
      }
    },
    async otfToTtf() {
      return new Uint8Array()
    },
    async subsetTtf() {
      return new Uint8Array()
    },
    async svgFontToTtf() {
      return new Uint8Array()
    },
    async svgsToTtf() {
      return new Uint8Array()
    },
    async ttfToEot() {
      return new Uint8Array()
    },
    async ttfToSvg() {
      return ''
    },
    async ttfToWoff() {
      return new Uint8Array()
    },
    ttfToWoff2: vi.fn<OptimizeRuntime['ttfToWoff2']>(),
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

    await expect(selector.resolve()).resolves.toBe(wasm)
    await expect(selector.resolve()).resolves.toBe(wasm)
    expect(loadWasm).toHaveBeenCalledTimes(1)
  })

  it('auto falls back only when the native binding cannot load', async () => {
    const wasm = runtime('wasm')
    const selector = createRuntimeSelector('auto', {
      loadNative() {
        throw new NativeBindingLoadError(new Error('missing artifact'))
      },
      loadWasm: async () => wasm,
    })

    const selected = await selector.resolve()

    expect(selected.kind).toBe('wasm')
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
