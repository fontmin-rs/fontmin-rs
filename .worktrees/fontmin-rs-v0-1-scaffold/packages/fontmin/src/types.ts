export type FontFormat =
  | 'ttf'
  | 'otf'
  | 'woff'
  | 'woff2'
  | 'eot'
  | 'svg'
  | 'unknown'

export type OutputFormat = 'ttf' | 'woff' | 'woff2' | 'eot' | 'svg' | 'css'

export type LayoutSubsetMode = 'drop' | 'conservative' | 'preserve'

export interface SubsetOptions {
  text?: string
  unicodes?: number[]
  basicText?: boolean
  preserveHinting?: boolean
  trim?: boolean
  keepNotdef?: boolean
  keepLayout?: LayoutSubsetMode
  hinting?: boolean
  clone?: boolean
}

export interface FontAsset {
  path: string
  contents: Uint8Array
  format: FontFormat
  sourceFormat: FontFormat
  meta: Record<string, unknown>
}

export interface PluginContext {
  cwd: string
  emitFile(asset: FontAsset): void
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

export interface CssOptions {
  fontPath?: string
  base64?: boolean
  glyph?: boolean
  iconPrefix?: string
  fontFamily?: string
  asFileName?: boolean
  local?: boolean
  fontDisplay?: 'auto' | 'block' | 'swap' | 'fallback' | 'optional'
  target?: 'css' | 'scss' | 'less'
}

export interface FontminConfig {
  cwd?: string
  input?: (string | Uint8Array)[]
  outDir?: string
  clean?: boolean
  preserveOriginal?: boolean
  subset?: SubsetOptions
  plugins?: FontminPlugin[]
}
