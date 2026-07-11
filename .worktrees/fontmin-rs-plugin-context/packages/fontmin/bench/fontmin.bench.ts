import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { resolve } from 'node:path'
import { bench, describe } from 'vitest'
import { glyph, optimize, ttf2woff } from '../src/index'

interface FontminFile {
  contents: Buffer
  path: string
}

interface FontminRunner {
  run(callback: (error: Error | null, files: FontminFile[]) => void): void
  src(input: string): FontminRunner
  use(plugin: unknown): FontminRunner
}

interface FontminConstructor {
  glyph(options: Record<string, unknown>): unknown
  new (): FontminRunner
  ttf2woff(options?: Record<string, unknown>): unknown
}

const require = createRequire(import.meta.url)
const Fontmin = require('fontmin') as FontminConstructor
const fixturePath = resolve(
  import.meta.dirname,
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
)
const fixture = readFileSync(fixturePath)
const subsetText =
  'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'

function runFontminGlyphWoff(): Promise<void> {
  return new Promise((finish, reject) => {
    new Fontmin()
      .src(fixturePath)
      .use(Fontmin.glyph({ text: subsetText }))
      .use(Fontmin.ttf2woff({ clone: false }))
      .run(error => {
        if (error !== null) {
          reject(error)
          return
        }

        finish()
      })
  })
}

describe('fontmin baseline', () => {
  bench('fontmin-rs glyph + ttf2woff', () =>
    optimize({
      cache: false,
      input: [fixture],
      plugins: [
        glyph({ clone: false, text: subsetText }),
        ttf2woff({ clone: false }),
      ],
    }).then(() => {}))

  bench('fontmin glyph + ttf2woff', () => runFontminGlyphWoff())
})
