export type FontminDiagnosticCode =
  | 'fontmin::config'
  | 'fontmin::convert_failed'
  | 'fontmin::invalid_font'
  | 'fontmin::io'
  | 'fontmin::missing_glyph'
  | 'fontmin::napi_bridge_failed'
  | 'fontmin::plugin_failed'
  | 'fontmin::unsupported_format'

const bridgeDiagnosticPattern =
  /^\[(?<code>fontmin::[a-z_]+)\] (?<message>[\s\S]+)$/u

export class FontminDiagnosticError extends Error {
  override readonly name = 'FontminDiagnosticError'
  readonly code: FontminDiagnosticCode

  constructor(
    code: FontminDiagnosticCode,
    message: string,
    options?: ErrorOptions,
  ) {
    super(message, options)
    this.code = code
  }
}

export function createFontminDiagnostics(operationName: string) {
  function normalizeFontminDiagnostic(error: unknown): Error {
    let message: string | undefined
    if (error instanceof Error) {
      message = error.message
    } else if (typeof error === 'string') {
      message = error
    }
    const match = message?.match(bridgeDiagnosticPattern)

    if (match === null || match === undefined) {
      if (error instanceof Error) {
        return error
      }
      if (typeof error === 'string') {
        return new Error(error, { cause: error })
      }
      return new Error(`${operationName} failed`, { cause: error })
    }

    const code = match.groups?.['code']
    const diagnosticMessage = match.groups?.['message']

    if (code === undefined || diagnosticMessage === undefined) {
      return new Error(`${operationName} returned an invalid diagnostic`, {
        cause: error,
      })
    }

    return new FontminDiagnosticError(
      code as FontminDiagnosticCode,
      diagnosticMessage,
      { cause: error },
    )
  }

  function withFontminDiagnostics<T>(operation: () => T): T {
    try {
      return operation()
    } catch (error) {
      throw normalizeFontminDiagnostic(error)
    }
  }

  return {
    normalizeFontminDiagnostic,
    withFontminDiagnostics,
  }
}
