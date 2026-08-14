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
export type MissingGlyphPolicy = 'ignore' | 'warn' | 'error'

export interface CoverageOptions {
  basicText?: boolean
  text?: string
  unicodeRanges?: string[]
  unicodes?: number[]
}

export interface CoverageReport {
  coveragePercent: number
  missing: number[]
  requested: number[]
  supported: number[]
}

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

export interface SubsetOptions extends CoverageOptions {
  /** Original glyph IDs to retain in addition to Unicode selection. */
  gids?: number[]
  /** PostScript glyph names to retain in addition to other selectors. */
  glyphNames?: string[]
  /** Retain the original glyph-zero outline; false emits the required empty glyph-zero slot. */
  keepNotdef?: boolean
  /** Preserve original glyph IDs and leave null entries for empty mapping slots. */
  retainGids?: boolean
  /** Retain PostScript glyph names in a rewritten version 2 `post` table. */
  retainGlyphNames?: boolean
  /** Retain non-Unicode, non-symbol cmap records after remapping. */
  retainLegacyCmap?: boolean
  /** Retain the Windows symbol cmap record after remapping. */
  retainSymbolCmap?: boolean
  /** Drop layout, remap supported data, or reject known contextual loss. */
  layout?: LayoutSubsetMode
  /** Four-byte OpenType feature tags to retain, or all features when omitted. */
  layoutFeatures?: string[]
  /** Four-byte OpenType script tags to retain, or all scripts when omitted. */
  layoutScripts?: string[]
  /** OpenType language tags to retain; `default` selects DefaultLangSys. */
  layoutLanguages?: string[]
  /** OpenType name IDs to retain, or all name IDs when omitted. */
  nameIds?: number[]
  /** Platform-specific name language IDs to retain, or all languages when omitted. */
  nameLanguages?: number[]
  /** Optional OpenType tables to remove after subsetting. */
  dropTables?: string[]
  /** Optional source tables to copy verbatim into the subset. */
  passThroughTables?: string[]
  missingGlyphs?: MissingGlyphPolicy
  /** Retain the cvt, fpgm, and prep TrueType program tables while trimming. */
  preserveHinting?: boolean
  /** Skip subsetting and return the validated source bytes unchanged when false. */
  trim?: boolean
}

export interface GidMapping {
  newGid: number
  oldGid: number
}

export interface UnicodeGidMapping {
  oldGid: number
  unicode: number
}

export interface GlyphNameGidMapping {
  glyphName: string
  oldGid: number
}

export interface SubsetReport {
  cffCharstringsVerbatim: boolean
  droppedContextSubtables: number
  glyphsRetained: number
  glyphNameToOldGid: GlyphNameGidMapping[]
  missingGids: number[]
  missingGlyphNames: string[]
  newToOld: (number | null)[]
  oldToNew: GidMapping[]
  originalSize: number
  requestedGids: number[]
  requestedGlyphNames: string[]
  subsetSize: number
  supportedGids: number[]
  supportedGlyphNames: string[]
  tablesRetained: string[]
  unicodeToOldGid: UnicodeGidMapping[]
}

export interface SubsetResult {
  data: Uint8Array
  report: SubsetReport
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
  /** Compatibility option: Type 2 hinting cannot be translated to TrueType and is discarded. */
  preserveHinting?: boolean
  variationCoordinates?: Record<string, number>
}

export interface InstanceOptions {
  /** Axis values in fvar user units. Unspecified axes use their defaults. */
  variationCoordinates?: Record<string, number>
}

export interface AxisRange {
  /** Inclusive lower bound in fvar user units. */
  min: number
  /** Inclusive upper bound in fvar user units. */
  max: number
  /** New default, or the original default clamped into the range when omitted. */
  default?: number
}

export type AxisSetting = number | AxisRange

export interface VariationSpaceOptions {
  /** Axis tags mapped to a pin or retained range. Unlisted axes stay variable. */
  axes: Record<string, AxisSetting>
  /** Convert fully pinned CFF2 outlines to CFF1 for older renderers. */
  downgradeCff2?: boolean
}

export interface Svg2TtfOptions {
  /** Compatibility option accepted without generating TrueType hint instructions. */
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
