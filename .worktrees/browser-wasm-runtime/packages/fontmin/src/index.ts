export { defineConfig, loadConfig } from './config'
export { default } from './compat'
export {
  eotToTtf,
  generateFontFaceCss,
  inspect,
  otfToTtf,
  subsetTtf,
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
export { optimize } from './optimize'
export {
  css,
  definePlugin,
  glyph,
  otf2ttf,
  svg2ttf,
  svgs2ttf,
  ttf2eot,
  ttf2svg,
  ttf2woff,
  ttf2woff2,
} from './plugins'
export { fontminCompatPreset, modernWeb } from './presets'
export type * from './types'
