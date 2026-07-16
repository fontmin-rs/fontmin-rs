import type { CoverageReport } from '@fontmin-rs/wasm'

export type InputFormat = 'eot' | 'otf' | 'svg' | 'ttf' | 'woff' | 'woff2'

export type PlaygroundFormat = 'css' | 'eot' | 'svg' | 'ttf' | 'woff' | 'woff2'

export type PlaygroundDeliveryPreset = 'latin' | 'cjk' | 'custom'

export interface BrowserDeliverySlice {
  name: string
  unicodeRanges: string[]
}

export interface PlaygroundAsset {
  contents: Uint8Array
  fileName: string
  format: PlaygroundFormat
  unicodeRanges?: string[]
}

export interface ProcessFontRequest {
  contents: Uint8Array
  deliverySlices?: BrowserDeliverySlice[]
  fileName: string
  formats: ReadonlySet<PlaygroundFormat>
  onCoverage?: (report: CoverageReport) => void
  onPhase?: (phase: PlaygroundPhase) => void
  text: string
  unicodeRanges?: string[]
}

export type PlaygroundPhase =
  | 'archiving'
  | 'complete'
  | 'converting'
  | 'error'
  | 'idle'
  | 'initializing'
  | 'normalizing'
  | 'subsetting'

export interface PlaygroundCopy {
  chooseFile: string
  coverage: string
  coverageComplete: string
  coverageMissing: string
  coverageRequested: string
  coverageSupported: string
  characters: string
  charactersHelp: string
  changeFile: string
  download: string
  downloadAsset: string
  downloadZip: string
  delivery: string
  deliveryCjk: string
  deliveryCustom: string
  deliveryCustomHelp: string
  deliveryHelp: string
  deliveryLatin: string
  dropFile: string
  fontFile: string
  formats: string
  generate: string
  generatedFiles: string
  localOnly: string
  processing: Record<PlaygroundPhase, string>
  replaceFile: string
  resultsHelp: string
  selectFormats: string
  title: string
  uniqueCodePoints: string
  unicodeRanges: string
  unicodeRangesHelp: string
  uploadHelp: string
}
