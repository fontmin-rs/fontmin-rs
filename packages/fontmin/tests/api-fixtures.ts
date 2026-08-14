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
export const cjkFixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/noto-sans-sc-compact.ttf',
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
          /(?<flag>--[a-z][a-z0-9-]*|-[A-Za-z])(?=[|),\s\]])/gmu,
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

export function fontCollection(fonts: Buffer[]): Buffer {
  const headerSize = 12 + fonts.length * 4
  const header = Buffer.alloc(headerSize)
  const parts: Buffer[] = [header]
  header.write('ttcf', 0, 'ascii')
  header.writeUInt32BE(65_536, 4)
  header.writeUInt32BE(fonts.length, 8)
  let offset = headerSize

  for (const [index, font] of fonts.entries()) {
    const padding = (4 - (offset % 4)) % 4
    if (padding > 0) {
      parts.push(Buffer.alloc(padding))
      offset += padding
    }
    header.writeUInt32BE(offset, 12 + index * 4)
    const face = Buffer.from(font)
    const tableCount = face.readUInt16BE(4)
    for (let tableIndex = 0; tableIndex < tableCount; tableIndex += 1) {
      const recordOffset = 12 + tableIndex * 16 + 8
      face.writeUInt32BE(offset + face.readUInt32BE(recordOffset), recordOffset)
    }
    parts.push(face)
    offset += face.length
  }

  return Buffer.concat(parts)
}

export function colrFont(input: Buffer, version: number): Buffer {
  const font = Buffer.from(input)
  const replacements = new Map([
    ['cvt ', 'COLR'],
    ['fpgm', 'CPAL'],
  ])
  const tableCount = font.readUInt16BE(4)

  for (let index = 0; index < tableCount; index += 1) {
    const recordOffset = 12 + index * 16
    const target = replacements.get(
      font.toString('ascii', recordOffset, recordOffset + 4),
    )
    if (target === undefined) {
      continue
    }
    font.write(target, recordOffset, 4, 'ascii')
    if (target === 'COLR') {
      font.writeUInt16BE(version, font.readUInt32BE(recordOffset + 8))
    }
  }

  return font
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

export function variationAxes(input: Uint8Array): {
  default: number
  max: number
  min: number
  tag: string
}[] {
  const view = new DataView(input.buffer, input.byteOffset, input.byteLength)
  const decoder = new TextDecoder()
  const tableCount = view.getUint16(4)

  for (let index = 0; index < tableCount; index += 1) {
    const recordOffset = 12 + index * 16
    if (
      decoder.decode(input.subarray(recordOffset, recordOffset + 4)) !== 'fvar'
    ) {
      continue
    }
    const tableOffset = view.getUint32(recordOffset + 8)
    const axesOffset = tableOffset + view.getUint16(tableOffset + 4)
    const axisCount = view.getUint16(tableOffset + 8)
    const axisSize = view.getUint16(tableOffset + 10)

    return Array.from({ length: axisCount }, (_, axisIndex) => {
      const axisOffset = axesOffset + axisIndex * axisSize

      return {
        default: view.getInt32(axisOffset + 8) / 65_536,
        max: view.getInt32(axisOffset + 12) / 65_536,
        min: view.getInt32(axisOffset + 4) / 65_536,
        tag: decoder.decode(input.subarray(axisOffset, axisOffset + 4)),
      }
    })
  }

  return []
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
