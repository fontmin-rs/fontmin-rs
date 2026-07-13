import { describe, expect, it, vi } from 'vitest'
import type { PlaygroundAsset } from './types'
import { useFontPlayground } from './useFontPlayground'

const { downloadAsset } = vi.hoisted(() => ({
  downloadAsset: vi.fn<(asset: PlaygroundAsset) => void>(),
}))

vi.mock(import('./archive'), () => ({
  downloadArchive:
    vi.fn<(assets: PlaygroundAsset[], fileName: string) => void>(),
  downloadAsset,
}))

describe('useFontPlayground', () => {
  it('keeps a valid selection when an unsupported file is chosen', () => {
    const playground = useFontPlayground()
    const valid = new File([new Uint8Array([1])], 'demo.ttf')

    playground.selectFile(valid)
    playground.selectFile(new File([new Uint8Array([1])], 'demo.bin'))

    expect(playground.selectedFile.value).toBe(valid)
    expect(playground.error.value).toBe('Unsupported input format: .bin.')
  })

  it('delegates one generated asset to the archive helper', () => {
    const playground = useFontPlayground()
    const asset = {
      contents: new Uint8Array([1]),
      fileName: 'demo.woff2',
      format: 'woff2' as const,
    }

    playground.downloadAsset(asset)

    expect(downloadAsset).toHaveBeenCalledWith(asset)
  })
})
