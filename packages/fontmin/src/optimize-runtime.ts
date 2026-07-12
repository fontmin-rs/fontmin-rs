import * as native from './native'
import { NativeBindingLoadError, loadNativeBinding } from './native-loader'
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
import { loadWasmRuntime, type WasmRuntime } from './wasm-fallback'

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

export function resolvePipelineRuntimeMode(
  configured: RuntimeMode | undefined,
  fallbacks: readonly NonNullable<Ttf2Woff2Options['fallback']>[],
): RuntimeMode {
  if (fallbacks.includes('js')) {
    throw new Error('WOFF2 fallback `js` is not available in this build')
  }
  const legacy = [...new Set(fallbacks)]
  if (legacy.length > 1) {
    throw new Error(`conflicting WOFF2 fallback modes: ${legacy.join(', ')}`)
  }
  const fallback = legacy[0] as RuntimeMode | undefined
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
  if (requested === 'native') return loaders.loadNative()
  if (requested === 'wasm') return loaders.loadWasm()
  try {
    return loaders.loadNative()
  } catch (error) {
    if (!(error instanceof NativeBindingLoadError)) throw error
    return loaders.loadWasm()
  }
}

const nativeRuntime: OptimizeRuntime = {
  kind: 'native',
  async generateFontFaceCss(sources, options) {
    return native.generateFontFaceCss(sources, options)
  },
  async inspect(input) {
    return native.inspect(input)
  },
  async otfToTtf(input, options) {
    return native.otfToTtf(input, options)
  },
  async subsetTtf(input, options) {
    return native.subsetTtf(input, options)
  },
  async svgFontToTtf(input, options) {
    return native.svgFontToTtf(input, options)
  },
  async svgsToTtf(inputs, options) {
    return native.svgsToTtf(inputs, options)
  },
  async ttfToEot(input, options) {
    return native.ttfToEot(input, options)
  },
  async ttfToSvg(input, options) {
    return native.ttfToSvg(input, options)
  },
  async ttfToWoff(input, options) {
    return native.ttfToWoff(input, options)
  },
  async ttfToWoff2(input, options) {
    return native.ttfToWoff2(input, options)
  },
}

export async function createWasmRuntime(
  loadRuntime: () => Promise<WasmRuntime> = loadWasmRuntime,
): Promise<OptimizeRuntime> {
  const wasm = await loadRuntime()

  return {
    kind: 'wasm',
    async generateFontFaceCss(sources, options) {
      const { fontFamily, ...wasmOptions } = options
      assertWasmOptionSupported(
        'generateFontFaceCss',
        'fontFamily',
        typeof fontFamily === 'function' ? fontFamily : undefined,
      )
      return wasm.generateFontFaceCss(sources, {
        ...wasmOptions,
        ...(typeof fontFamily === 'string' ? { fontFamily } : {}),
      })
    },
    inspect: input => wasm.inspect(input),
    otfToTtf: (input, options) => wasm.otfToTtf(input, options),
    async subsetTtf(input, options) {
      const {
        clone: _clone,
        keepLayout,
        hinting,
        textFile,
        ...wasmOptions
      } = options
      assertWasmOptionSupported('subsetTtf', 'textFile', textFile)
      return wasm.subsetTtf(input, {
        ...wasmOptions,
        ...(keepLayout === undefined ? {} : { layout: keepLayout }),
        ...(options.preserveHinting === undefined && hinting !== undefined
          ? { preserveHinting: hinting }
          : {}),
      })
    },
    svgFontToTtf: (input, options) => wasm.svgFontToTtf(input, options),
    svgsToTtf: (inputs, options) => wasm.svgsToTtf(inputs, options),
    ttfToEot: (input, options) => wasm.ttfToEot(input, options),
    ttfToSvg: (input, options) => wasm.ttfToSvg(input, options),
    ttfToWoff: (input, options) => wasm.ttfToWoff(input, options),
    ttfToWoff2(input, options) {
      const { clone: _clone, fallback: _fallback, ...wasmOptions } = options

      return wasm.ttfToWoff2(input, wasmOptions)
    },
  }
}

function assertWasmOptionSupported(
  operation: string,
  name: string,
  value: unknown,
): void {
  if (value !== undefined) {
    throw new Error(
      `fontmin-rs WASM ${operation} does not support option ${name}`,
    )
  }
}

const defaultRuntimeLoaders: RuntimeLoaders = {
  loadNative() {
    loadNativeBinding()
    return nativeRuntime
  },
  loadWasm: createWasmRuntime,
}
