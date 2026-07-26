import { readFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { glyph, optimize, ttf2woff } from '../packages/fontmin/dist/index.mjs'

const workspaceRoot = dirname(import.meta.dirname)
const iterationText = process.env.FONTMIN_PROFILE_ITERATIONS ?? '2500'
const iterations = Number(iterationText)

if (!Number.isSafeInteger(iterations) || iterations < 1) {
  throw new Error(
    `FONTMIN_PROFILE_ITERATIONS must be a positive integer, got ${iterationText}`,
  )
}

const input = await readFile(
  join(workspaceRoot, 'fixtures/fonts/ttf/roboto-regular.ttf'),
)
const text = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'

for (let index = 0; index < iterations; index += 1) {
  await optimize({
    cache: false,
    input: [input],
    plugins: [glyph({ clone: false, text }), ttf2woff({ clone: false })],
  })
}

console.log(
  `Profiled ${iterations} release-binding glyph + ttf2woff iterations.`,
)
