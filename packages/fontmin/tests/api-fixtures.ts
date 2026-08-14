import { resolve } from 'node:path'

export const currentDir = import.meta.dirname
export const fixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
)
export const variableTtfFixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/noto-sans-sc-variable-compact.ttf',
)
export const multiAxisVariableTtfFixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/estedad-variable.ttf',
)
export const cffFixture = resolve(
  currentDir,
  '../../../fixtures/fonts/otf/source-sans-3-regular.otf',
)
export const cff2Fixture = resolve(
  currentDir,
  '../../../fixtures/fonts/otf/source-serif-4-variable-roman.otf',
)
export const bin = resolve(currentDir, '../bin/fontmin-rs.mjs')
export const homeSvg =
  '<svg viewBox="0 0 1000 1000"><path d="M100 500 L500 100 L900 500 L900 900 L100 900 Z"/></svg>'
export const userSvg =
  '<svg viewBox="0 0 1000 1000"><path d="M500 100 C620 100 700 180 700 300 C700 420 620 500 500 500 C380 500 300 420 300 300 C300 180 380 100 500 100 Z M250 900 Q500 650 750 900 Z"/></svg>'
export const svgFont =
  '<svg xmlns="http://www.w3.org/2000/svg"><defs><font id="icons" horiz-adv-x="1000"><font-face font-family="SVG Icons" units-per-em="1000" ascent="850" descent="-150" /><glyph glyph-name="home" unicode="&#xE101;" horiz-adv-x="1000" d="M100 100 L900 100 L900 900 L100 900 Z" /></font></defs></svg>'

export function flagsFromUsage(usage: string): string[] {
  return [
    ...new Set(
      [
        ...usage.matchAll(
          /(?:^|[|(\s[])(?<flag>--[a-z][a-z-]*|-[A-Za-z])(?=[|),\s\]])/gmu,
        ),
      ]
        .map(match => match.groups?.['flag'])
        .filter(flag => flag !== undefined),
    ),
  ].toSorted()
}

export function otfFromTtf(input: Buffer): Buffer {
  const otf = Buffer.from(input)

  otf.write('OTTO', 0, 'ascii')

  return otf
}

export function sfntTableVersion(input: Uint8Array, tag: string): number {
  const view = new DataView(input.buffer, input.byteOffset, input.byteLength)
  const decoder = new TextDecoder()
  const tableCount = view.getUint16(4)

  for (let index = 0; index < tableCount; index += 1) {
    const recordOffset = 12 + index * 16
    if (
      decoder.decode(input.subarray(recordOffset, recordOffset + 4)) === tag
    ) {
      return view.getUint32(view.getUint32(recordOffset + 8))
    }
  }

  throw new Error(`SFNT table ${tag} is missing`)
}

export function hasCmapRecord(
  input: Uint8Array,
  platformId: number,
  encodingId: number,
): boolean {
  const view = new DataView(input.buffer, input.byteOffset, input.byteLength)
  const decoder = new TextDecoder()
  const tableCount = view.getUint16(4)

  for (let index = 0; index < tableCount; index += 1) {
    const recordOffset = 12 + index * 16
    if (
      decoder.decode(input.subarray(recordOffset, recordOffset + 4)) !== 'cmap'
    ) {
      continue
    }
    const cmapOffset = view.getUint32(recordOffset + 8)
    const cmapRecordCount = view.getUint16(cmapOffset + 2)
    return Array.from({ length: cmapRecordCount }, (_, cmapIndex) => {
      const cmapRecordOffset = cmapOffset + 4 + cmapIndex * 8
      return (
        view.getUint16(cmapRecordOffset) === platformId &&
        view.getUint16(cmapRecordOffset + 2) === encodingId
      )
    }).some(Boolean)
  }

  return false
}
