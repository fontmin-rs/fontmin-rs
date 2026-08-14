import { readdirSync } from 'node:fs'
import { resolve } from 'node:path'

export interface FontminSubsetTextOptions {
  basicText?: boolean
  text?: string
  trim?: boolean
}

export const plugins = [
  'glyph',
  'ttf2eot',
  'ttf2woff',
  'ttf2woff2',
  'ttf2svg',
  'css',
  'svg2ttf',
  'svgs2ttf',
  'otf2ttf',
]

export const mime = {
  '.*': 'application/octet-stream',
  eot: 'application/octet-stream',
  otf: 'application/font-sfnt',
  svg: 'image/svg+xml',
  svgz: 'image/svg+xml',
  ttf: 'application/font-sfnt',
  woff: 'application/font-woff',
  woff2: 'application/font-woff2',
} as const

const BASIC_TEXT = Array.from({ length: 93 }, (_, index) =>
  String.fromCodePoint(index + 33),
).join('')

function getFontFolder(): string {
  const folders: Partial<Record<NodeJS.Platform, string>> = {
    darwin: '/Library/Fonts',
    linux: '/usr/share/fonts/truetype',
    win32: '/Windows/fonts',
  }
  const folder = folders[process.platform]

  if (folder === undefined) {
    throw new Error(
      `fontmin does not define a font folder for ${process.platform}`,
    )
  }

  return resolve(folder)
}

function getFonts(): string[] {
  return readdirSync(getFontFolder())
}

function getPureText(value: unknown): string {
  const whitespace = new Set<string>()
  const text = String(value)
    .replaceAll(/\s/gu, character => {
      whitespace.add(character)
      return ''
    })
    .trim()
    .replaceAll(/[\u2028\u2029]/gu, '')

  return text + [...whitespace].join('')
}

function getUniqText(value: string): string {
  // oxlint-disable-next-line unicorn/prefer-spread -- Fontmin splits UTF-16 code units.
  return [...new Set(value.split(''))].join('')
}

function getSubsetText(options: FontminSubsetTextOptions): string {
  let text = options.text ?? ''

  if (text.length > 0 && options.trim === true) {
    text = getPureText(text)
  }
  if (options.basicText === true) {
    text += BASIC_TEXT
  }

  return text
}

function string2unicodes(value: string): number[] {
  return [
    ...new Set(
      [...value].map(character => {
        const codePoint = character.codePointAt(0)

        if (codePoint === undefined) {
          throw new Error('fontmin could not read an empty Unicode character')
        }

        return codePoint
      }),
    ),
  ]
}

export const util = {
  getFontFolder,
  getFonts,
  getPureText,
  getSubsetText,
  getUniqText,
  string2unicodes,
}
