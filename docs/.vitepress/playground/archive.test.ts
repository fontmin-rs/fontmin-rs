import { strFromU8, unzipSync } from 'fflate'
import { describe, expect, it, vi } from 'vitest'
import { createArchive, downloadArchive, downloadAsset } from './archive'

const { saveAs } = vi.hoisted(() => ({
  saveAs: vi.fn<(blob: Blob | string, fileName?: string) => void>(),
}))

vi.mock(import('tinysaver'), () => ({ saveAs }))

const assets = [
  {
    contents: new Uint8Array([1, 2, 3]),
    fileName: 'demo.woff2',
    format: 'woff2' as const,
  },
  {
    contents: new TextEncoder().encode('body {}'),
    fileName: 'demo.css',
    format: 'css' as const,
  },
]

describe('createArchive', () => {
  it('creates a ZIP containing every generated asset', () => {
    const files = unzipSync(createArchive(assets))

    expect(Object.keys(files)).toStrictEqual(['demo.woff2', 'demo.css'])
    expect(files['demo.woff2']).toStrictEqual(new Uint8Array([1, 2, 3]))
    expect(strFromU8(files['demo.css'])).toBe('body {}')
  })
})

describe('downloadArchive', () => {
  it('passes a ZIP Blob and requested name to tinysaver', () => {
    downloadArchive(assets, 'demo-fontmin.zip')

    expect(saveAs).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'application/zip' }),
      'demo-fontmin.zip',
    )
  })
})

describe('downloadAsset', () => {
  it('passes an asset Blob and its original name to tinysaver', () => {
    downloadAsset(assets[0])

    expect(saveAs).toHaveBeenCalledWith(expect.any(Blob), 'demo.woff2')
  })
})
