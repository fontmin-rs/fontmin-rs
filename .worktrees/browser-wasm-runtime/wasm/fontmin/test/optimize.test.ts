import { readFile } from 'node:fs/promises'
import { beforeAll, expect, it } from 'vitest'
import {
  initWasm,
  fontminCompatPreset,
  modernWeb,
  optimizeBrowser,
  css,
  svg2ttf,
  svgs2ttf,
} from '../src/index'

const fixture = new URL(
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
  import.meta.url,
)
const wasm = new URL(
  '../src/generated/fontmin_wasm_core_bg.wasm',
  import.meta.url,
)

beforeAll(async () => {
  await initWasm(await readFile(wasm))
})

it('creates compatibility outputs through the browser preset', async () => {
  const assets = await optimizeBrowser({
    assets: [{ contents: await readFile(fixture), fileName: 'roboto.ttf' }],
    plugins: fontminCompatPreset({
      fontFamily: 'Roboto Compat',
      text: 'Hello',
    }),
  })

  expect(assets.map(asset => asset.fileName)).toStrictEqual([
    'roboto.ttf',
    'roboto.eot',
    'roboto.svg',
    'roboto.woff',
    'roboto.woff2',
    'roboto.css',
  ])
})

it('converts SVG font and SVG icon assets through built-in plugins', async () => {
  const svgFont = new TextEncoder().encode(
    '<svg><defs><font horiz-adv-x="1000"><font-face font-family="Icons" units-per-em="1000"/><glyph unicode="&#xE001;" d="M0 0 L1000 0 L1000 1000 Z"/></font></defs></svg>',
  )
  const convertedFont = await optimizeBrowser({
    assets: [{ contents: svgFont, fileName: 'icons.svg' }],
    plugins: [svg2ttf()],
  })
  const iconFont = await optimizeBrowser({
    assets: [
      {
        contents: new TextEncoder().encode(
          '<svg viewBox="0 0 1000 1000"><path d="M0 0 L1000 0 L1000 1000 Z"/></svg>',
        ),
        fileName: 'triangle.svg',
      },
    ],
    plugins: [svgs2ttf({ fontName: 'Project Icons' })],
  })

  expect(convertedFont.map(asset => asset.fileName)).toContain('icons.ttf')
  expect(iconFont.map(asset => asset.fileName)).toContain('project-icons.ttf')
})

it('runs custom in-memory plugin hooks without filesystem access', async () => {
  const assets = await optimizeBrowser({
    assets: [{ contents: new Uint8Array([1]), fileName: 'input.bin' }],
    plugins: [
      {
        name: 'example:memory',
        transform(asset, context) {
          context.warn('processed in memory')
          context.emitFile({
            contents: new Uint8Array([3]),
            fileName: 'emitted.bin',
          })
          return { ...asset, fileName: 'renamed.bin' }
        },
      },
    ],
  })

  expect(assets.map(asset => asset.fileName)).toStrictEqual([
    'renamed.bin',
    'emitted.bin',
  ])
})

it('runs subset, WOFF, WOFF2, and CSS entirely in memory', async () => {
  const plugins = modernWeb({ fontFamily: 'Roboto Browser', text: 'Hello' })

  expect(plugins[0]?.options).toMatchObject({ text: 'Hello' })

  const assets = await optimizeBrowser({
    assets: [
      {
        contents: await readFile(fixture),
        fileName: 'roboto.ttf',
      },
    ],
    plugins,
  })

  expect(assets.map(asset => asset.fileName)).toEqual([
    'roboto.ttf',
    'roboto.woff',
    'roboto.woff2',
    'roboto.css',
  ])
  expect(new TextDecoder().decode(assets[2]?.contents.subarray(0, 4))).toBe(
    'wOF2',
  )
  expect(new TextDecoder().decode(assets[3]?.contents)).toContain(
    "font-family: 'Roboto Browser'",
  )
})

it('embeds browser assets when the CSS plugin enables Base64 output', async () => {
  const assets = await optimizeBrowser({
    assets: [{ contents: await readFile(fixture), fileName: 'roboto.ttf' }],
    plugins: [css({ base64: true, local: false })],
  })

  expect(new TextDecoder().decode(assets[1]?.contents)).toContain(
    'data:font/ttf;base64,',
  )
})
