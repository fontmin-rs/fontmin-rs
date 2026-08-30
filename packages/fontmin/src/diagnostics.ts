import { createFontminDiagnostics } from './runtime-neutral/diagnostics'

export { FontminDiagnosticError } from './runtime-neutral/diagnostics'
export type { FontminDiagnosticCode } from './runtime-neutral/diagnostics'

export const {
  normalizeFontminDiagnostic,
  withFontminDiagnostics,
  withFontminDiagnosticsAsync,
} = createFontminDiagnostics('fontmin-rs operation')
