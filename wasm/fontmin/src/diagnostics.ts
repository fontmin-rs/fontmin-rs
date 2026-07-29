import { createFontminDiagnostics } from '../../../packages/fontmin/src/runtime-neutral/diagnostics'

export { FontminDiagnosticError } from '../../../packages/fontmin/src/runtime-neutral/diagnostics'
export type { FontminDiagnosticCode } from '../../../packages/fontmin/src/runtime-neutral/diagnostics'

export const { normalizeFontminDiagnostic, withFontminDiagnostics } =
  createFontminDiagnostics('fontmin-rs WASM operation')
