import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const binding = require('./bindings.js')

export const {
  analyzeCoverage,
  createTtfSubsetPlan,
  eotToTtf,
  extractCollectionFace,
  generateFontFaceCss,
  inspectCollection,
  inspectCapabilities,
  inspectFont,
  instantiateFont,
  otfToTtf,
  reduceVariationSpace,
  subsetTtf,
  subsetTtfWithPlan,
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
