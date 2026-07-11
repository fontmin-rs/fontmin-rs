import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { bench, describe } from 'vitest'
import { glyph, optimize, subsetTtf } from '../src/index'

const fixture = readFileSync(
  resolve(
    import.meta.dirname,
    '../../../fixtures/fonts/ttf/roboto-regular.ttf',
  ),
)
const subsetText =
  'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'

describe('subset', () => {
  bench('subsetTtf text', () => {
    subsetTtf(fixture, { text: subsetText })
  })

  bench('optimize glyph pipeline', () =>
    optimize({
      cache: false,
      input: [fixture],
      plugins: [glyph({ clone: false, text: subsetText })],
    }).then(() => undefined))
})
