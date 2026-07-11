export type FontFormat =
  | 'ttf'
  | 'otf'
  | 'woff'
  | 'woff2'
  | 'eot'
  | 'svg'
  | 'css'
  | 'unknown'

export type OutputFormat = 'ttf' | 'woff' | 'woff2' | 'eot' | 'svg' | 'css'
export type AssetFormat = FontFormat | OutputFormat

export type LayoutSubsetMode = 'drop' | 'conservative' | 'preserve'

export interface FontMetadata {
  familyName?: string
  subfamilyName?: string
  fullName?: string
  postScriptName?: string
  glyphCount: number
  unitsPerEm: number
  ascender: number
  descender: number
  tables: string[]
}

export interface FontInfo {
  format: FontFormat
  size: number
  metadata: FontMetadata
}

export type CssFontFamily = string | ((info: FontInfo) => string)

export interface SubsetOptions {
  text?: string
  textFile?: string
  unicodes?: number[]
  basicText?: boolean
  preserveHinting?: boolean
  trim?: boolean
  keepNotdef?: boolean
  keepLayout?: LayoutSubsetMode
  hinting?: boolean
  clone?: boolean
}

export interface WoffOptions {
  deflate?: boolean
  compressionLevel?: number
  metadata?: string
  privateData?: Uint8Array
}

export interface CssFontSource {
  contents?: Uint8Array
  fileName: string
  format: Exclude<OutputFormat, 'css'>
  glyphs?: CssGlyph[]
}

export interface CssGlyph {
  name?: string
  unicode: number
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

export interface Ttf2WoffOptions {
  clone?: boolean
  deflate?: boolean
  compressionLevel?: number
}

export interface Ttf2Woff2Options {
  clone?: boolean
  quality?: number
  fallback?: 'native' | 'wasm' | 'js' | 'auto'
}

export interface Ttf2EotOptions {
  clone?: boolean
  version?: number
}

export interface Otf2TtfOptions {
  clone?: boolean
  preserveHinting?: boolean
  variationCoordinates?: Record<string, number>
}

export interface Ttf2SvgOptions {
  clone?: boolean
  fontFamily?: string
}

export interface Svg2TtfOptions {
  clone?: boolean
  hinting?: boolean
  normalize?: boolean
}

export interface SvgIcon {
  name: string
  contents: string
  unicode?: number
}

export interface Svgs2TtfOptions {
  clone?: boolean
  fontName?: string
  startUnicode?: number
  ascent?: number
  descent?: number
  normalize?: boolean
}

export interface CssOptions {
  fontPath?: string
  base64?: boolean
  glyph?: boolean
  iconPrefix?: string
  fontFamily?: CssFontFamily
  asFileName?: boolean
  local?: boolean
  fontDisplay?: 'auto' | 'block' | 'swap' | 'fallback' | 'optional'
  target?: 'css' | 'scss' | 'less'
}

export interface OutputConfig {
  clone?: boolean
  ext?: string
  fileName?: string
  format: OutputFormat
}

export type ConfigOutput = OutputFormat | OutputConfig

export interface ModernWebOptions
  extends SubsetOptions, Ttf2WoffOptions, Ttf2Woff2Options {
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
}

export interface CacheOptions {
  enabled?: boolean
  dir?: string
}
