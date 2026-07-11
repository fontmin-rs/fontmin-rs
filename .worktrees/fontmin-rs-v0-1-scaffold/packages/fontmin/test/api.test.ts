import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { expect, it } from 'vitest'
import Fontmin, {
  defineConfig,
  definePlugin,
  glyph,
  subsetTtf,
  ttf2woff2,
} from '../src/index'

const currentDir = import.meta.dirname
const fixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
)

it('subsets through the public package api', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { text: 'Hello' })

  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('returns typed config and plugin objects', () => {
  const config = defineConfig({
    input: ['fonts/*.ttf'],
    outDir: 'build',
    plugins: [glyph({ text: 'Hello' }), ttf2woff2()],
  })
  const plugin = definePlugin({ name: 'example' })

  expect(config.plugins).toHaveLength(2)
  expect(plugin.name).toBe('example')
})

it('builds a fontmin-compatible chain', () => {
  const instance = new Fontmin()
    .src('fixtures/fonts/ttf/roboto-regular.ttf')
    .use(Fontmin.glyph({ text: 'Hello' }))
    .dest('build')

  expect(instance.config()).toMatchObject({
    input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
    outDir: 'build',
  })
})
