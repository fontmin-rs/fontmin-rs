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
export type LayoutSubsetMode = 'drop' | 'conservative' | 'preserve'

export interface FontMetadata {
  ascender: number
  descender: number
  familyName?: string
  fullName?: string
  glyphCount: number
  postScriptName?: string
  subfamilyName?: string
  tables: string[]
  unitsPerEm: number
}

export interface FontInfo {
  format: FontFormat
  metadata: FontMetadata
  size: number
}

export interface SubsetOptions {
  basicText?: boolean
  keepNotdef?: boolean
  layout?: LayoutSubsetMode
  preserveHinting?: boolean
  text?: string
  trim?: boolean
  unicodeRanges?: string[]
  unicodes?: number[]
}

export interface WoffOptions {
  compressionLevel?: number
  deflate?: boolean
  metadata?: string
  privateData?: Uint8Array
}

export interface Ttf2Woff2Options {
  quality?: number
}

export interface Ttf2EotOptions {
  version?: number
}

export interface Ttf2SvgOptions {
  fontFamily?: string
}

export interface Otf2TtfOptions {
  preserveHinting?: boolean
  variationCoordinates?: Record<string, number>
}

export interface Svg2TtfOptions {
  hinting?: boolean
  normalize?: boolean
}

export interface SvgIcon {
  contents: string
  name: string
  unicode?: number
}

export interface Svgs2TtfOptions {
  ascent?: number
  descent?: number
  fontName?: string
  normalize?: boolean
  startUnicode?: number
}

export interface CssGlyph {
  name?: string
  unicode: number
}

export interface CssFontSource {
  contents?: Uint8Array
  fileName: string
  format: Exclude<OutputFormat, 'css'>
  glyphs?: CssGlyph[]
  unicodeRanges?: string[]
}

export interface CssOptions {
  asFileName?: boolean
  base64?: boolean
  fontDisplay?: 'auto' | 'block' | 'swap' | 'fallback' | 'optional'
  fontFamily?: string
  fontPath?: string
  glyph?: boolean
  iconPrefix?: string
  local?: boolean
  target?: 'css' | 'scss' | 'less'
  unicodeRanges?: string[]
}
