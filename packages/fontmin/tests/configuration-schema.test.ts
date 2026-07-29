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
        keepLayout: 'conservative',
        missingGlyphs: 'error',
        text: 'Hello',
      },
    }),
  ).toBe(true)
})

it('validates descriptors produced by public built-in plugin helpers', () => {
  const config: FontminConfig = {
    input: ['fonts/*.ttf'],
    plugins: [
      glyph({ text: 'Hello', unicodeRanges: ['U+0000-00FF'] }),
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
