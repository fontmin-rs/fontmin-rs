import { readFileSync } from 'node:fs'
import type { readFile as readFileAsync } from 'node:fs/promises'
import { expect, it, vi } from 'vitest'
import { fixture } from './api-fixtures'

const mocks = vi.hoisted(() => ({
  readFile: vi.fn<() => Promise<Buffer>>(),
}))

vi.mock(import('node:fs/promises'), () => ({
  readFile: mocks.readFile as unknown as typeof readFileAsync,
}))

it('retries the public WASM fallback after a transient load failure', async () => {
  const wasm = readFileSync(
    new URL(
      '../../../wasm/fontmin/dist/fontmin_wasm_core_bg.wasm',
      import.meta.url,
    ),
  )
  mocks.readFile
    .mockRejectedValueOnce(new Error('transient WASM read failure'))
    .mockResolvedValue(wasm)
  const { ttfToWoff2Async } = await import('../src/native')
  const input = readFileSync(fixture)

  await expect(ttfToWoff2Async(input, { fallback: 'wasm' })).rejects.toThrow(
    'WASM runtime failed to initialize',
  )

  const output = await ttfToWoff2Async(input, { fallback: 'wasm' })

  expect(Buffer.from(output.subarray(0, 4)).toString('ascii')).toBe('wOF2')
})
