import {
  generateFontFaceCss,
  inspect,
  otfToTtf,
  subsetTtf,
  svgFontToTtf,
  svgsToTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
} from './native'
import { loadNativeBinding, NativeBindingLoadError } from './native-loader'
import type {
  CssFontSource,
  CssOptions,
  FontInfo,
  Otf2TtfOptions,
  RuntimeMode,
  SubsetOptions,
  Svg2TtfOptions,
  SvgIcon,
  Svgs2TtfOptions,
  Ttf2EotOptions,
  Ttf2SvgOptions,
  Ttf2Woff2Options,
  WoffOptions,
} from './types'
import { loadWasmRuntime } from './wasm-fallback'

export interface OptimizeRuntime {
  readonly kind: Exclude<RuntimeMode, 'auto'>
  generateFontFaceCss(
    sources: CssFontSource[],
    options: CssOptions,
  ): Promise<string>
  inspect(input: Uint8Array): Promise<FontInfo>
  otfToTtf(input: Uint8Array, options: Otf2TtfOptions): Promise<Uint8Array>
  subsetTtf(input: Uint8Array, options: SubsetOptions): Promise<Uint8Array>
  svgFontToTtf(input: string, options: Svg2TtfOptions): Promise<Uint8Array>
  svgsToTtf(inputs: SvgIcon[], options: Svgs2TtfOptions): Promise<Uint8Array>
  ttfToEot(input: Uint8Array, options: Ttf2EotOptions): Promise<Uint8Array>
  ttfToSvg(input: Uint8Array, options: Ttf2SvgOptions): Promise<string>
  ttfToWoff(input: Uint8Array, options: WoffOptions): Promise<Uint8Array>
  ttfToWoff2(input: Uint8Array, options: Ttf2Woff2Options): Promise<Uint8Array>
}

export interface RuntimeSelector {
  readonly requested: RuntimeMode
  resolve(): Promise<OptimizeRuntime>
}

interface RuntimeLoaders {
  loadNative(): OptimizeRuntime
  loadWasm(): Promise<OptimizeRuntime>
}

const nativeRuntime: OptimizeRuntime = {
  kind: 'native',
  async generateFontFaceCss(sources, options) {
    return generateFontFaceCss(sources, options)
  },
  async inspect(input) {
    return inspect(input)
  },
  async otfToTtf(input, options) {
    return otfToTtf(input, options)
  },
  async subsetTtf(input, options) {
    return subsetTtf(input, options)
  },
  async svgFontToTtf(input, options) {
    return svgFontToTtf(input, options)
  },
  async svgsToTtf(inputs, options) {
    return svgsToTtf(inputs, options)
  },
  async ttfToEot(input, options) {
    return ttfToEot(input, options)
  },
  async ttfToSvg(input, options) {
    return ttfToSvg(input, options)
  },
  async ttfToWoff(input, options) {
    return ttfToWoff(input, options)
  },
  async ttfToWoff2(input, options) {
    return ttfToWoff2(input, { ...options, fallback: 'native' })
  },
}

const defaultRuntimeLoaders: RuntimeLoaders = {
  loadNative() {
    loadNativeBinding()
    return nativeRuntime
  },
  async loadWasm() {
    const wasm = await loadWasmRuntime()

    return {
      kind: 'wasm',
      async generateFontFaceCss(sources, options) {
        const { fontFamily, ...rest } = options

        if (typeof fontFamily === 'function') {
          throw unsupportedWasmOption('generateFontFaceCss', 'fontFamily')
        }

        return runWasmOperation('generateFontFaceCss', () =>
          wasm.generateFontFaceCss(sources, {
            ...rest,
            ...(fontFamily === undefined ? {} : { fontFamily }),
          }),
        )
      },
      inspect(input) {
        return runWasmOperation('inspect', () => wasm.inspect(input))
      },
      otfToTtf(input, options) {
        return runWasmOperation('otfToTtf', () => wasm.otfToTtf(input, options))
      },
      subsetTtf(input, options) {
        if (options.textFile !== undefined) {
          throw unsupportedWasmOption('subsetTtf', 'textFile')
        }

        return runWasmOperation('subsetTtf', () =>
          wasm.subsetTtf(input, wasmSubsetOptions(options)),
        )
      },
      svgFontToTtf(input, options) {
        return runWasmOperation('svgFontToTtf', () =>
          wasm.svgFontToTtf(input, options),
        )
      },
      svgsToTtf(inputs, options) {
        return runWasmOperation('svgsToTtf', () =>
          wasm.svgsToTtf(inputs, options),
        )
      },
      ttfToEot(input, options) {
        return runWasmOperation('ttfToEot', () => wasm.ttfToEot(input, options))
      },
      ttfToSvg(input, options) {
        return runWasmOperation('ttfToSvg', () => wasm.ttfToSvg(input, options))
      },
      ttfToWoff(input, options) {
        return runWasmOperation('ttfToWoff', () =>
          wasm.ttfToWoff(input, options),
        )
      },
      ttfToWoff2(input, options) {
        const { clone: _clone, fallback: _fallback, ...wasmOptions } = options

        return runWasmOperation('ttfToWoff2', () =>
          wasm.ttfToWoff2(input, wasmOptions),
        )
      },
    }
  },
}

export function resolvePipelineRuntimeMode(
  configured: RuntimeMode | undefined,
  fallbacks: readonly NonNullable<Ttf2Woff2Options['fallback']>[],
): RuntimeMode {
  if (fallbacks.includes('js')) {
    throw new Error('WOFF2 fallback `js` is not available in this build')
  }

  const legacy = [...new Set(fallbacks.filter(fallback => fallback !== 'js'))]

  if (legacy.length > 1) {
    throw new Error(`conflicting WOFF2 fallback modes: ${legacy.join(', ')}`)
  }

  const fallback = legacy[0]

  if (
    configured !== undefined &&
    fallback !== undefined &&
    configured !== fallback
  ) {
    throw new Error(
      `runtime \`${configured}\` conflicts with WOFF2 fallback \`${fallback}\``,
    )
  }

  return configured ?? fallback ?? 'native'
}

export function createRuntimeSelector(
  requested: RuntimeMode,
  loaders: RuntimeLoaders = defaultRuntimeLoaders,
): RuntimeSelector {
  let selected: Promise<OptimizeRuntime> | undefined

  return {
    requested,
    resolve() {
      selected ??= selectRuntime(requested, loaders)

      return selected
    },
  }
}

async function selectRuntime(
  requested: RuntimeMode,
  loaders: RuntimeLoaders,
): Promise<OptimizeRuntime> {
  if (requested === 'native') {
    return loaders.loadNative()
  }
  if (requested === 'wasm') {
    return loaders.loadWasm()
  }

  try {
    return loaders.loadNative()
  } catch (error) {
    if (!(error instanceof NativeBindingLoadError)) {
      throw error
    }

    return loaders.loadWasm()
  }
}

export async function runWasmOperation<T>(
  operation: string,
  execute: () => T | PromiseLike<T>,
): Promise<T> {
  try {
    return await execute()
  } catch (error) {
    throw new Error(`fontmin-rs WASM runtime failed during ${operation}`, {
      cause: error,
    })
  }
}

function unsupportedWasmOption(operation: string, option: string): Error {
  return new Error(
    `fontmin-rs WASM ${operation} does not support option ${option}`,
  )
}

function wasmSubsetOptions(options: SubsetOptions): Record<string, unknown> {
  const wasmOptions: Record<string, unknown> = {}
  const preserveHinting = options.preserveHinting ?? options.hinting

  assignDefined(wasmOptions, 'text', options.text)
  assignDefined(wasmOptions, 'unicodes', options.unicodes)
  assignDefined(wasmOptions, 'unicodeRanges', options.unicodeRanges)
  assignDefined(wasmOptions, 'basicText', options.basicText)
  assignDefined(wasmOptions, 'preserveHinting', preserveHinting)
  assignDefined(wasmOptions, 'trim', options.trim)
  assignDefined(wasmOptions, 'keepNotdef', options.keepNotdef)
  assignDefined(wasmOptions, 'layout', options.keepLayout)

  return wasmOptions
}

function assignDefined(
  target: Record<string, unknown>,
  key: string,
  value: unknown,
): void {
  if (value !== undefined) {
    target[key] = value
  }
}
