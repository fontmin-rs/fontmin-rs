import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import {
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  writeFile,
} from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { prepareProductionFixtures } from './prepare-production-fixtures.mjs'

const fixtureContents = Buffer.from('production-font-fixture')
const fixtureSha256 = createHash('sha256').update(fixtureContents).digest('hex')

async function createWorkspace({ sha256 = fixtureSha256 } = {}) {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-production-fixtures-'))
  const fixtureDirectory = join(root, 'fixtures/production')

  await mkdir(fixtureDirectory, { recursive: true })
  await writeFile(
    join(fixtureDirectory, 'manifest.json'),
    JSON.stringify({
      fixtures: [
        {
          byteLength: fixtureContents.length,
          cachePath: 'test-font.ttf',
          downloadUrl: 'https://example.com/test-font.ttf',
          expected: {
            familyName: 'Test Font',
            glyphCount: 1,
            tables: ['glyf'],
          },
          id: 'test-font',
          scenarios: ['inspect'],
          sha256,
          source: {
            license: 'MIT',
            licenseUrl: 'https://example.com/license',
            project: 'Test Font',
            url: 'https://example.com/test-font.ttf',
          },
        },
      ],
      schemaVersion: 1,
    }),
  )

  return root
}

function fixtureResponse({ contentLength = fixtureContents.length } = {}) {
  return new Response(fixtureContents, {
    headers: { 'content-length': String(contentLength) },
    status: 200,
  })
}

test('downloads, verifies, and reuses production fixtures', async () => {
  const root = await createWorkspace()
  let requestCount = 0
  const fetchImpl = async () => {
    requestCount += 1
    return fixtureResponse({ contentLength: fixtureContents.length - 1 })
  }

  try {
    const downloaded = await prepareProductionFixtures({ fetchImpl, root })
    const reused = await prepareProductionFixtures({ fetchImpl, root })

    assert.equal(requestCount, 1)
    assert.deepEqual(
      downloaded.fixtures.map(fixture => fixture.status),
      ['downloaded'],
    )
    assert.deepEqual(
      reused.fixtures.map(fixture => fixture.status),
      ['reused'],
    )
    assert.deepEqual(
      await readFile(join(root, 'fixtures/production/.cache/test-font.ttf')),
      fixtureContents,
    )
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('replaces a corrupt cache entry without leaving temporary files', async () => {
  const root = await createWorkspace()
  const cacheDirectory = join(root, 'fixtures/production/.cache')
  let requestCount = 0

  try {
    await mkdir(cacheDirectory, { recursive: true })
    await writeFile(join(cacheDirectory, 'test-font.ttf'), 'corrupt')
    await writeFile(
      join(cacheDirectory, '.test-font.ttf.interrupted.tmp'),
      'partial',
    )

    const result = await prepareProductionFixtures({
      fetchImpl: async () => {
        requestCount += 1
        return fixtureResponse()
      },
      root,
    })

    assert.equal(requestCount, 1)
    assert.equal(result.fixtures[0]?.status, 'downloaded')
    assert.deepEqual(await readdir(cacheDirectory), ['test-font.ttf'])
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('rejects a digest mismatch and removes the partial download', async () => {
  const root = await createWorkspace({ sha256: '0'.repeat(64) })
  const cacheDirectory = join(root, 'fixtures/production/.cache')

  try {
    await assert.rejects(
      prepareProductionFixtures({
        fetchImpl: async () => fixtureResponse(),
        root,
      }),
      /test-font digest is .*; expected 000000/u,
    )
    assert.deepEqual(await readdir(cacheDirectory), [])
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})
