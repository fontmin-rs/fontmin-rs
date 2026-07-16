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
  WoffOptions,
} from '../../../wasm/fontmin/types'

export type {
  CssFontSource,
  CssGlyph,
  FontFormat,
  FontInfo,
  FontMetadata,
  LayoutSubsetMode,
  OutputFormat,
  SvgIcon,
  WoffOptions,
} from '../../../wasm/fontmin/types'

export type AssetFormat = FontFormat | OutputFormat
export type RuntimeMode = 'native' | 'wasm' | 'auto'

export type CssFontFamily = string | ((info: FontInfo) => string)

export interface SubsetOptions extends Omit<WasmSubsetOptions, 'layout'> {
  textFile?: string
  keepLayout?: LayoutSubsetMode
  hinting?: boolean
  clone?: boolean
}

export interface DeliverySlice {
  name: string
  unicodeRanges: string[]
}

export interface FontAsset {
  path: string
  contents: Uint8Array
  format: AssetFormat
  sourceFormat: FontFormat
  meta: Record<string, unknown>
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

export interface Svgs2TtfOptions extends WasmSvgs2TtfOptions {
  clone?: boolean
}

export interface CssOptions extends Omit<WasmCssOptions, 'fontFamily'> {
  fontFamily?: CssFontFamily
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
  cwd?: string
  input?: (string | Uint8Array)[]
  outDir?: string
  clean?: boolean
  preserveOriginal?: boolean
  cache?: boolean | CacheOptions
  subset?: SubsetOptions
  outputs?: ConfigOutput[]
  css?: CssOptions
  plugins?: FontminPlugin[]
  runtime?: RuntimeMode
}

export interface CacheOptions {
  enabled?: boolean
  dir?: string
}
