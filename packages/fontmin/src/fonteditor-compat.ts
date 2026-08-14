import fontEditor from 'fonteditor-core'
import type {
  FontminGlyphTransform,
  FontminTtfEditor,
  FontminTtfObject,
} from './types'

interface FontEditorTransformResult {
  contents: Buffer
  ttfObject: FontminTtfObject
}

export function transformLegacyTtf(
  input: Uint8Array,
  transform: FontminGlyphTransform,
  preserveHinting: boolean,
): FontEditorTransformResult {
  const reader = new fontEditor.TTFReader({ hinting: preserveHinting })
  const ttfObject = reader.read(
    fontEditor.toArrayBuffer(Buffer.from(input)),
  ) as FontminTtfObject
  const ttf = new fontEditor.TTF(ttfObject) as FontminTtfEditor

  transform(ttf)

  const transformedObject = ttf.get()
  const writer = new fontEditor.TTFWriter({
    hinting: preserveHinting,
    writeZeroContoursGlyfData: true,
  })
  const contents = fontEditor.toBuffer(writer.write(transformedObject))

  return { contents, ttfObject: transformedObject }
}
