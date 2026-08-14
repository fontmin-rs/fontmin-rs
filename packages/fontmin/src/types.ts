import type {
  CssOptions as WasmCssOptions,
  FontFormat,
  FontInfo,
  LayoutSubsetMode,
  Otf2TtfOptions as WasmOtf2TtfOptions,
  OutputFormat,
  SubsetOptions as WasmSubsetOptions,
  Svg2TtfOptions as WasmSvg2TtfOptions,
  Svgs2TtfOptions as WasmSvgs2TtfOptions,
  Ttf2EotOptions as WasmTtf2EotOptions,
  Ttf2SvgOptions as WasmTtf2SvgOptions,
  Ttf2Woff2Options as WasmTtf2Woff2Options,
  VariationSpaceOptions as WasmVariationSpaceOptions,
  WoffOptions,
} from '../../../wasm/fontmin/types'
import type { AutoDeliveryPlanOptions } from './runtime-neutral/auto-delivery'

export type {
  AxisRange,
  AxisSetting,
  CapabilitySupport,
  ColorFontCapabilityReport,
  ColorFontTechnology,
  ColorFontTechnologyCapability,
  CoverageOptions,
  CoverageReport,
  CssFontSource,
  CssGlyph,
  FontCollectionFaceInfo,
  FontCollectionInfo,
  FontCapabilityReport,
  FontFormat,
  FontInfo,
  FontMetadata,
  GidMapping,
  GlyphNameGidMapping,
  InstanceOptions,
  LayoutSubsetMode,
  MissingGlyphPolicy,
  OutputFormat,
  SubsetReport,
  SubsetResult,
  SvgIcon,
  UnicodeGidMapping,
  VariationSpaceOptions,
  WoffOptions,
} from '../../../wasm/fontmin/types'

export type ArtifactFormat = FontFormat | OutputFormat | 'html' | 'json'
export type AssetFormat = FontFormat | OutputFormat
export type RuntimeMode = 'native' | 'wasm' | 'auto'

export type CssFontFamily = string | ((info: FontInfo) => string)

export interface FontminTtfObject {
  glyf?: unknown[]
  name: {
    fontFamily?: string
    [key: string]: string | undefined
  }
  [key: string]: unknown
}

export interface FontminTtfEditor {
  get(): FontminTtfObject
  setGlyf(glyphs: NonNullable<FontminTtfObject['glyf']>): unknown
  setName(name: Partial<FontminTtfObject['name']>): FontminTtfObject['name']
}

export type FontminGlyphTransform = (ttf: FontminTtfEditor) => void

export interface FontminCompatCssInfo {
  base64: boolean | string
  fontFile: string
  fontPath: string
  glyph: boolean
  iconPrefix: string
  local: boolean | string
  [key: string]: unknown
}

export type FontminCompatFontFamily = (
  info: FontminCompatCssInfo,
  ttf: FontminTtfObject,
) => string | null | undefined

export interface SubsetOptions extends Omit<WasmSubsetOptions, 'layout'> {
  textFile?: string
  /** Local HTML/CSS/JS/framework source files or globs used for text discovery. */
  content?: string[]
  /** Drop layout, remap supported data, or reject known contextual loss. */
  keepLayout?: LayoutSubsetMode
  /** Fontmin-compatible alias for preserveHinting. */
  hinting?: boolean
  clone?: boolean
}

export type {
  WebTextDiscoveryOptions,
  WebTextDiscoveryResult,
} from './web-text'

export interface FontminCompatGlyphOptions extends SubsetOptions {
  use?: FontminGlyphTransform
}

export interface DeliverySlice {
  name: string
  unicodeRanges: string[]
}

export interface AutoDeliverySubsetOptions {
  dropTables?: string[]
  keepLayout?: LayoutSubsetMode
  keepNotdef?: boolean
  layoutFeatures?: string[]
  layoutLanguages?: string[]
  layoutScripts?: string[]
  nameIds?: number[]
  nameLanguages?: number[]
  passThroughTables?: string[]
  preserveHinting?: boolean
  retainGlyphNames?: boolean
  retainLegacyCmap?: boolean
  retainSymbolCmap?: boolean
}

export interface AutoDeliveryOptions extends AutoDeliveryPlanOptions {
  /** Format used to enforce targetBytes. Defaults to WOFF2. */
  measureFormat?: 'ttf' | 'woff' | 'woff2'
  subset?: AutoDeliverySubsetOptions
  woff2Quality?: number
  woffCompressionLevel?: number
}

export interface FontAsset {
  path: string
  contents: Uint8Array
  format: ArtifactFormat
  sourceFormat: FontFormat
  meta: Record<string, unknown>
}

export interface WebDeliveryOptions {
  /** URL prefix used by generated CSS and preload markup. */
  basePath?: string
  cssFile?: string
  /** Emit the original full font as a dynamic-content fallback. */
  fallback?: boolean
  fontDisplay?: NonNullable<CssOptions['fontDisplay']>
  fontFamily: string
  /** Add a deterministic content hash before each delivered font extension. */
  hashFileNames?: boolean
  /** Number of SHA-256 hexadecimal characters used in hashed file names. */
  hashLength?: number
  manifestFile?: string
  /** Preload the first preferred subset per source, every subset, or none. */
  preload?: 'first' | 'all' | false
  preloadFile?: string
  /** Selector receiving the generated subset/fallback family stack. */
  selector?: string
  /** Emit a standalone delivery preview page, or false to disable it. */
  testHtmlFile?: string | false
  /** Text rendered by the optional delivery preview page. */
  testText?: string
}

export interface WebDeliveryManifestAsset {
  format: ArtifactFormat
  path: string
  preload: boolean
  sha256: string
  size: number
  unicodeRanges: string[]
}

export interface WebDeliveryManifestSource {
  fallback?: WebDeliveryManifestAsset
  id: string
  sourceFormat: FontFormat
  sourcePath: string
  subsets: WebDeliveryManifestAsset[]
}

export interface WebDeliveryManifest {
  css: string
  fontFamily: string
  preload: string
  schemaVersion: 1
  sources: WebDeliveryManifestSource[]
  summary: WebDeliveryManifestSummary
  testHtml?: string
}

export interface WebDeliveryManifestSummary {
  codePointCount: number
  fallbackBytes: number
  requestCount: number
  sourceBytes: number
  subsetBytes: number
  subsetCount: number
}

export interface PluginDiagnostic {
  level: 'warn'
  message: string
}

export interface PluginContext {
  cwd: string
  diagnostics: PluginDiagnostic[]
  emitFile(asset: FontAsset): void
  readFile(path: string): Promise<Buffer>
  resolve(path: string): string
  warn(message: string | Error): void
  writeFile(path: string, contents: string | Uint8Array): Promise<void>
}

export type MaybePromise<T> = T | Promise<T>

export interface FontminPlugin {
  name: string
  enforce?: 'pre' | 'post'
  native?: {
    kind: 'builtin'
    name: string
    options: Record<string, unknown>
  }
  buildStart?(ctx: PluginContext): MaybePromise<void>
  transform?(
    asset: FontAsset,
    ctx: PluginContext,
  ): MaybePromise<FontAsset | FontAsset[] | null | undefined>
  generateBundle?(assets: FontAsset[], ctx: PluginContext): MaybePromise<void>
  buildEnd?(ctx: PluginContext): MaybePromise<void>
}

export interface Ttf2WoffOptions extends Pick<
  WoffOptions,
  'compressionLevel' | 'deflate'
> {
  clone?: boolean
}

export interface Ttf2Woff2Options extends WasmTtf2Woff2Options {
  clone?: boolean
  fallback?: 'native' | 'wasm' | 'js' | 'auto'
}

export interface Ttf2EotOptions extends WasmTtf2EotOptions {
  clone?: boolean
}

export interface Otf2TtfOptions extends WasmOtf2TtfOptions {
  clone?: boolean
}

export interface Ttf2SvgOptions extends WasmTtf2SvgOptions {
  clone?: boolean
}

export interface Svg2TtfOptions extends WasmSvg2TtfOptions {
  clone?: boolean
}

export interface FontminOutputFile {
  path: string
  relative?: string
}

export type Svgs2TtfOutput = string | FontminOutputFile

export interface Svgs2TtfOptions extends WasmSvgs2TtfOptions {
  clone?: boolean
}

export interface CssOptions extends Omit<WasmCssOptions, 'fontFamily'> {
  fontFamily?: CssFontFamily
}

export interface FontminCompatCssOptions extends Omit<
  CssOptions,
  'fontFamily'
> {
  fontFamily?: string | FontminCompatFontFamily
}

export interface OutputConfig {
  clone?: boolean
  ext?: string
  fileName?: string
  format: OutputFormat
}

export type ConfigOutput = OutputFormat | OutputConfig

export interface ModernWebOptions
  extends SubsetOptions, Otf2TtfOptions, Ttf2WoffOptions, Ttf2Woff2Options {
  fontFamily?: CssOptions['fontFamily']
  fontPath?: string
  local?: boolean
  fontDisplay?: CssOptions['fontDisplay']
  variationAxes?: WasmVariationSpaceOptions['axes']
  downgradeCff2?: boolean
}

export interface VariationSpacePluginOptions extends WasmVariationSpaceOptions {
  clone?: boolean
}

export interface FontminCompatPresetOptions
  extends
    SubsetOptions,
    Ttf2WoffOptions,
    Ttf2Woff2Options,
    Otf2TtfOptions,
    Ttf2EotOptions,
    Ttf2SvgOptions {
  asFileName?: boolean
  base64?: boolean
  cssGlyph?: boolean
  deflateWoff?: boolean
  fontDisplay?: CssOptions['fontDisplay']
  fontPath?: string
  glyph?: boolean
  glyphCss?: boolean
  iconPrefix?: string
  local?: boolean
  target?: CssOptions['target']
}

export interface FontminConfig {
  $schema?: string
  cwd?: string
  input?: (string | Uint8Array)[]
  outDir?: string
  clean?: boolean
  preserveOriginal?: boolean
  cache?: boolean | CacheOptions
  autoDelivery?: AutoDeliveryOptions
  subset?: SubsetOptions
  outputs?: ConfigOutput[]
  css?: CssOptions
  plugins?: FontminPlugin[]
  runtime?: RuntimeMode
  webDelivery?: WebDeliveryOptions
}

export interface CacheOptions {
  enabled?: boolean
  dir?: string
}
