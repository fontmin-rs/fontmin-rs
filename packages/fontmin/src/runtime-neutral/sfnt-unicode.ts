/**
 * Read mapped Unicode scalar values from the Unicode cmap subtables of an SFNT.
 *
 * @param input - TTF or OTF bytes.
 * @returns Sorted, unique code points whose mapped glyph ID is non-zero.
 */
export function unicodeCodePointsFromSfnt(input: Uint8Array): number[] {
  const view = new DataView(input.buffer, input.byteOffset, input.byteLength)
  const cmapOffset = tableOffset(view, input, 'cmap')
  if (cmapOffset === undefined || cmapOffset + 4 > view.byteLength) {
    return []
  }
  const recordCount = view.getUint16(cmapOffset + 2)
  const codePoints = new Set<number>()
  const visited = new Set<number>()

  for (let index = 0; index < recordCount; index += 1) {
    const recordOffset = cmapOffset + 4 + index * 8
    if (recordOffset + 8 > view.byteLength) {
      break
    }
    const platform = view.getUint16(recordOffset)
    const encoding = view.getUint16(recordOffset + 2)
    if (
      platform !== 0 &&
      !(platform === 3 && (encoding === 1 || encoding === 10))
    ) {
      continue
    }
    const subtableOffset = cmapOffset + view.getUint32(recordOffset + 4)
    if (visited.has(subtableOffset) || subtableOffset + 2 > view.byteLength) {
      continue
    }
    visited.add(subtableOffset)
    collectSubtable(view, subtableOffset, codePoints)
  }

  return [...codePoints].toSorted((left, right) => left - right)
}

function tableOffset(
  view: DataView,
  input: Uint8Array,
  requestedTag: string,
): number | undefined {
  if (view.byteLength < 12) {
    return
  }
  const tableCount = view.getUint16(4)
  const decoder = new TextDecoder()

  for (let index = 0; index < tableCount; index += 1) {
    const recordOffset = 12 + index * 16
    if (recordOffset + 16 > view.byteLength) {
      return
    }
    const tag = decoder.decode(input.subarray(recordOffset, recordOffset + 4))
    if (tag === requestedTag) {
      const offset = view.getUint32(recordOffset + 8)
      return offset < view.byteLength ? offset : undefined
    }
  }

  return undefined
}

function collectSubtable(
  view: DataView,
  offset: number,
  codePoints: Set<number>,
): void {
  const format = view.getUint16(offset)
  if (format === 0) {
    collectFormat0(view, offset, codePoints)
  } else if (format === 4) {
    collectFormat4(view, offset, codePoints)
  } else if (format === 12 || format === 13) {
    collectFormat12Or13(view, offset, codePoints, format)
  }
}

function collectFormat0(
  view: DataView,
  offset: number,
  codePoints: Set<number>,
): void {
  if (offset + 262 > view.byteLength) {
    return
  }
  for (let codePoint = 0; codePoint < 256; codePoint += 1) {
    if (view.getUint8(offset + 6 + codePoint) !== 0) {
      codePoints.add(codePoint)
    }
  }
}

function collectFormat4(
  view: DataView,
  offset: number,
  codePoints: Set<number>,
): void {
  if (offset + 16 > view.byteLength) {
    return
  }
  const length = view.getUint16(offset + 2)
  const end = Math.min(offset + length, view.byteLength)
  const segmentCount = view.getUint16(offset + 6) / 2
  const endCodesOffset = offset + 14
  const startCodesOffset = endCodesOffset + segmentCount * 2 + 2
  const deltasOffset = startCodesOffset + segmentCount * 2
  const rangeOffsetsOffset = deltasOffset + segmentCount * 2

  if (rangeOffsetsOffset + segmentCount * 2 > end) {
    return
  }
  for (let segment = 0; segment < segmentCount; segment += 1) {
    const start = view.getUint16(startCodesOffset + segment * 2)
    const segmentEnd = view.getUint16(endCodesOffset + segment * 2)
    const delta = view.getInt16(deltasOffset + segment * 2)
    const rangeOffsetPosition = rangeOffsetsOffset + segment * 2
    const rangeOffset = view.getUint16(rangeOffsetPosition)

    for (
      let codePoint = start;
      codePoint <= segmentEnd && codePoint < 0xff_ff;
      codePoint += 1
    ) {
      let glyphId: number

      if (rangeOffset === 0) {
        glyphId = uint16(codePoint + delta)
      } else {
        const glyphOffset =
          rangeOffsetPosition + rangeOffset + (codePoint - start) * 2
        if (glyphOffset + 2 > end) {
          continue
        }
        const rawGlyphId = view.getUint16(glyphOffset)
        glyphId = rawGlyphId === 0 ? 0 : uint16(rawGlyphId + delta)
      }
      if (glyphId !== 0 && isValidUnicodeScalar(codePoint)) {
        codePoints.add(codePoint)
      }
    }
  }
}

function collectFormat12Or13(
  view: DataView,
  offset: number,
  codePoints: Set<number>,
  format: 12 | 13,
): void {
  if (offset + 16 > view.byteLength) {
    return
  }
  const length = view.getUint32(offset + 4)
  const end = Math.min(offset + length, view.byteLength)
  const groupCount = view.getUint32(offset + 12)

  for (let group = 0; group < groupCount; group += 1) {
    const groupOffset = offset + 16 + group * 12
    if (groupOffset + 12 > end) {
      break
    }
    const start = view.getUint32(groupOffset)
    const groupEnd = Math.min(view.getUint32(groupOffset + 4), 0x10_ff_ff)
    const startGlyphId = view.getUint32(groupOffset + 8)

    for (let codePoint = start; codePoint <= groupEnd; codePoint += 1) {
      const glyphId =
        format === 12 ? startGlyphId + codePoint - start : startGlyphId
      if (glyphId !== 0 && isValidUnicodeScalar(codePoint)) {
        codePoints.add(codePoint)
      }
    }
  }
}

function isValidUnicodeScalar(codePoint: number): boolean {
  return codePoint < 0xd8_00 || codePoint > 0xdf_ff
}

function uint16(value: number): number {
  return ((value % 65_536) + 65_536) % 65_536
}
