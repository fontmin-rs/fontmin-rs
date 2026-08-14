import { missingGlyphWarning } from '../../../packages/fontmin/src/runtime-neutral/optimize-policy'
import type {
  CoverageOptions,
  CoverageReport,
  CssFontSource,
  CssOptions,
  FontCapabilityReport,
  FontInfo,
  FontCollectionInfo,
  InstanceOptions,
  Otf2TtfOptions,
  SubsetOptions,
  SubsetPlan,
  SubsetResult,
  Svg2TtfOptions,
  SvgIcon,
  Svgs2TtfOptions,
  Ttf2EotOptions,
  Ttf2SvgOptions,
  Ttf2Woff2Options,
  VariationSpaceOptions,
  WoffOptions,
} from '../types'
import { withFontminDiagnostics } from './diagnostics'
import { getWasmModule } from './runtime'

function bytes(value: unknown): Uint8Array {
  return new Uint8Array(value as ArrayLike<number>)
}

async function binary(
  operation: string,
  input: Uint8Array,
  options: object = {},
): Promise<Uint8Array> {
  const wasm = await getWasmModule()
  return withFontminDiagnostics(() =>
    bytes(wasm.transform(operation, input, options)),
  )
}

export async function subsetTtf(
  input: Uint8Array,
  options: SubsetOptions = {},
): Promise<Uint8Array> {
  if (
    (options.missingGlyphs ?? 'warn') === 'warn' &&
    hasUnicodeSelection(options)
  ) {
    const report = await analyzeCoverage(input, coverageOptions(options))
    const warning = missingGlyphWarning(report)

    if (warning !== undefined) {
      console.warn(warning)
    }
  }

  return binary('subsetTtf', input, options)
}

export async function subsetTtfWithReport(
  input: Uint8Array,
  options: SubsetOptions = {},
): Promise<SubsetResult> {
  const wasm = await getWasmModule()
  const result = withFontminDiagnostics(
    () => wasm.transform('subsetTtfWithReport', input, options) as SubsetResult,
  )

  return {
    data: bytes(result.data),
    report: {
      ...result.report,
      newToOld: result.report.newToOld.map(gid => gid ?? null),
    },
  }
}

export async function createTtfSubsetPlan(
  input: Uint8Array,
  options: SubsetOptions = {},
): Promise<SubsetPlan> {
  const wasm = await getWasmModule()
  return withFontminDiagnostics(
    () => wasm.transform('createTtfSubsetPlan', input, options) as SubsetPlan,
  )
}

export async function subsetTtfWithPlan(
  input: Uint8Array,
  plan: SubsetPlan,
): Promise<SubsetResult> {
  const wasm = await getWasmModule()
  const result = withFontminDiagnostics(
    () => wasm.transform('subsetTtfWithPlan', input, plan) as SubsetResult,
  )

  return {
    data: bytes(result.data),
    report: {
      ...result.report,
      newToOld: result.report.newToOld.map(gid => gid ?? null),
    },
  }
}

function hasUnicodeSelection(options: SubsetOptions): boolean {
  return (
    options.basicText === true ||
    (options.text?.length ?? 0) > 0 ||
    (options.unicodes?.length ?? 0) > 0 ||
    (options.unicodeRanges?.length ?? 0) > 0
  )
}

export async function analyzeCoverage(
  input: Uint8Array,
  options: CoverageOptions = {},
): Promise<CoverageReport> {
  const wasm = await getWasmModule()
  return withFontminDiagnostics(
    () => wasm.transform('analyzeCoverage', input, options) as CoverageReport,
  )
}

export async function instantiateFont(
  input: Uint8Array,
  options: InstanceOptions = {},
): Promise<Uint8Array> {
  return binary('instantiateFont', input, options)
}

export async function reduceVariationSpace(
  input: Uint8Array,
  options: VariationSpaceOptions,
): Promise<Uint8Array> {
  return binary('reduceVariationSpace', input, options)
}

export async function ttfToWoff(
  input: Uint8Array,
  options: WoffOptions = {},
): Promise<Uint8Array> {
  return binary('ttfToWoff', input, options)
}

export async function woffToTtf(input: Uint8Array): Promise<Uint8Array> {
  return binary('woffToTtf', input)
}

export async function ttfToWoff2(
  input: Uint8Array,
  options: Ttf2Woff2Options = {},
): Promise<Uint8Array> {
  return binary('ttfToWoff2', input, options)
}

export async function woff2ToTtf(input: Uint8Array): Promise<Uint8Array> {
  return binary('woff2ToTtf', input)
}

export async function validateWoff2(input: Uint8Array): Promise<void> {
  const wasm = await getWasmModule()
  withFontminDiagnostics(() => wasm.transform('validateWoff2', input, {}))
}

export async function ttfToEot(
  input: Uint8Array,
  options: Ttf2EotOptions = {},
): Promise<Uint8Array> {
  return binary('ttfToEot', input, options)
}

export async function eotToTtf(input: Uint8Array): Promise<Uint8Array> {
  return binary('eotToTtf', input)
}

export async function ttfToSvg(
  input: Uint8Array,
  options: Ttf2SvgOptions = {},
): Promise<string> {
  const wasm = await getWasmModule()
  return withFontminDiagnostics(
    () => wasm.transform('ttfToSvg', input, options) as string,
  )
}

export async function svgFontToTtf(
  input: string,
  options: Svg2TtfOptions = {},
): Promise<Uint8Array> {
  const wasm = await getWasmModule()
  return withFontminDiagnostics(() =>
    bytes(wasm.transform_text('svgFontToTtf', input, options)),
  )
}

export async function svgsToTtf(
  inputs: SvgIcon[],
  options: Svgs2TtfOptions = {},
): Promise<Uint8Array> {
  const wasm = await getWasmModule()
  return withFontminDiagnostics(() =>
    bytes(wasm.transform_icons(inputs, options)),
  )
}

export async function otfToTtf(
  input: Uint8Array,
  options: Otf2TtfOptions = {},
): Promise<Uint8Array> {
  return binary('otfToTtf', input, options)
}

export async function inspect(input: Uint8Array): Promise<FontInfo> {
  const wasm = await getWasmModule()
  return withFontminDiagnostics(
    () => wasm.transform('inspect', input, {}) as FontInfo,
  )
}

export async function inspectCapabilities(
  input: Uint8Array,
): Promise<FontCapabilityReport> {
  const wasm = await getWasmModule()
  const report = withFontminDiagnostics(
    () =>
      wasm.transform('inspectCapabilities', input, {}) as FontCapabilityReport,
  )

  if (report.color.subsetSupport === undefined) {
    const { subsetSupport: _subsetSupport, ...color } = report.color

    return { ...report, color }
  }

  return report
}

export async function inspectCollection(
  input: Uint8Array,
): Promise<FontCollectionInfo> {
  const wasm = await getWasmModule()
  return withFontminDiagnostics(
    () => wasm.transform('inspectCollection', input, {}) as FontCollectionInfo,
  )
}

export async function extractCollectionFace(
  input: Uint8Array,
  faceIndex: number,
): Promise<Uint8Array> {
  return binary('extractCollectionFace', input, { faceIndex })
}

export async function generateFontFaceCss(
  sources: CssFontSource[],
  options: CssOptions = {},
): Promise<string> {
  const wasm = await getWasmModule()
  return withFontminDiagnostics(
    () => wasm.generate_css(sources, options) as string,
  )
}

function coverageOptions(options: SubsetOptions): CoverageOptions {
  const coverage: CoverageOptions = {}

  if (options.basicText !== undefined) {
    coverage.basicText = options.basicText
  }
  if (options.text !== undefined) {
    coverage.text = options.text
  }
  if (options.unicodeRanges !== undefined) {
    coverage.unicodeRanges = options.unicodeRanges
  }
  if (options.unicodes !== undefined) {
    coverage.unicodes = options.unicodes
  }

  return coverage
}
