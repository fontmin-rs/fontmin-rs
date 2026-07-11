import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { expect, it } from 'vitest'
import { subsetTtf } from '../src-js/index.js'

const currentDir = import.meta.dirname
const fixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
)

it('subsets a TTF buffer through napi', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { text: 'Hello' })

  expect(Buffer.isBuffer(output)).toBe(true)
  expect(output.byteLength).toBeLessThan(input.byteLength)
})
