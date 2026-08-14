import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import Ajv from 'ajv'
import { expect, it } from 'vitest'
import { css, glyph, ttf2woff2 } from '../src/index'
import type { FontminConfig } from '../src/index'

const schema = JSON.parse(
  readFileSync(
    resolve(import.meta.dirname, '../configuration_schema.json'),
    'utf8',
  ),
) as object
const ajv = new Ajv({ allErrors: true, strict: true })

ajv.addKeyword({ keyword: 'allowComments', schemaType: 'boolean' })
ajv.addKeyword({ keyword: 'allowTrailingCommas', schemaType: 'boolean' })

const validate = ajv.compile(schema)

it('validates shared and runtime-specific project config fields', () => {
  expect(
    validate({
      $schema: './node_modules/fontmin-rs/configuration_schema.json',
      cache: {
        dir: 'node_modules/.cache/fontmin-rs',
        enabled: true,
      },
      clean: true,
      css: {
        fontDisplay: 'swap',
        fontFamily: 'Roboto',
        unicodeRanges: ['U+0000-00FF'],
      },
      delivery: {
        slices: [
          {
            name: 'latin',
            unicodeRanges: ['U+0000-00FF'],
          },
        ],
      },
      diagnostics: {
        failOnWarning: true,
        level: 'warn',
      },
      input: ['fonts/*.ttf'],
      otf: {
        variationCoordinates: {
          opsz: 14,
          wght: 700,
        },
      },
      outDir: 'build',
      outputs: ['ttf', { clone: false, format: 'woff2' }],
      parallel: {
        perFile: true,
        threads: {
          count: 4,
        },
      },
      runtime: 'auto',
      subset: {
        basicText: true,
        content: ['src/**/*.{html,tsx,vue}'],
        gids: [1, 7],
        glyphNames: ['A', 'space'],
        keepLayout: 'conservative',
        layoutFeatures: ['kern', 'liga'],
        layoutLanguages: ['default', 'ENG'],
        layoutScripts: ['DFLT', 'latn'],
        nameIds: [1, 2],
        nameLanguages: [0x0409],
        dropTables: ['GPOS'],
        passThroughTables: ['gasp'],
        retainGlyphNames: true,
        retainLegacyCmap: true,
        retainSymbolCmap: true,
        missingGlyphs: 'error',
        text: 'Hello',
      },
      webDelivery: {
        basePath: '/assets/fonts',
        fallback: true,
        fontFamily: 'Roboto Web',
        preload: 'all',
      },
    }),
  ).toBe(true)
})

it('validates descriptors produced by public built-in plugin helpers', () => {
  const config: FontminConfig = {
    input: ['fonts/*.ttf'],
    plugins: [
      glyph({
        gids: [1],
        glyphNames: ['A'],
        layoutFeatures: ['liga'],
        layoutLanguages: ['default'],
        layoutScripts: ['latn'],
        nameIds: [1],
        nameLanguages: [0x0409],
        dropTables: ['GPOS'],
        passThroughTables: ['gasp'],
        retainGlyphNames: true,
        retainLegacyCmap: true,
        retainSymbolCmap: true,
        text: 'Hello',
        unicodeRanges: ['U+0000-00FF'],
      }),
      ttf2woff2({ quality: 9 }),
      css({ fontDisplay: 'swap', fontFamily: 'Roboto' }),
    ],
  }

  expect(validate(JSON.parse(JSON.stringify(config)))).toBe(true)
})

it('rejects unknown fields and invalid constrained values', () => {
  expect(validate({ input: [], unknownOption: true })).toBe(false)
  expect(validate({ runtime: 'js' })).toBe(false)
  expect(
    validate({
      subset: {
        unicodeRanges: ['latin'],
      },
    }),
  ).toBe(false)
  expect(
    validate({
      subset: {
        layoutFeatures: ['too-long'],
        text: 'A',
      },
    }),
  ).toBe(false)
  expect(
    validate({
      subset: {
        nameLanguages: [65_536],
        text: 'A',
      },
    }),
  ).toBe(false)
  expect(
    validate({
      subset: {
        dropTables: ['long-tag'],
        text: 'A',
      },
    }),
  ).toBe(false)
  expect(
    validate({
      plugins: [
        {
          name: 'fontmin:ttf2woff2',
          native: {
            kind: 'builtin',
            name: 'ttf2woff2',
            options: {
              quality: 12,
            },
          },
        },
      ],
    }),
  ).toBe(false)
})
