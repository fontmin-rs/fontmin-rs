import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const binding = require('./bindings.js')

export const {
  analyzeCoverage,
  eotToTtf,
  generateFontFaceCss,
  inspectFont,
  instantiateFont,
  otfToTtf,
  reduceVariationSpace,
  subsetTtf,
  subsetTtfWithReport,
  svgFontToTtf,
  svgsToTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
  validateWoff2,
  woff2ToTtf,
  woffToTtf,
} = binding
