import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { checkFontFixtures } from './check-font-fixtures.mjs'

async function createFixtureWorkspace({ lineEnding = '\n', validDigest }) {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-fixtures-'))
  const fontDirectory = join(root, 'fixtures/fonts/ttf')
  const malformedDirectory = join(root, 'fixtures/malformed')
  const productionDirectory = join(root, 'fixtures/production')
  const fontPath = join(fontDirectory, 'test.ttf')
  const contents = Buffer.concat([
    Buffer.from([0, 1, 0, 0]),
    Buffer.from('glyf'),
  ])
  const sha256 = createHash('sha256').update(contents).digest('hex')
  const checksumDigest = validDigest ? sha256 : '0'.repeat(64)

  await Promise.all([
    mkdir(fontDirectory, { recursive: true }),
    mkdir(malformedDirectory, { recursive: true }),
    mkdir(productionDirectory, { recursive: true }),
  ])
  await writeFile(fontPath, contents)
  await writeFile(
    `${fontPath}.sha256`,
    `${checksumDigest}  fixtures/fonts/ttf/test.ttf${lineEnding}`,
  )
  await writeFile(
    join(root, 'fixtures/fonts/manifest.json'),
    JSON.stringify({
      fonts: [
        {
          container: 'ttf',
          coverage: ['test'],
          outlines: 'glyf',
          path: 'fixtures/fonts/ttf/test.ttf',
          sha256,
          source: {
            license: 'MIT',
            licenseUrl: 'https://example.com/license',
            project: 'test',
            url: 'https://example.com/test.ttf',
          },
          variation: 'static',
        },
      ],
      schemaVersion: 1,
    }),
  )
  await writeFile(
    join(malformedDirectory, 'manifest.json'),
    JSON.stringify({ cases: [], schemaVersion: 1 }),
  )
  await writeFile(
    join(productionDirectory, 'manifest.json'),
    JSON.stringify({ fixtures: [], schemaVersion: 1 }),
  )

  return root
}

test('verifies the repository font fixture inventory', async () => {
  const result = await checkFontFixtures()

  assert.equal(result.count, 5)
  assert.equal(result.malformedCount, 9)
  assert.equal(result.productionCount, 2)
  assert.deepEqual(result.productionIds, [
    'noto-color-emoji',
    'noto-sans-sc-vf',
  ])
  assert.deepEqual(result.paths, [
    'fixtures/fonts/otf/font-awesome-free-solid-900.otf',
    'fixtures/fonts/otf/source-sans-3-regular.otf',
    'fixtures/fonts/otf/source-serif-4-variable-roman.otf',
    'fixtures/fonts/ttf/noto-sans-sc-compact.ttf',
    'fixtures/fonts/ttf/roboto-regular.ttf',
  ])
  assert.deepEqual(result.malformedPaths, [
    'fixtures/malformed/cff-index-offset-outside-data.otf.hex',
    'fixtures/malformed/duplicate-sfnt-table.ttf.hex',
    'fixtures/malformed/not-a-font.bin',
    'fixtures/malformed/overlapping-woff-tables.woff.hex',
    'fixtures/malformed/subset-short-head-table.ttf.hex',
    'fixtures/malformed/subset-table-count-overflow.ttf.hex',
    'fixtures/malformed/truncated-otf.bin',
    'fixtures/malformed/truncated-woff.bin',
    'fixtures/malformed/truncated-woff2.bin',
  ])
})

test('rejects a companion checksum that differs from the manifest', async () => {
  const root = await createFixtureWorkspace({ validDigest: false })

  try {
    await assert.rejects(
      checkFontFixtures({ root }),
      /test\.ttf\.sha256 does not match the manifest/u,
    )
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('accepts companion checksums checked out with CRLF line endings', async () => {
  const root = await createFixtureWorkspace({
    lineEnding: '\r\n',
    validDigest: true,
  })

  try {
    const result = await checkFontFixtures({ root })

    assert.equal(result.count, 1)
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('rejects a malformed fixture checksum that differs from the manifest', async () => {
  const root = await createFixtureWorkspace({ validDigest: true })
  const malformedDirectory = join(root, 'fixtures/malformed')
  const fixturePath = join(malformedDirectory, 'invalid.bin')
  const fixtureRelativePath = 'fixtures/malformed/invalid.bin'
  const contents = Buffer.from('invalid')
  const sha256 = createHash('sha256').update(contents).digest('hex')

  try {
    await writeFile(fixturePath, contents)
    await writeFile(
      `${fixturePath}.sha256`,
      `${'0'.repeat(64)}  ${fixtureRelativePath}\n`,
    )
    await writeFile(
      join(malformedDirectory, 'manifest.json'),
      JSON.stringify({
        cases: [
          {
            expectedDiagnostic: {
              code: 'fontmin::invalid_font',
              message: 'invalid font data',
            },
            operation: 'inspect',
            path: fixtureRelativePath,
            sha256,
            source: {
              generator: 'Literal invalid bytes.',
              kind: 'synthetic',
              license: 'MIT',
              licenseUrl: 'https://example.com/license',
              project: 'test',
              url: 'https://example.com/invalid.bin',
            },
          },
        ],
        schemaVersion: 1,
      }),
    )

    await assert.rejects(
      checkFontFixtures({ root }),
      /invalid\.bin\.sha256 does not match the manifest/u,
    )
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})
