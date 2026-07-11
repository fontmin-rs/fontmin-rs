import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const binding = require('./bindings.js')

export const {
  eotToTtf,
  generateFontFaceCss,
  inspectFont,
  otfToTtf,
  subsetTtf,
  svgFontToTtf,
  svgsToTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
  woffToTtf,
} = binding
