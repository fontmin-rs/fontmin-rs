import { zipSync } from 'fflate'
import { saveAs } from 'tinysaver'
import type { PlaygroundAsset } from './types'

export function createArchive(assets: PlaygroundAsset[]): Uint8Array {
  return zipSync(
    Object.fromEntries(assets.map(asset => [asset.fileName, asset.contents])),
  )
}

export function downloadArchive(
  assets: PlaygroundAsset[],
  fileName: string,
): void {
  saveAs(
    new Blob([createArchive(assets)], { type: 'application/zip' }),
    fileName,
  )
}

export function downloadAsset(asset: PlaygroundAsset): void {
  saveAs(
    new Blob([asset.contents], {
      type: asset.format === 'css' ? 'text/css;charset=utf-8' : 'font/*',
    }),
    asset.fileName,
  )
}
