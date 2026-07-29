import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { prepareProductionFixtures } from './prepare-production-fixtures.mjs'

const workspaceRoot = dirname(import.meta.dirname)
const manifestRelativePath = 'fixtures/production/manifest.json'
const deliverySlices = [
  { name: 'latin', unicodeRanges: ['U+0020-007E'] },
  { name: 'cjk', unicodeRanges: ['U+4E00-4E7F'] },
  { name: 'punctuation', unicodeRanges: ['U+3000-303F'] },
]

function digest(contents) {
  return createHash('sha256').update(contents).digest('hex')
}

export function assertExpectedMetadata(runtime, fixture, metadata) {
  assert.equal(
    metadata.familyName,
    fixture.expected.familyName,
    `${runtime} ${fixture.id} has an unexpected family name`,
  )
  assert.equal(
    metadata.glyphCount,
    fixture.expected.glyphCount,
    `${runtime} ${fixture.id} has an unexpected glyph count`,
  )

  for (const table of fixture.expected.tables) {
    assert.ok(
      metadata.tables.includes(table),
      `${runtime} ${fixture.id} is missing expected table ${table}`,
    )
  }
}

function normalizedDeliveryAssets(assets) {
  return assets
    .map(asset => ({
      byteLength: asset.contents.byteLength,
      fileName: asset.path ?? asset.fileName,
      sha256: digest(asset.contents),
    }))
    .toSorted((left, right) => left.fileName.localeCompare(right.fileName))
}

export function assertDeliveryParity(fixtureId, nativeAssets, wasmAssets) {
  try {
    assert.deepEqual(
      normalizedDeliveryAssets(wasmAssets),
      normalizedDeliveryAssets(nativeAssets),
    )
  } catch (error) {
    throw new Error(
      `${fixtureId} delivery output differs between native and WASM`,
      { cause: error },
    )
  }
}

export async function assertDeliverySemantics(
  runtime,
  fixture,
  assets,
  inspect,
) {
  const expectedTables = fixture.expected.deliveryTables ?? []

  for (const asset of assets) {
    const fileName = asset.path ?? asset.fileName
    const info = await inspect(asset.contents)

    for (const table of expectedTables) {
      assert.ok(
        info.metadata.tables.includes(table),
        `${runtime} ${fixture.id} delivery slice ${fileName} is missing table ${table}`,
      )
    }
    assert.ok(
      info.metadata.glyphCount > 0 &&
        info.metadata.glyphCount < fixture.expected.glyphCount,
      `${runtime} ${fixture.id} delivery slice ${fileName} was not subset`,
    )
  }
}

export async function runProductionConformance({ root = workspaceRoot } = {}) {
  const prepared = await prepareProductionFixtures({ root })
  const manifest = JSON.parse(
    await readFile(join(root, manifestRelativePath), 'utf8'),
  )
  const native = await import(
    pathToFileURL(join(root, 'packages/fontmin/dist/index.mjs')).href
  )
  const wasm = await import(
    pathToFileURL(join(root, 'wasm/fontmin/dist/index.mjs')).href
  )

  if (!wasm.isWasmInitialized()) {
    await wasm.initWasm(
      await readFile(join(root, 'wasm/fontmin/dist/fontmin_wasm_core_bg.wasm')),
    )
  }

  const preparedById = new Map(
    prepared.fixtures.map(fixture => [fixture.id, fixture]),
  )
  const reports = []

  for (const fixture of manifest.fixtures) {
    const preparedFixture = preparedById.get(fixture.id)
    if (preparedFixture === undefined) {
      throw new Error(`production fixture ${fixture.id} was not prepared`)
    }

    const contents = await readFile(preparedFixture.path)
    const [nativeInfo, wasmInfo] = await Promise.all([
      native.inspect(contents),
      wasm.inspect(contents),
    ])

    assertExpectedMetadata('native', fixture, nativeInfo.metadata)
    assertExpectedMetadata('wasm', fixture, wasmInfo.metadata)
    assert.deepEqual(
      wasmInfo.metadata,
      nativeInfo.metadata,
      `${fixture.id} metadata differs between native and WASM`,
    )

    const report = {
      byteLength: contents.length,
      id: fixture.id,
      metadata: nativeInfo.metadata,
      scenarios: fixture.scenarios,
    }

    if (fixture.scenarios.includes('mixed-delivery')) {
      const nativeAssets = await native.optimize({
        cache: false,
        input: [preparedFixture.path],
        plugins: [native.deliverySlices(deliverySlices)],
      })
      const wasmAssets = await wasm.optimizeBrowser({
        assets: [{ contents, fileName: fixture.cachePath }],
        plugins: [wasm.deliverySlices(deliverySlices)],
      })

      await Promise.all([
        assertDeliverySemantics(
          'native',
          fixture,
          nativeAssets,
          native.inspect,
        ),
        assertDeliverySemantics('wasm', fixture, wasmAssets, wasm.inspect),
      ])
      assertDeliveryParity(fixture.id, nativeAssets, wasmAssets)
      report.delivery = normalizedDeliveryAssets(nativeAssets)
    }

    reports.push(report)
  }

  return { fixtures: reports, schemaVersion: 1 }
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  const result = await runProductionConformance()
  const deliveryCount = result.fixtures.filter(
    fixture => fixture.delivery !== undefined,
  ).length

  console.log(
    `Verified ${result.fixtures.length} production fixtures across native and WASM (${deliveryCount} mixed-delivery case).`,
  )
}
