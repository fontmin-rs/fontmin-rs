import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'
import {
  assertDeliveryParity,
  assertDeliverySemantics,
  assertExpectedMetadata,
} from './production-conformance.mjs'

const expected = {
  familyName: 'Test Font',
  glyphCount: 2,
  tables: ['fvar', 'glyf'],
}

async function inspectDeliveryFixture(contents) {
  return {
    metadata: {
      glyphCount: contents.toString() === 'latin' ? 20 : 30,
      tables: ['fvar', 'glyf', 'gvar'],
    },
  }
}

test('accepts production metadata with every required table', () => {
  assert.doesNotThrow(() =>
    assertExpectedMetadata(
      'native',
      { expected, id: 'test-font' },
      {
        familyName: 'Test Font',
        glyphCount: 2,
        tables: ['fvar', 'glyf', 'name'],
      },
    ),
  )
})

test('reports the runtime and fixture for a metadata mismatch', () => {
  assert.throws(
    () =>
      assertExpectedMetadata(
        'wasm',
        { expected, id: 'test-font' },
        {
          familyName: 'Test Font',
          glyphCount: 2,
          tables: ['glyf'],
        },
      ),
    /wasm test-font is missing expected table fvar/u,
  )
})

test('requires native and WASM delivery assets to be byte-identical', () => {
  const nativeAssets = [
    {
      contents: Buffer.from('latin'),
      path: 'font-latin.ttf',
    },
  ]
  const wasmAssets = [
    {
      contents: new TextEncoder().encode('latin'),
      fileName: 'font-latin.ttf',
    },
  ]

  assert.doesNotThrow(() =>
    assertDeliveryParity('test-font', nativeAssets, wasmAssets),
  )
  assert.throws(
    () =>
      assertDeliveryParity('test-font', nativeAssets, [
        {
          contents: new TextEncoder().encode('different'),
          fileName: 'font-latin.ttf',
        },
      ]),
    /test-font delivery output differs between native and WASM/u,
  )
})

test('requires every delivery slice to retain declared semantic tables', async () => {
  const fixture = {
    expected: {
      deliveryTables: ['fvar', 'gvar'],
      glyphCount: 100,
    },
    id: 'test-font',
  }
  const assets = [
    { contents: Buffer.from('latin'), path: 'font-latin.ttf' },
    { contents: Buffer.from('cjk'), path: 'font-cjk.ttf' },
  ]
  await assert.doesNotReject(
    assertDeliverySemantics('native', fixture, assets, inspectDeliveryFixture),
  )
  await assert.rejects(
    assertDeliverySemantics('wasm', fixture, assets, async () => ({
      metadata: { glyphCount: 20, tables: ['glyf'] },
    })),
    /wasm test-font delivery slice font-latin\.ttf is missing table fvar/u,
  )
  await assert.rejects(
    assertDeliverySemantics('native', fixture, assets, async () => ({
      metadata: { glyphCount: 100, tables: ['fvar', 'gvar'] },
    })),
    /native test-font delivery slice font-latin\.ttf was not subset/u,
  )
})

test('runs the production corpus from the cached CI benchmark job', async () => {
  const [packageManifest, workflow] = await Promise.all([
    readFile(new URL('../package.json', import.meta.url), 'utf8').then(
      JSON.parse,
    ),
    readFile(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8'),
  ])

  assert.equal(
    packageManifest.scripts['build:release'],
    'pnpm --filter @fontmin-rs/binding build:release && oxfmt napi/fontmin/src-js/bindings.js napi/fontmin/src-js/index.d.ts',
  )
  assert.equal(
    packageManifest.scripts['fixtures:production:conformance'],
    'node scripts/prepare-production-fixtures.mjs && pnpm run build:release && pnpm run build && pnpm -C wasm/fontmin run build:js && node scripts/production-conformance.mjs',
  )
  assert.match(workflow, /uses: actions\/cache@[0-9a-f]{40} # v6\.1\.0/u)
  assert.match(
    workflow,
    /key: production-fonts-\$\{\{ hashFiles\('fixtures\/production\/manifest\.json'\) \}\}/u,
  )
  assert.match(workflow, /run: pnpm run bench:production/u)
})
