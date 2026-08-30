import * as native from './native'
import { NativeBindingLoadError, loadNativeBinding } from './native-loader'
import type {
  CssFontSource,
  CssOptions,
  FontInfo,
  InstanceOptions,
  Otf2TtfOptions,
  RuntimeMode,
  SubsetOptions,
  Svg2TtfOptions,
  SvgIcon,
  Svgs2TtfOptions,
  Ttf2EotOptions,
  Ttf2SvgOptions,
  Ttf2Woff2Options,
  VariationSpaceOptions,
  WoffOptions,
} from './types'
import { loadWasmRuntime } from './wasm-fallback'
import type { WasmRuntime } from './wasm-fallback'

export interface OptimizeRuntime {
  readonly kind: Exclude<RuntimeMode, 'auto'>
  eotToTtf(input: Uint8Array): Promise<Uint8Array>
  generateFontFaceCss(
    sources: CssFontSource[],
    options: CssOptions,
  ): Promise<string>
  inspect(input: Uint8Array): Promise<FontInfo>
  instantiateFont(
    input: Uint8Array,
    options: InstanceOptions,
  ): Promise<Uint8Array>
  otfToTtf(input: Uint8Array, options: Otf2TtfOptions): Promise<Uint8Array>
  reduceVariationSpace(
    input: Uint8Array,
    options: VariationSpaceOptions,
  ): Promise<Uint8Array>
  subsetTtf(input: Uint8Array, options: SubsetOptions): Promise<Uint8Array>
  svgFontToTtf(input: string, options: Svg2TtfOptions): Promise<Uint8Array>
  svgsToTtf(inputs: SvgIcon[], options: Svgs2TtfOptions): Promise<Uint8Array>
  ttfToEot(input: Uint8Array, options: Ttf2EotOptions): Promise<Uint8Array>
  ttfToSvg(input: Uint8Array, options: Ttf2SvgOptions): Promise<string>
  ttfToWoff(input: Uint8Array, options: WoffOptions): Promise<Uint8Array>
  ttfToWoff2(input: Uint8Array, options: Ttf2Woff2Options): Promise<Uint8Array>
  woff2ToTtf(input: Uint8Array): Promise<Uint8Array>
  woffToTtf(input: Uint8Array): Promise<Uint8Array>
}

export interface RuntimeSelector {
  readonly requested: RuntimeMode
  resolve(): Promise<OptimizeRuntime>
}

export function createBoundedRuntimeSelector(
  selector: RuntimeSelector,
  concurrency: number,
): RuntimeSelector {
  const limiter = createOperationLimiter(concurrency)
  let selected: Promise<OptimizeRuntime> | undefined

  return {
    requested: selector.requested,
    async resolve() {
      selected ??= (async () => {
        const runtime = await selector.resolve()

        return {
          kind: runtime.kind,
          eotToTtf: input => limiter.run(() => runtime.eotToTtf(input)),
          generateFontFaceCss: (sources, options) =>
            limiter.run(() => runtime.generateFontFaceCss(sources, options)),
          inspect: input => limiter.run(() => runtime.inspect(input)),
          instantiateFont: (input, options) =>
            limiter.run(() => runtime.instantiateFont(input, options)),
          otfToTtf: (input, options) =>
            limiter.run(() => runtime.otfToTtf(input, options)),
          reduceVariationSpace: (input, options) =>
            limiter.run(() => runtime.reduceVariationSpace(input, options)),
          subsetTtf: (input, options) =>
            limiter.run(() => runtime.subsetTtf(input, options)),
          svgFontToTtf: (input, options) =>
            limiter.run(() => runtime.svgFontToTtf(input, options)),
          svgsToTtf: (inputs, options) =>
            limiter.run(() => runtime.svgsToTtf(inputs, options)),
          ttfToEot: (input, options) =>
            limiter.run(() => runtime.ttfToEot(input, options)),
          ttfToSvg: (input, options) =>
            limiter.run(() => runtime.ttfToSvg(input, options)),
          ttfToWoff: (input, options) =>
            limiter.run(() => runtime.ttfToWoff(input, options)),
          ttfToWoff2: (input, options) =>
            limiter.run(() => runtime.ttfToWoff2(input, options)),
          woff2ToTtf: input => limiter.run(() => runtime.woff2ToTtf(input)),
          woffToTtf: input => limiter.run(() => runtime.woffToTtf(input)),
        }
      })()

      return selected
    },
  }
}

function createOperationLimiter(concurrency: number): {
  run<T>(operation: () => Promise<T>): Promise<T>
} {
  if (!Number.isInteger(concurrency) || concurrency < 1) {
    throw new TypeError('parallel thread count must be a positive integer')
  }

  let active = 0
  const waiting: (() => void)[] = []

  return {
    async run<T>(operation: () => Promise<T>): Promise<T> {
      if (active >= concurrency) {
        await new Promise<void>(resolve => {
          waiting.push(resolve)
        })
      }
      active += 1

      try {
        return await operation()
      } finally {
        active -= 1
        waiting.shift()?.()
      }
    },
  }
}

interface RuntimeLoaders {
  loadNative(): OptimizeRuntime
  loadWasm(): Promise<OptimizeRuntime>
}

const defaultRuntimeLoaders: RuntimeLoaders = {
  loadNative: loadNativeRuntime,
  loadWasm: createWasmRuntime,
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

const nativeRuntime: OptimizeRuntime = {
  kind: 'native',
  eotToTtf: native.eotToTtfAsync,
  generateFontFaceCss: native.generateFontFaceCssAsync,
  inspect: native.inspectAsync,
  instantiateFont: native.instantiateFontAsync,
  otfToTtf: native.otfToTtfAsync,
  reduceVariationSpace: native.reduceVariationSpaceAsync,
  subsetTtf: native.subsetTtfAsync,
  svgFontToTtf: native.svgFontToTtfAsync,
  svgsToTtf: native.svgsToTtfAsync,
  ttfToEot: native.ttfToEotAsync,
  ttfToSvg: native.ttfToSvgAsync,
  ttfToWoff: native.ttfToWoffAsync,
  ttfToWoff2: native.ttfToWoff2Async,
  woff2ToTtf: native.woff2ToTtfAsync,
  woffToTtf: native.woffToTtfAsync,
}

function loadNativeRuntime(): OptimizeRuntime {
  loadNativeBinding()
  return nativeRuntime
}

export async function createWasmRuntime(
  loadRuntime: () => Promise<WasmRuntime> = loadWasmRuntime,
): Promise<OptimizeRuntime> {
  const wasm = await loadRuntime()

  return {
    kind: 'wasm',
    eotToTtf: input => runWasmOperation('eotToTtf', () => wasm.eotToTtf(input)),
    async generateFontFaceCss(sources, options) {
      const {
        ext: _ext,
        fileName: _fileName,
        fontFamily,
        ...wasmOptions
      } = options as CssOptions & { ext?: string; fileName?: string }
      assertWasmOptionSupported(
        'generateFontFaceCss',
        'fontFamily',
        typeof fontFamily === 'function' ? fontFamily : undefined,
      )
      return runWasmOperation('generateFontFaceCss', () =>
        wasm.generateFontFaceCss(sources, {
          ...wasmOptions,
          ...(typeof fontFamily === 'string' ? { fontFamily } : {}),
        }),
      )
    },
    inspect: input => runWasmOperation('inspect', () => wasm.inspect(input)),
    instantiateFont: (input, options) =>
      runWasmOperation('instantiateFont', () =>
        wasm.instantiateFont(input, options),
      ),
    otfToTtf: (input, options) =>
      runWasmOperation('otfToTtf', () => wasm.otfToTtf(input, options)),
    reduceVariationSpace: (input, options) =>
      runWasmOperation('reduceVariationSpace', () =>
        wasm.reduceVariationSpace(input, options),
      ),
    async subsetTtf(input, options) {
      const {
        clone: _clone,
        keepLayout,
        hinting,
        textFile,
        ...wasmOptions
      } = options
      assertWasmOptionSupported('subsetTtf', 'textFile', textFile)
      return runWasmOperation('subsetTtf', () =>
        wasm.subsetTtf(input, {
          ...wasmOptions,
          ...(keepLayout === undefined ? {} : { layout: keepLayout }),
          ...(options.preserveHinting === undefined && hinting !== undefined
            ? { preserveHinting: hinting }
            : {}),
        }),
      )
    },
    svgFontToTtf: (input, options) =>
      runWasmOperation('svgFontToTtf', () => wasm.svgFontToTtf(input, options)),
    svgsToTtf: (inputs, options) =>
      runWasmOperation('svgsToTtf', () => wasm.svgsToTtf(inputs, options)),
    ttfToEot: (input, options) =>
      runWasmOperation('ttfToEot', () => wasm.ttfToEot(input, options)),
    ttfToSvg: (input, options) =>
      runWasmOperation('ttfToSvg', () => wasm.ttfToSvg(input, options)),
    ttfToWoff: (input, options) =>
      runWasmOperation('ttfToWoff', () => wasm.ttfToWoff(input, options)),
    ttfToWoff2(input, options) {
      const { clone: _clone, fallback: _fallback, ...wasmOptions } = options

      return runWasmOperation('ttfToWoff2', () =>
        wasm.ttfToWoff2(input, wasmOptions),
      )
    },
    woff2ToTtf: input =>
      runWasmOperation('woff2ToTtf', () => wasm.woff2ToTtf(input)),
    woffToTtf: input =>
      runWasmOperation('woffToTtf', () => wasm.woffToTtf(input)),
  }
}

async function runWasmOperation<T>(
  operation: string,
  run: () => Promise<T>,
): Promise<T> {
  try {
    return await run()
  } catch (error) {
    if (error instanceof Error) {
      throw error
    }

    throw new Error(
      typeof error === 'string'
        ? error
        : `fontmin-rs WASM runtime failed during ${operation}`,
      { cause: error },
    )
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
