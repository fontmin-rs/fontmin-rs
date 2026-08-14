export { defineConfig, loadConfig } from './config'
export { mime, plugins, util } from './compat-exports'
export { default } from './compat'
export { FontminDiagnosticError } from './diagnostics'
export type { FontminDiagnosticCode } from './diagnostics'
export {
  analyzeCoverage,
  eotToTtf,
  generateFontFaceCss,
  inspect,
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
  ttfToWoff2Async,
  validateWoff2,
  woff2ToTtf,
  woffToTtf,
} from './native'
export { optimize } from './optimize'
export { discoverWebText, extractWebText } from './web-text'
export {
  detectDeliveryLanguages,
  unicodeRangesFromCodePoints,
} from './runtime-neutral/auto-delivery'
export type {
  AutoDeliveryPlan,
  AutoDeliveryPlanOptions,
  AutoDeliveryPlanSlice,
  DeliveryLanguagePreset,
} from './runtime-neutral/auto-delivery'
export {
  autoDeliverySlices,
  css,
  deliverySlices,
  definePlugin,
  glyph,
  otf2ttf,
  svg2ttf,
  svgs2ttf,
  ttf2eot,
  ttf2svg,
  ttf2woff,
  ttf2woff2,
  variationSpace,
  webDelivery,
} from './plugins'
export { fontminCompatPreset, modernWeb } from './presets'
export type * from './types'
