import { readFile } from 'node:fs/promises'
import { extname, relative, resolve } from 'node:path'
import { glob } from 'tinyglobby'

export interface WebTextDiscoveryOptions {
  /** Project directory used to resolve file paths and globs. */
  cwd?: string
  /** Local source files or glob patterns. */
  files: string[]
}

export interface WebTextDiscoveryResult {
  /** Matched files, relative to cwd and sorted deterministically. */
  files: string[]
  /** Unique discovered characters, sorted by Unicode scalar value. */
  text: string
  /** Unicode scalar values represented by text. */
  unicodes: number[]
}

const MARKUP_EXTENSIONS = new Set([
  '.astro',
  '.htm',
  '.html',
  '.svelte',
  '.vue',
])
const SCRIPT_EXTENSIONS = new Set([
  '.cjs',
  '.cts',
  '.js',
  '.jsx',
  '.mjs',
  '.mts',
  '.ts',
  '.tsx',
])
const TEXT_EXTENSIONS = new Set(['.md', '.mdx', '.txt'])

/**
 * Discover static text from local web source files.
 *
 * @param options - Source paths and project directory.
 * @returns Deterministically sorted files, characters, and Unicode values.
 */
export async function discoverWebText(
  options: WebTextDiscoveryOptions,
): Promise<WebTextDiscoveryResult> {
  if (options.files.length === 0) {
    throw new TypeError('discoverWebText requires at least one file or glob')
  }
  const cwd = resolve(options.cwd ?? process.cwd())
  const paths = new Set<string>()

  for (const pattern of options.files) {
    if (isGlobPattern(pattern)) {
      const matches = await glob(pattern, {
        absolute: true,
        cwd,
        onlyFiles: true,
      })
      if (matches.length === 0) {
        throw new Error(`fontmin-rs content glob matched no files: ${pattern}`)
      }
      for (const match of matches) {
        paths.add(resolve(match))
      }
    } else {
      paths.add(resolve(cwd, pattern))
    }
  }

  const sortedPaths = [...paths].sort((left, right) =>
    left.localeCompare(right),
  )
  const characters = new Set<string>()
  for (const path of sortedPaths) {
    const source = await readFile(path, 'utf8')
    for (const character of extractWebText(source, extname(path))) {
      if (isUsefulCharacter(character)) {
        characters.add(normalizeWhitespace(character))
      }
    }
  }
  const text = [...characters]
    .toSorted((left, right) => codePoint(left) - codePoint(right))
    .join('')

  return {
    files: sortedPaths.map(path => relative(cwd, path)),
    text,
    unicodes: [...text].map(character => codePoint(character)),
  }
}

/**
 * Extract conservative static text from one source file.
 *
 * @param source - UTF-8 source contents.
 * @param extension - File extension including the leading dot.
 * @returns Candidate user-visible text.
 */
export function extractWebText(source: string, extension: string): string {
  const normalizedExtension = extension.toLowerCase()
  if (MARKUP_EXTENSIONS.has(normalizedExtension)) {
    return extractMarkupText(source)
  }
  if (SCRIPT_EXTENSIONS.has(normalizedExtension)) {
    return extractStringLiterals(source)
  }
  if (normalizedExtension === '.css') {
    return extractCssContent(source)
  }
  if (normalizedExtension === '.json') {
    return extractJsonValues(source)
  }
  if (TEXT_EXTENSIONS.has(normalizedExtension)) {
    return source
      .replaceAll(/```[\s\S]*?```/gu, ' ')
      .replaceAll(/`[^`]*`/gu, ' ')
  }

  return source
}

function extractMarkupText(source: string): string {
  const scripts = [
    ...source.matchAll(/<script\b[^>]*>(?<value>[\s\S]*?)<\/script>/giu),
  ]
    .map(match => extractStringLiterals(match.groups?.['value'] ?? ''))
    .join(' ')
  const styles = [
    ...source.matchAll(/<style\b[^>]*>(?<value>[\s\S]*?)<\/style>/giu),
  ]
    .map(match => extractCssContent(match.groups?.['value'] ?? ''))
    .join(' ')
  const attributes = [
    ...source.matchAll(
      /\b(?:alt|aria-label|placeholder|title|value)\s*=\s*(?<quote>["'])(?<value>.*?)\k<quote>/giu,
    ),
  ]
    .map(match => decodeHtmlEntities(match.groups?.['value'] ?? ''))
    .join(' ')
  const body = decodeHtmlEntities(
    source
      .replaceAll(/<!--[\s\S]*?-->/gu, ' ')
      .replaceAll(/<script\b[^>]*>[\s\S]*?<\/script>/giu, ' ')
      .replaceAll(/<style\b[^>]*>[\s\S]*?<\/style>/giu, ' ')
      .replaceAll(/<[^>]+>/gu, ' '),
  )

  return `${body} ${attributes} ${scripts} ${styles}`
}

function extractCssContent(source: string): string {
  return [
    ...source.matchAll(
      /\bcontent\s*:\s*(?<quote>["'])(?<value>.*?)\k<quote>/giu,
    ),
  ]
    .map(match => decodeCssEscapes(match.groups?.['value'] ?? ''))
    .join(' ')
}

function extractStringLiterals(source: string): string {
  return [
    ...source.matchAll(
      /(?<quote>["'`])(?<value>(?:\\.|(?!\k<quote>)[\s\S])*?)\k<quote>/gu,
    ),
  ]
    .map(match => decodeScriptEscapes(match.groups?.['value'] ?? ''))
    .join(' ')
}

function extractJsonValues(source: string): string {
  let value: unknown
  try {
    value = JSON.parse(source)
  } catch {
    return extractStringLiterals(source)
  }

  const values: string[] = []
  const collect = (entry: unknown): void => {
    if (typeof entry === 'string') {
      values.push(entry)
    } else if (Array.isArray(entry)) {
      for (const value of entry) {
        collect(value)
      }
    } else if (entry !== null && typeof entry === 'object') {
      for (const value of Object.values(entry)) {
        collect(value)
      }
    }
  }
  collect(value)

  return values.join(' ')
}

function decodeHtmlEntities(value: string): string {
  const named: Record<string, string> = {
    amp: '&',
    apos: "'",
    gt: '>',
    lt: '<',
    nbsp: ' ',
    quot: '"',
  }

  return replaceMatches(
    value,
    /&(?<body>#x[\da-f]+|#\d+|[a-z]+);/giu,
    match => {
      const normalized = (match.groups?.['body'] ?? '').toLowerCase()
      if (normalized.startsWith('#x')) {
        return scalarFromNumber(
          Number.parseInt(normalized.slice(2), 16),
          match[0],
        )
      }
      if (normalized.startsWith('#')) {
        return scalarFromNumber(
          Math.trunc(Number(normalized.slice(1))),
          match[0],
        )
      }
      return named[normalized] ?? match[0]
    },
  )
}

function decodeScriptEscapes(value: string): string {
  const unicodeScalar = replaceHexEscape(value, /\\u\{(?<hex>[\da-f]+)\}/giu)
  const unicode = replaceHexEscape(unicodeScalar, /\\u(?<hex>[\da-f]{4})/giu)
  const hexadecimal = replaceHexEscape(unicode, /\\x(?<hex>[\da-f]{2})/giu)

  return hexadecimal
    .replaceAll(/\\n|\\r|\\t/gu, ' ')
    .replaceAll(/\\(?<value>["'`\\])/gu, '$<value>')
}

function decodeCssEscapes(value: string): string {
  return replaceHexEscape(value, /\\(?<hex>[\da-f]{1,6})\s?/giu)
}

function replaceHexEscape(value: string, pattern: RegExp): string {
  return replaceMatches(value, pattern, match =>
    scalarFromNumber(
      Number.parseInt(match.groups?.['hex'] ?? '', 16),
      match[0],
    ),
  )
}

function replaceMatches(
  value: string,
  pattern: RegExp,
  replacer: (match: RegExpExecArray) => string,
): string {
  let result = ''
  let cursor = 0
  for (const match of value.matchAll(pattern)) {
    const index = match.index
    result += value.slice(cursor, index)
    result += replacer(match)
    cursor = index + match[0].length
  }

  return result + value.slice(cursor)
}

function scalarFromNumber(value: number, fallback: string): string {
  try {
    return String.fromCodePoint(value)
  } catch {
    return fallback
  }
}

function isUsefulCharacter(character: string): boolean {
  const value = codePoint(character)
  return value === 9 || value === 10 || value === 13 || value >= 32
}

function normalizeWhitespace(character: string): string {
  return /\s/u.test(character) ? ' ' : character
}

function codePoint(character: string): number {
  return character.codePointAt(0) ?? 0
}

function isGlobPattern(path: string): boolean {
  return /[*?[\]{}]/u.test(path)
}
