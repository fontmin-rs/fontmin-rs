import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { bench, describe } from 'vitest'
import {
  modernWeb,
  optimize,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
} from '../src/index'

const fixture = readFileSync(
  resolve(
    import.meta.dirname,
    '../../../fixtures/fonts/ttf/roboto-regular.ttf',
  ),
)
const subsetText =
  'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'

describe('convert', () => {
  bench('ttfToWoff', () => {
    ttfToWoff(fixture)
  })

  bench('ttfToWoff2', () => {
    ttfToWoff2(fixture)
  })

  bench('ttfToSvg', () => {
    ttfToSvg(fixture, { fontFamily: 'Roboto' })
  })

  bench('optimize modernWeb pipeline', async () => {
    await optimize({
      cache: false,
      input: [fixture],
      plugins: modernWeb({
        clone: false,
        fontFamily: 'Roboto',
        text: subsetText,
      }),
    })
  })
})
