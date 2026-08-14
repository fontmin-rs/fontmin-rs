export { initWasm, isWasmInitialized } from './runtime'
export { FontminDiagnosticError } from './diagnostics'
export type { FontminDiagnosticCode } from './diagnostics'
export {
  analyzeCoverage,
  eotToTtf,
  extractCollectionFace,
  generateFontFaceCss,
  inspect,
  inspectCapabilities,
  inspectCollection,
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
} from './native'
export type * from '../types'
export {
  detectDeliveryLanguages,
  unicodeRangesFromCodePoints,
} from '../../../packages/fontmin/src/runtime-neutral/auto-delivery'
export type {
  AutoDeliveryPlan,
  AutoDeliveryPlanOptions,
  AutoDeliveryPlanSlice,
  DeliveryLanguagePreset,
} from '../../../packages/fontmin/src/runtime-neutral/auto-delivery'
export { optimizeBrowser } from './optimize'
export type { BrowserAsset, BrowserOptimizeConfig } from './optimize'
export {
  autoDeliverySlices,
  css,
  deliverySlices,
  fontminCompatPreset,
  glyph,
  modernWeb,
  otf2ttf,
  svg2ttf,
  svgs2ttf,
  ttf2eot,
  ttf2svg,
  ttf2woff,
  ttf2woff2,
  variationSpace,
} from './plugins'
export type {
  AutoDeliveryPluginOptions,
  BrowserPlugin,
  BrowserPluginContext,
  DeliverySlicesOptions,
  DeliverySlice,
  FontminCompatPresetOptions,
  GlyphOptions,
  MaybePromise,
  ModernWebOptions,
  Otf2TtfPluginOptions,
  Svg2TtfPluginOptions,
  Svgs2TtfPluginOptions,
  Ttf2EotPluginOptions,
  Ttf2SvgPluginOptions,
  Ttf2Woff2PluginOptions,
  Ttf2WoffPluginOptions,
  VariationSpacePluginOptions,
} from './plugins'
